# Router Design

## What to learn

### Matching strategies
A linear `Vec<Route>` scan is O(n) per request but trivial to implement and fine for a handful of routes. A radix/trie-based router (what most production Rust routers — `matchit`, axum's router — use) shares common path prefixes in a tree so lookup is roughly O(path length), and handles path parameters (`/users/:id`) and wildcards without backtracking regex costs.

### Path parameters & precedence
When both `/users/:id` and `/users/me` could match `/users/me`, the router needs a deterministic precedence rule — static segments beat parameter segments beat wildcards. Getting this wrong means route matches become order-dependent on registration order, which is a subtle source of bugs as a route table grows.

### Method dispatch
Routing is two-dimensional: path *and* method. A common bug is matching the path and returning 404 instead of 405 (Method Not Allowed) when the path exists but the method doesn't — proxies and APIs are expected to distinguish these.

### Middleware as composition, not a special case
Think of a route handler as a `tower::Service<Request> -> Response`. Middleware (auth, rate limiting, logging) is just another `Service` that wraps the inner one — this is why `07-security/auth.md` and `07-security/ratelimit.md` plug into the same router abstraction instead of being bolted onto it separately.

```rust
// sketch: router as a Vec of (matcher, handler), method-aware
struct Route {
    method: Method,
    matcher: PathMatcher, // static segments + :param + wildcard
    handler: Handler,
}
```

### Routing in a reverse proxy vs. an API server
An API server routes to a handler function; a reverse proxy (`06-proxy/`) routes to an *upstream pool* — the "handler" is "forward this request to service X's load balancer." The matching logic (path prefix, host header, headers-based routing) is the same problem, just with a different terminal action.

## Practice
1. In `labs/03-router`, implement a linear route matcher first (method + exact path), wire it into the hyper request handler.
2. Add path parameters (`/users/:id`) and wildcard segments; write tests for precedence when multiple patterns could match the same path.
3. Add correct 404 vs 405 handling, including an `Allow` header listing the methods that *do* match the path.
4. Refactor handlers to implement a common `Service`-like trait so middleware (start with just a logging middleware) can wrap any handler.
5. When you reach `labs/05-reverse-proxy`, extend the router so a matched route resolves to an upstream pool name instead of a local handler.
