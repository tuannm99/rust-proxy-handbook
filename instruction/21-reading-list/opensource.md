# Open Source
- NGINX — C. The reference L7 proxy this whole handbook is modeled on; read the event loop and worker-process model.
- Envoy — C++. Read for the xDS dynamic-config model behind `06-proxy/07-service-discovery.md` and its L7 filter chain (maps to `09-architecture/02-plugin.md`).
- HAProxy — C. Read for load-balancing algorithms (`06-proxy/02-load-balancer.md`) and the PROXY protocol it originated (`01-network/08-proxy-protocol.md`).
- Pingora — Rust (Cloudflare). The closest real-world Rust equivalent of `proxy/README.md`; read its connection pooling and graceful-restart code.
- linkerd2-proxy — Rust (Buoyant). A production Rust L4/L7 proxy built on Tokio/Hyper/Tower; smaller and more readable than Pingora for tracing how `06-proxy/01-upstream.md` and `06-proxy/05-retry.md` become real code.
- Hyper — Rust. The HTTP library this handbook's `labs/` and `proxy/` crates are built on; read its `h1`/`h2` codec source alongside `05-http-stack/01-parser.md`.
- Tokio — Rust. The async runtime underneath everything from `04-runtime/01-tokio.md` onward; read the reactor/scheduler source after doing the hand-rolled-executor exercise in `03-rust/05-async.md`.
- Tower — Rust. The service/middleware abstraction (`Service`, layers) that Hyper and linkerd2-proxy build on; relevant once `09-architecture/02-plugin.md` needs a real trait design.
