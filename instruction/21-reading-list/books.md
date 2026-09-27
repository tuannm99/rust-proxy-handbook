# Books
- TCP/IP Illustrated, Vol. 1 (Stevens) — the reference for everything in `01-network/`; read the TCP handshake/retransmit chapters before `01-network/08-tcp.md`.
- The Linux Programming Interface (Kerrisk) — syscall-level detail behind `02-linux/`; read the epoll and signals chapters before `02-linux/07-epoll.md` and `02-linux/10-signals.md`.
- Rust Atomics and Locks (Mara Bos) — the real backing for `03-rust/04-sync.md`; read before touching `Arc`/`Mutex` internals or lock-free upstream pools.
- Programming Rust (Blandy, Orendorff, Tierney) — broader Rust reference; useful alongside `03-rust/01-ownership.md` and `03-rust/02-lifetimes.md` if those feel thin.
- Zero To Production In Rust (Luca Palmieri) — closest real-world analog to this handbook's project arc (build a production Rust web service end-to-end); good cross-check once you reach `proxy/README.md`.
- Database Internals (Petrov) — not proxy-specific, but the chapters on caching and consistency generalize directly to `05-http-stack/07-cache.md` and `06-proxy/01-upstream.md`.
- Site Reliability Engineering (Google, free online) — the source for the operational concepts in `09-architecture/04-graceful-shutdown.md`, `09-architecture/06-canary-deploy.md`, and `12-testing/03-chaos.md`.
