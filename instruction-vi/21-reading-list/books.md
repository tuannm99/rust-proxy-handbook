# Sách
- TCP/IP Illustrated, Vol. 1 (Stevens) — tài liệu tham chiếu cho mọi thứ
  trong `01-network/`; đọc các chương về TCP handshake/retransmit trước
  `01-network/08-tcp.md`.
- The Linux Programming Interface (Kerrisk) — chi tiết ở tầng syscall
  đứng sau `02-linux/`; đọc các chương về epoll và signal trước
  `02-linux/07-epoll.md` và `02-linux/10-signals.md`.
- Rust Atomics and Locks (Mara Bos) — nền tảng thực sự cho
  `03-rust/04-sync.md`; đọc trước khi động vào nội bộ `Arc`/`Mutex` hay
  các connection pool lock-free.
- Programming Rust (Blandy, Orendorff, Tierney) — tài liệu tham khảo Rust
  rộng hơn; hữu ích song song với `03-rust/01-ownership.md` và
  `03-rust/02-lifetimes.md` nếu hai file đó cảm thấy chưa đủ.
- Zero To Production In Rust (Luca Palmieri) — tương đồng thực tế gần
  nhất với mạch dự án của handbook này (xây một web service Rust
  production từ đầu tới cuối); đáng đối chiếu khi bạn tới
  `proxy/README.md`.
- Database Internals (Petrov) — không chuyên về proxy, nhưng các chương
  về caching và consistency khái quát hóa trực tiếp sang
  `05-http-stack/07-cache.md` và `06-proxy/01-upstream.md`.
- Site Reliability Engineering (Google, đọc miễn phí online) — nguồn cho
  các khái niệm vận hành trong `09-architecture/04-graceful-shutdown.md`,
  `09-architecture/06-canary-deploy.md`, và `12-testing/03-chaos.md`.
