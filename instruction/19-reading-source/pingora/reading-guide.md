# Reading guide: pingora

A route through Cloudflare's pingora, with questions to answer as you go.
Read this after [`proxy/`](../../../proxy) works, as [`19-reading-source/00-README.md`](../00-README.md) explains:
every question below is really "how does this compare to what you
built?", and it has no answer until you've built something. This is a
guide, not the notes — the per-project files in [`00-README.md`](00-README.md) are yours to
write.

Repository: `github.com/cloudflare/pingora`, a workspace of several
crates. Paths match the versions current at the time of writing; if one
has moved, search for the named type or trait.

## The crates you'll visit

- `pingora-core` — the server, services, listeners, protocol handling (`src/protocols/`), and outbound connectors (`src/connectors/`).
- `pingora-proxy` — the HTTP proxy logic and the `ProxyHttp` trait you implement to build a proxy.
- `pingora-load-balancing` — selection algorithms, health checks, service discovery.
- `pingora-pool` — the upstream connection pool.
- `pingora-cache`, `pingora-limits`, `pingora-timeout`, `pingora-error` — caching, rate estimation, timers, error types.

## The route

### Stop 1: the extension point: `ProxyHttp`
Start in `pingora-proxy` with the `ProxyHttp` trait (its own file,
`proxy_trait.rs`) and read every method's doc comment in order. They are
called at fixed points in a request's life — picking the upstream,
filtering the request, filtering the response, logging, handling
connection failures.
- Draw the order these callbacks fire in for one successful request. Where in that order would your own proxy's middleware ([`labs/14-plugin`](../../../labs/14-plugin)) have run?
- Which callbacks can end the request early, and how?
- Compare with [`09-architecture/02-plugin.md`](../../09-architecture/02-plugin.md) and [`03-rust/18-async-traits.md`](../../03-rust/18-async-traits.md): how does pingora make async callbacks work on a trait, and what does it cost?

### Stop 2: one request end to end
Follow a downstream HTTP/1 request from `pingora-core`'s server-side
session (`src/protocols/http/v1/server.rs`) into `pingora-proxy` (the
HTTP/1 path, `proxy_h1.rs`), out through a connector to the upstream
(`src/protocols/http/v1/client.rs`), and back.
- How are request and response bodies moved between the two connections — whole, or streamed chunk by chunk? Where would backpressure from a slow client stop reads from the upstream?
- Where is the upstream connection returned to the pool, and under what conditions is it discarded instead?

### Stop 3: the connection pool: `pingora-pool`
- How is a pooled connection keyed — by address only, or by more? Why does TLS/SNI matter to the key?
- How does the pool detect that an idle pooled connection was closed by the upstream before reusing it?
- Compare with your design from [`06-proxy/01-upstream.md`](../../06-proxy/01-upstream.md). Which of the pool's choices would have saved you a bug?

### Stop 4: load balancing and health checks: `pingora-load-balancing`
- Which selection algorithms are provided, and which of the ones from [`labs/06-load-balancer`](../../../labs/06-load-balancer) are missing?
- How does the consistent-hashing implementation compare to [`13-algorithms/consistent-hash.md`](../../13-algorithms/consistent-hash.md)?
- How do health-check results feed back into selection, and how does that compare to [`06-proxy/03-healthcheck.md`](../../06-proxy/03-healthcheck.md)?

### Stop 5: running as a server
In `pingora-core`, find the server bootstrap and the graceful upgrade path
(search for listening-socket transfer between an old and a new process).
- How does a new pingora process take over listening sockets from the old one without dropping connections?
- Compare with your approach to [`09-architecture/04-graceful-shutdown.md`](../../09-architecture/04-graceful-shutdown.md) and [`09-architecture/05-rolling-restart.md`](../../09-architecture/05-rolling-restart.md). What does pingora's approach give you that a drain-and-restart doesn't?

### Stop 6: rate estimation: `pingora-limits`
- What data structure does its rate estimator use, and how does it relate to [`13-algorithms/count-min-sketch.md`](../../13-algorithms/count-min-sketch.md)?
- What does it trade away compared to your [`labs/11-rate-limit`](../../../labs/11-rate-limit) implementation?

## What to write in your notes
`architecture.md` should have your own diagram of the crates and how a
request passes through them. `interesting-code.md` should record, per
stop, the one design choice most different from yours and whether you'd
now adopt it. This file is the payoff of the whole handbook — the more
honestly it records where your proxy fell short, the more it's worth.
