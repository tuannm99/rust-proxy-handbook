# Components
Listener -> ConnMgr -> Codec -> Router -> Modules

## What to learn
### What each stage owns
- **Listener**: binds the socket(s), accepts connections, may do TLS termination (handing off a decrypted stream). Owns nothing about HTTP semantics.
- **ConnMgr (connection manager)**: tracks live connections, enforces per-connection limits/timeouts, drives graceful shutdown (stop handing new connections in, let existing ones drain — see `09-architecture/graceful-shutdown.md`).
- **Codec**: turns bytes into typed `Request`/`Response` values and back (HTTP/1.1 parsing, HTTP/2 framing) — this is where hyper sits if you use it, or your own parser if you did `labs/01-http-parser`.
- **Router**: matches a request to a destination — a specific upstream pool, or a local handler (health endpoint, metrics endpoint). Pure decision logic, no I/O.
- **Modules**: everything that wraps the request/response on the way through — auth, rate limiting, WAF, logging, metrics. Order matters (e.g. rate limit before auth to reject cheaply; WAF before both to reject malicious bodies early).

### Module order is a security decision, not a preference
Scattered through `07-security/` and `05-http-stack/` are ordering
constraints that each look local and together define the pipeline. Collected:

| Position | Stage | Why here |
| --- | --- | --- |
| 1 | Connection/accept limits | Cheapest possible rejection, before any parsing (`07-security/ddos.md`) |
| 2 | IP filtering on the real peer | Before anything expensive; uses the socket address, not headers (`07-security/ip-filtering.md`) |
| 3 | TLS termination | Client-cert rejection should happen at handshake, not after (`07-security/auth.md`) |
| 4 | Codec / parse | Framing validation and smuggling rejection (`07-security/request-smuggling.md`) |
| 5 | **Strip hop-by-hop and identity headers** | Must happen before anything reads them (`05-http-stack/keepalive.md`, `07-security/auth.md`) |
| 6 | Path normalization | Before routing, or routing decides on a different path than the upstream sees (`05-http-stack/router.md`) |
| 7 | Routing | Needed to know *which* policy applies to the rest |
| 8 | Per-route rate limiting | Cheap rejection before expensive work (`07-security/ratelimit.md`) |
| 9 | Auth | Before body inspection and before any upstream cost |
| 10 | WAF / body inspection | Most expensive check, runs last and only for authenticated, non-rate-limited traffic (`07-security/waf.md`) |
| 11 | Cache lookup | Before the upstream call, after auth (or you serve one user's response to another — `05-http-stack/cache.md`) |
| 12 | Upstream call | Load balancing, retries, circuit breaking (`06-proxy/`) |

Logging and metrics wrap the whole thing, since they must observe requests
that were rejected at every stage above.

Gotcha: stages 5 and 7 are the ones people get wrong by accident. Stripping
identity headers *after* a module has read them, or routing before
normalizing, produces a pipeline that is correct in tests and exploitable
in production.

### Composing modules as a pipeline, not a monolith
Each module should be independently testable: given a request (and some state), does it forward, short-circuit (403/429), or mutate (add a header) — without knowing about the others. In Rust this maps naturally onto `tower::Service`/`Layer`: a `Router` is a `Service`, each module is a `Layer` wrapping it, and the whole stack composes via `ServiceBuilder`.

```rust
use tower::{ServiceBuilder, service_fn};

let svc = ServiceBuilder::new()
    .layer(RateLimitLayer::new(/* ... */))
    .layer(AuthLayer::new(/* ... */))
    .layer(MetricsLayer::new(/* ... */))
    .service(router_service);
```
Gotcha: `Layer` order in `ServiceBuilder` wraps outside-in but *executes* outer-first on the request path — get this backwards and rate limiting runs after the expensive auth check it was supposed to protect.

Gotcha: `Service::poll_ready` is tower's backpressure mechanism and it is
routinely ignored. A service that returns `Poll::Pending` from
`poll_ready` is saying "I am at capacity, don't send me a request yet" —
which is how a concurrency limit propagates *back* through the stack
instead of queueing internally (`07-security/ddos.md`'s shed-vs-queue
argument). A module that always returns `Ready` and buffers internally has
silently converted backpressure into unbounded memory.

Gotcha: `poll_ready` reserving capacity means the *next* `call` is the one
entitled to it. Calling `poll_ready` once and then `call` many times is a
contract violation that tower's own combinators assume you won't commit.

### Where per-request state lives
Modules need to pass information forward — the validated identity from
auth, the matched route, the chosen upstream, timing marks. The two
options are threading a custom context type through every module (explicit,
type-safe, and a signature change every time anything is added) or using
`http::Extensions`, a type-keyed map attached to the request.

Extensions is the idiomatic choice in a tower stack, with one discipline:
insert *newtypes*, not bare primitives. `req.extensions().get::<String>()`
is ambiguous the moment two modules insert a `String`; `get::<UserId>()`
cannot collide.

Gotcha: an extension that a later module *requires* is an invisible
dependency — the type system won't tell you that `UpstreamSelector` panics
when `AuthLayer` isn't in the stack. Make retrieval fail explicitly (a
5xx with a clear internal error) rather than `unwrap()`, and document the
requirement where the module is defined.

