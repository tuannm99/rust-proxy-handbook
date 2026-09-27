# Open Source
- NGINX — C. Kiến trúc proxy L7 tham chiếu mà toàn bộ handbook này lấy
  cảm hứng; đọc phần event loop và mô hình worker-process.
- Envoy — C++. Đọc để hiểu mô hình config động xDS đứng sau
  [`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md) và chuỗi filter L7 của nó (tương ứng
  với [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md)).
- HAProxy — C. Đọc để hiểu các thuật toán load-balancing
  ([`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)) và giao thức PROXY mà nó khởi xướng
  ([`01-network/14-proxy-protocol.md`](../01-network/14-proxy-protocol.md)).
- Pingora — Rust (Cloudflare). Tương đương Rust thực tế gần nhất với
  [`proxy/README.md`](../../proxy/README.md); đọc code connection pooling và graceful-restart của
  nó.
- linkerd2-proxy — Rust (Buoyant). Một proxy L4/L7 Rust production được
  xây trên Tokio/Hyper/Tower; nhỏ hơn và dễ đọc hơn Pingora để theo dõi
  cách [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) và [`06-proxy/05-retry.md`](../06-proxy/05-retry.md) trở thành code
  thật.
- Hyper — Rust. Thư viện HTTP mà các crate [`labs/`](../../labs) và [`proxy/`](../../proxy) của
  handbook này được xây trên đó; đọc source codec `h1`/`h2` của nó song
  song với [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md).
- Tokio — Rust. Async runtime bên dưới mọi thứ từ [`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md)
  trở đi; đọc source reactor/scheduler sau khi làm bài tập hand-rolled
  executor ở [`03-rust/05-async.md`](../03-rust/05-async.md).
- Tower — Rust. Lớp trừu tượng service/middleware (`Service`, layer) mà
  Hyper và linkerd2-proxy xây trên đó; liên quan khi
  [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md) cần một thiết kế trait thật sự.
