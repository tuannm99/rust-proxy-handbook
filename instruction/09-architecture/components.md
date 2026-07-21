# Components
Listener -> ConnMgr -> Codec -> Router -> Modules

## What to learn
### What each stage owns
- **Listener**: binds the socket(s), accepts connections, may do TLS termination (handing off a decrypted stream). Owns nothing about HTTP semantics.
- **ConnMgr (connection manager)**: tracks live connections, enforces per-connection limits/timeouts, drives graceful shutdown (stop handing new connections in, let existing ones drain — see `09-architecture/graceful-shutdown.md`).
- **Codec**: turns bytes into typed `Request`/`Response` values and back (HTTP/1.1 parsing, HTTP/2 framing) — this is where hyper sits if you use it, or your own parser if you did `labs/01-http-parser`.
- **Router**: matches a request to a destination — a specific upstream pool, or a local handler (health endpoint, metrics endpoint). Pure decision logic, no I/O.
- **Modules**: everything that wraps the request/response on the way through — auth, rate limiting, WAF, logging, metrics. Order matters (e.g. rate limit before auth to reject cheaply; WAF before both to reject malicious bodies early).

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

### Why this shape, not "one big async fn"
A single giant handler function works for a toy but becomes untestable and unreadable once you have 5+ cross-cutting concerns. Splitting into Listener/ConnMgr/Codec/Router/Modules means each piece maps to one handbook section (`01-network`, `04-runtime`, `05-http-stack`, `06-proxy`, `07-security`) and can be built/tested in isolation before wiring together in `proxy`.

### Data flow through the pipeline
Inbound: `TcpStream` → (TLS decrypt) → Codec decodes → `Request` flows through Module stack → Router picks upstream → Codec encodes outbound `Request` → forwarded. Response flows back through the same Module stack in reverse (so a logging module sees both the original request and the final response/status).

## Practice
1. Sketch the pipeline for `proxy` as a diagram before writing code: which modules exist, in what order, and why.
2. Implement Router and one Module (e.g. rate limiting) as separate `tower::Service`/`Layer` implementations; unit test each without a real network connection.
3. Wire Listener → ConnMgr → Codec → Router → Modules end-to-end for a single upstream.
4. Add a second Module (auth) and verify via a test that a request rejected by rate limiting never reaches the auth check (order matters).
5. Add graceful shutdown at the ConnMgr layer per `09-architecture/graceful-shutdown.md` and confirm in-flight requests complete before the process exits.