### A module must never take down the connection
Two failure modes to design for up front:
- **Errors.** In tower, a `Service` error propagates up and typically
  kills the connection. For a proxy, almost every module failure should
  instead become a *response* (401, 429, 502) — so modules should be
  infallible at the type level (`Error = Infallible`) and convert internal
  failures into responses themselves. That way a bug in one module cannot
  drop an unrelated pipelined request on the same connection.
- **Panics.** A panic in a request task, by default, unwinds that task
  only — tokio catches it and the runtime survives — but the connection is
  dropped mid-response and the client sees a reset. Catch panics at the
  pipeline boundary and convert them into a 500 with a logged event, so
  one bad request doesn't take a whole keep-alive connection's worth of
  other requests with it.

Gotcha: `panic = "abort"` in your release profile turns every panic into a
process-wide crash, which changes this calculation entirely. Know which
you've configured.

### Why this shape, not "one big async fn"
A single giant handler function works for a toy but becomes untestable and unreadable once you have 5+ cross-cutting concerns. Splitting into Listener/ConnMgr/Codec/Router/Modules means each piece maps to one handbook section (`01-network`, `04-runtime`, `05-http-stack`, `06-proxy`, `07-security`) and can be built/tested in isolation before wiring together in `proxy`.

### Data flow through the pipeline
Inbound: `TcpStream` → (TLS decrypt) → Codec decodes → `Request` flows through Module stack → Router picks upstream → Codec encodes outbound `Request` → forwarded. Response flows back through the same Module stack in reverse (so a logging module sees both the original request and the final response/status).

Gotcha: "in reverse" means a module's response-side work runs in the
opposite order from its request-side work, which is usually what you want
(logging outermost sees the final status) and occasionally not
(compression must run *inside* caching, so the cache stores one
representation rather than a compressed one it can't re-serve to a
different client — `05-http-stack/compression.md`,
`05-http-stack/cache.md`). Write the response path's order down
explicitly; don't assume it falls out correctly.

Gotcha: a streaming response means the response "passes through" modules
before the body has been produced. A module that wants to inspect or
transform the body is choosing to buffer it
(`07-security/waf.md`'s limit discussion) — and a module that merely wants
the status code must not accidentally force buffering by awaiting the
whole body.

## Practice
Build these in order.

1. Sketch the pipeline for `proxy` — modules, order, and why — against the
   table above. **Done when** you can justify each position from a
   specific failure it prevents, not from convention.
2. Implement the Router and one module (rate limiting) as separate
   `tower::Service`/`Layer` impls with `Error = Infallible`. **Done when**
   each is unit-tested with no network involved, and a rate-limit
   rejection is a 429 response rather than a service error.
3. Wire Listener → ConnMgr → Codec → Router → Modules end-to-end for one
   upstream. **Done when** a real request flows through and back.
4. Add auth and verify ordering. **Done when** a test proves a
   rate-limited request never reaches auth, and a request with a forged
   `X-User-Id` has it stripped before any module can read it.
5. Pass state via typed extensions. **Done when** auth inserts a `UserId`
   newtype that the logging module reads, and removing `AuthLayer` from
   the stack produces a clear 500 with a logged explanation rather than a
   panic.
6. Implement `poll_ready`-based backpressure in a concurrency-limiting
   module. **Done when** overload causes the stack to shed
   (`07-security/ddos.md`) rather than buffer — measure memory under
   sustained overload to prove nothing is queueing invisibly.
7. Add panic containment at the pipeline boundary. **Done when** a module
   that panics on one request returns 500 for that request and the
   connection's *other* pipelined requests still complete.
8. Write down the response-path order. **Done when** compression runs
   inside caching (the cache stores an uncompressed representation), and a
   test proves a cached entry can be served to clients with different
   `Accept-Encoding`.
9. Add graceful shutdown at the ConnMgr layer
   (`09-architecture/graceful-shutdown.md`). **Done when** in-flight
   requests complete before exit.
