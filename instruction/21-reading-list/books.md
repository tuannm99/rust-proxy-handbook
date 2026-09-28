# Books

Free, from-zero resources for phase 0 ([`00-introduction/02-prerequisites.md`](../00-introduction/02-prerequisites.md)):
- The Rust Programming Language ("the Book", doc.rust-lang.org/book) — the Rust syntax and ownership basics this handbook assumes; its final project, a multithreaded web server, is the direct predecessor of [`labs/00-tcp-server`](../../labs/00-tcp-server).
- Rustlings (github.com/rust-lang/rustlings) — small compile-until-fixed exercises; the fastest route from reading about the borrow checker to satisfying it.
- Tokio tutorial (tokio.rs/tokio/tutorial) — builds a mini-Redis; covers `spawn`, shared state, channels, and framing, the toolkit of `labs/00`-`05`.
- Beej's Guide to Network Programming (free online) — sockets from C, the layer tokio hides; a second voice alongside [`01-network/07-socket.md`](../01-network/07-socket.md).
- Operating Systems: Three Easy Pieces (Arpaci-Dusseau, free online) — the virtualization and concurrency parts back [`02-linux/`](../02-linux)'s fundamentals and [`22-theory/`](../22-theory).

Deeper references:
- TCP/IP Illustrated, Vol. 1 (Stevens) — the reference for everything in [`01-network/`](../01-network); read the TCP handshake/retransmit chapters before [`01-network/08-tcp.md`](../01-network/08-tcp.md).
- The Linux Programming Interface (Kerrisk) — syscall-level detail behind [`02-linux/`](../02-linux); read the epoll and signals chapters before [`02-linux/07-epoll.md`](../02-linux/07-epoll.md) and [`02-linux/10-signals.md`](../02-linux/10-signals.md).
- Rust Atomics and Locks (Mara Bos) — the real backing for [`03-rust/04-sync.md`](../03-rust/04-sync.md); read before touching `Arc`/`Mutex` internals or lock-free upstream pools.
- Programming Rust (Blandy, Orendorff, Tierney) — broader Rust reference; useful alongside [`03-rust/01-ownership.md`](../03-rust/01-ownership.md) and [`03-rust/02-lifetimes.md`](../03-rust/02-lifetimes.md) if those feel thin.
- Zero To Production In Rust (Luca Palmieri) — closest real-world analog to this handbook's project arc (build a production Rust web service end-to-end); good cross-check once you reach [`proxy/README.md`](../../proxy/README.md).
- Database Internals (Petrov) — not proxy-specific, but the chapters on caching and consistency generalize directly to [`05-http-stack/07-cache.md`](../05-http-stack/07-cache.md) and [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md).
- Site Reliability Engineering (Google, free online) — the source for the operational concepts in [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md), [`09-architecture/06-canary-deploy.md`](../09-architecture/06-canary-deploy.md), and [`12-testing/03-chaos.md`](../12-testing/03-chaos.md).
