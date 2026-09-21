# Reading Source

Structured study of production Rust networking projects and the
C-based proxies that established the concepts this handbook teaches. Each
subfolder is one project; read its source with a specific lens rather than
browsing aimlessly.

## Status: index only — deliberately deferred until after `proxy/`

The eight project subfolders hold stubs, not content, and nothing links to
them. This one is not a backlog item to clear early — the sequencing is the
point.

Reading `pingora`'s upstream pool before you have written one teaches you
almost nothing: you have no design of your own to compare it against, so
every decision reads as arbitrary. Read it *after* `proxy/` works, and the
same code becomes a running commentary on choices you already had to make —
including the ones you got wrong. That contrast is the entire value of this
folder.

Two exceptions worth reading early, and both are already pointed at from
where they matter: tokio's reactor after the raw-epoll exercise
(`02-linux/01-epoll.md`, Practice step 6) and `hyper`'s `h1` codec after your
own parser (`05-http-stack/01-parser.md`, Practice step 8). Those work early
precisely because you have just built the thing being compared.

When to write these files: as notes to yourself while reading, after phase
10 in `00-introduction/01-learning-roadmap.md`.

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
- `quinn/` — QUIC/HTTP-3 implementation, relevant once `01-network/06-http3.md` is in scope
