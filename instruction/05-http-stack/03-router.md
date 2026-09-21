# Router Design

## What to learn

### Matching strategies
A linear `Vec<Route>` scan is O(n) per request but trivial to implement and fine for a handful of routes. A radix/trie-based router (what most production Rust routers — `matchit`, axum's router — use) shares common path prefixes in a tree so lookup is roughly O(path length), and handles path parameters (`/users/:id`) and wildcards without backtracking regex costs.

The structures themselves are in `13-algorithms/trie.md` (segment-based
prefix matching) and `13-algorithms/radix-tree.md` (the compressed form
production routers actually ship, including the tricky
longest-common-prefix split on insert). Read those for the implementation;
this file is about what a *proxy* needs on top of them.

### Normalize the path before matching, or you have a security bug
This is the single most important thing in this file, and it is not
obvious: the path you match against and the path the upstream ultimately
resolves must be the same path, or an attacker lives in the difference.

Consider a proxy configured to block `/admin` and allow `/public/*`. A
request for `/public/../admin` matches `/public/*` by naive segment
matching — and an upstream that normalizes `..` before serving hands back
`/admin`. The proxy's routing decision and the upstream's interpretation
disagreed, and the access control was the casualty. This is the same
parser-differential family as `07-security/05-request-smuggling.md`, applied
to paths instead of framing.

The variants to handle, all of which have produced real bypasses:
- **Dot segments**: `/a/../b`, `/a/./b` — resolve them before matching.
- **Encoded separators**: `%2f` is `/` after decoding. Decide explicitly
  whether an encoded slash is a path separator (most upstreams say yes,
  and most naive routers say no) and reject rather than guess if you
  can't be sure.
- **Double-encoding**: `%252e%252e%252f` — decode to a fixed, documented
  depth, matching what your upstream does (`07-security/06-waf.md` has the
  same discussion).
- **Duplicate slashes**: `//admin` vs `/admin` — collapse them.
- **Trailing slash**: `/admin/` vs `/admin`. Pick one canonical form and
  redirect the other rather than registering both.
- **Case**: paths are case-sensitive in HTTP and on Linux filesystems, but
  not on macOS/Windows ones — an upstream on a case-insensitive filesystem
  serves `/ADMIN` for a rule written as `/admin`.

The rule: normalize once, early, into a canonical form; route on that
form; and forward *that* form upstream rather than the original bytes, so
there is no second interpretation to disagree with.

### Path parameters & precedence
When both `/users/:id` and `/users/me` could match `/users/me`, the router needs a deterministic precedence rule — static segments beat parameter segments beat wildcards. Getting this wrong means route matches become order-dependent on registration order, which is a subtle source of bugs as a route table grows.

Gotcha: detect genuine conflicts at *registration* time, not match time.
Two routes that can never be distinguished (the same pattern registered
twice, or `/a/:x` and `/a/:y`) are a configuration bug, and failing at
startup — or at config reload, with the old table left running
(`09-architecture/03-config.md`) — beats silently picking one and leaving an
endpoint permanently unreachable.

### Method dispatch
Routing is two-dimensional: path *and* method. A common bug is matching the path and returning 404 instead of 405 (Method Not Allowed) when the path exists but the method doesn't — proxies and APIs are expected to distinguish these.

Two cases the two-dimensional model tends to miss:
- **`HEAD` must work wherever `GET` does.** RFC 9110 defines `HEAD` as
  `GET` without a body. A router that requires explicit `HEAD`
  registration returns 405 for a perfectly valid request, and monitoring
  tools use `HEAD` constantly.
- **`OPTIONS`** is a CORS preflight, and browsers send it *without*
  credentials before the real request. If the proxy applies auth
  (`07-security/01-auth.md`) to preflights, every cross-origin call from a
  browser fails in a way that looks like a CORS misconfiguration and is
  actually an auth-ordering bug.

Gotcha: the 405 response must include an `Allow` header listing the
methods that do match. It is required by spec and it is what makes the
difference debuggable.

### Routing on more than the path
A proxy routes on the whole request, not just the path: `Host` (see
`05-http-stack/11-vhost-routing.md`), arbitrary headers (canary routing by
`X-Version`, `09-architecture/06-canary-deploy.md`), sometimes weights for
traffic splitting.

Gotcha: `Host` is not one thing. In HTTP/1.1 it's the `Host` header; in
HTTP/2 and HTTP/3 it's the `:authority` pseudo-header; and under TLS
there's also the SNI name from the handshake (`01-network/07-tls.md`), which
the client chose *before* sending any of them. These can all disagree —
an attacker connects with SNI `public.example.com` and sends `Host:
admin.internal`. Decide which one is authoritative for routing, validate
that the others match it, and reject the request when they don't.

### Middleware as composition, not a special case
Think of a route handler as a `tower::Service<Request> -> Response`. Middleware (auth, rate limiting, logging) is just another `Service` that wraps the inner one — this is why `07-security/01-auth.md` and `07-security/07-ratelimit.md` plug into the same router abstraction instead of being bolted onto it separately.

```rust
// sketch: router as a Vec of (matcher, handler), method-aware
struct Route {
    method: Method,
    matcher: PathMatcher, // static segments + :param + wildcard
    handler: Handler,
}
```

The ordering question that follows: middleware that must run *before*
routing (connection limits, IP filtering — you can't afford to route
first) versus middleware that needs the route to exist first (per-route
auth policy, per-route rate limits). That split is the real structure of
the pipeline, and `09-architecture/01-components.md` is where it's designed.

### Routing in a reverse proxy vs. an API server
An API server routes to a handler function; a reverse proxy (`06-proxy/`) routes to an *upstream pool* — the "handler" is "forward this request to service X's load balancer." The matching logic (path prefix, host header, headers-based routing) is the same problem, just with a different terminal action.

One consequence specific to the proxy case: the route table changes at
runtime (`09-architecture/03-config.md`, `labs/13-hot-reload`) while requests
are in flight. Build the table as an immutable structure swapped atomically
(`arc_swap`, as in `06-proxy/07-service-discovery.md`) rather than one mutated
under a lock — routing is on the hot path of every request, and it should
never wait on a config update.

Gotcha: decide what the router does about the matched prefix when
forwarding. Stripping it (`/api/v1/users` → `/users` upstream) is common
and is a second place where the path the proxy matched and the path the
upstream sees diverge — apply the same discipline as the normalization
section, and make the rewrite explicit per route rather than implicit.

## Practice
Build these in order.

1. In `labs/03-router`, implement a linear route matcher (method + exact
   path) wired into the hyper handler. **Done when** two distinct paths
   route to distinct handlers.
2. Write the path-confusion tests before adding normalization: request
   `/public/../admin`, `//admin`, `/admin/`, `/%2e%2e/admin`, and
   `/pub%2f../admin` against a router that allows `/public/*` and blocks
   `/admin`. **Done when** you've confirmed at least one of them reaches
   the blocked route — you need the failing baseline.
3. Add path normalization (dot-segment resolution, slash collapsing,
   documented decode depth, canonical trailing slash) applied before
   matching *and* used for the forwarded path. **Done when** every test
   from step 2 is blocked, and the upstream receives the normalized path.
4. Add path parameters and wildcards with explicit precedence, and
   conflict detection at registration. **Done when** `/users/me` beats
   `/users/:id` regardless of registration order, and registering a
   genuinely ambiguous pair fails at startup.
5. Add method dispatch with correct 404 vs 405 and an `Allow` header, plus
   `HEAD` falling back to `GET` routes and `OPTIONS` bypassing auth.
   **Done when** `HEAD` on a `GET`-only route returns 200 with no body,
   and a CORS preflight succeeds without credentials.
6. Swap the linear matcher for the radix tree from
   `13-algorithms/radix-tree.md`. **Done when** the full test suite passes
   unchanged and lookup latency is flat from 10 to 1000 routes.
7. Refactor handlers behind a `Service`-like trait and add a logging
   middleware. **Done when** the same middleware wraps both a local
   handler and (later) a proxy route without modification.
8. Add `Host`/`:authority`/SNI consistency validation. **Done when** a
   request whose `Host` disagrees with its SNI is rejected, and you can
   state which source your router treats as authoritative.
9. When you reach `labs/05-reverse-proxy`, make a matched route resolve to
   an upstream pool name, with explicit per-route prefix rewriting, and
   swap the table atomically on reload. **Done when** a route table
   reload under concurrent load produces zero dropped or misrouted
   requests.
