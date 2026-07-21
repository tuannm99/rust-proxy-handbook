# Rust Nginx Handbook

A structured handbook for building a production-grade L7 (HTTP) proxy in Rust,
modeled conceptually on nginx, Envoy, and HAProxy. The handbook is organized
as a dependency chain: each numbered directory assumes the ones before it.

```
00 introduction  -> why this exists, how to use it
01 network       -> the wire protocols a proxy speaks (DNS, HTTP/1/2/3, TCP, TLS)
02 linux         -> the kernel primitives a proxy is built on (epoll, io_uring, zero-copy)
03 rust          -> the language mechanics async Rust needs (ownership, Pin, unsafe, sync)
04 runtime       -> how tokio turns 02+03 into an async runtime
05 http-stack    -> parsing/routing/caching on top of the runtime
06 proxy         -> forwarding to upstreams: load balancing, health checks, retries
07 security      -> auth, rate limiting, WAF, request smuggling, IP filtering
08 observability -> logging, metrics, tracing, profiling
09 architecture  -> wiring it all into one process: components, config, plugins, shutdown
12 testing       -> load testing, fuzzing, chaos testing for what you built
```

(`13-algorithms/` through `21-reading-list/` are a deep-dive/appendix layer
outside this dependency chain — see `CLAUDE.md`.)

A companion Cargo workspace lives at the repo root (`labs/`, `proxy/`) —
see the root `README.md`. `proxy/` is the actual deliverable; `labs/` are
18 numbered, progressively harder exercises that build up the skills
`proxy/` needs. Each handbook topic's `## Practice` section points at a
specific `labs/` or `proxy/` crate to implement.

## How to use this handbook

1. Work through directories roughly in numeric order — `06-proxy` assumes
   `04-runtime` and `05-http-stack`, not just "some Rust experience".
2. For each topic file, read `## What to learn`, then do the linked exercise
   in `## Practice` before moving on. Don't binge-read all 13 directories
   before writing any code — the `labs/` and `proxy/` crates are where the
   concepts actually stick.
3. Use `21-reading-list/` as background reading in parallel, not a
   prerequisite — nothing in `01`-`09` requires having read a book first.
4. Treat `12-testing/` as an exercise to run *after* a project is working,
   not before — you need a running proxy to load-test or fuzz.

## Scope

In scope: everything needed to hand-build an L7 HTTP proxy — TCP/TLS
termination, HTTP parsing, routing, load balancing, and the security/
observability layers a real deployment needs.

Out of scope: L3/L4-only load balancers (e.g. IPVS-style), non-HTTP
protocols (gRPC gets a passing mention where it affects HTTP/2 handling but
isn't its own topic), and cloud/infra concerns (Kubernetes Ingress
controllers, service meshes) beyond the "what would plug in here" pointers
in `09-architecture/`.
