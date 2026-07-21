# Reading Source

Structured study of production Rust networking projects and the
C-based proxies that established the concepts this handbook teaches. Each
subfolder is one project; read its source with a specific lens rather than
browsing aimlessly.

## Template (per project)

Each project subfolder is planned to eventually hold:

- `architecture.md` — the major components and how they fit together
- `request-flow.md` — trace one request end-to-end through the source
- `memory.md` — how the project manages memory/buffers on the hot path
- `interesting-code.md` — specific functions/files worth reading closely, with why
- `what-to-learn.md` — the handbook topics (by path) this project is the best real-world example of

## Projects

- `nginx/` — the reference L7 proxy architecture (master/worker, event loop)
- `envoy/` — modern C++ proxy, xDS dynamic config, observability-first design
- `haproxy/` — battle-tested L4/L7 load balancer, minimal-allocation event loop
- `pingora/` — Cloudflare's Rust proxy framework, the closest real-world analog to `proxy/`
- `hyper/` — the HTTP library `labs/02-http-server` onward is built on
- `tokio/` — the async runtime underneath everything in this workspace
- `mio/` — the epoll/kqueue abstraction underneath tokio
- `quinn/` — QUIC/HTTP-3 implementation, relevant once `01-network/http3.md` is in scope
