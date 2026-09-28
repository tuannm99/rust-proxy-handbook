# Sách

Tài liệu miễn phí, từ số 0, cho phase 0 ([`00-introduction/02-prerequisites.md`](../00-introduction/02-prerequisites.md)):
- The Rust Programming Language ("the Book", doc.rust-lang.org/book) — phần syntax và ownership cơ bản mà handbook này giả định bạn đã có; dự án cuối của nó, một multithreaded web server, là tiền thân trực tiếp của [`labs/00-tcp-server`](../../labs/00-tcp-server).
- Rustlings (github.com/rust-lang/rustlings) — các bài tập nhỏ không compile cho tới khi sửa đúng; con đường nhanh nhất từ đọc về borrow checker tới làm nó hài lòng.
- Tokio tutorial (tokio.rs/tokio/tutorial) — xây một mini-Redis; bao quát `spawn`, shared state, channel, và framing, bộ công cụ của `labs/00`-`05`.
- Beej's Guide to Network Programming (miễn phí online) — socket từ C, tầng mà tokio che đi; một góc nhìn thứ hai bên cạnh [`01-network/07-socket.md`](../01-network/07-socket.md).
- Operating Systems: Three Easy Pieces (Arpaci-Dusseau, miễn phí online) — phần virtualization và concurrency làm nền cho nhóm fundamentals của [`02-linux/`](../02-linux) và cho [`22-theory/`](../22-theory).

Tài liệu tham khảo sâu hơn:
- TCP/IP Illustrated, Vol. 1 (Stevens) — tài liệu tham chiếu cho mọi thứ
  trong [`01-network/`](../01-network); đọc các chương về TCP handshake/retransmit trước
  [`01-network/08-tcp.md`](../01-network/08-tcp.md).
- The Linux Programming Interface (Kerrisk) — chi tiết ở tầng syscall
  đứng sau [`02-linux/`](../02-linux); đọc các chương về epoll và signal trước
  [`02-linux/07-epoll.md`](../02-linux/07-epoll.md) và [`02-linux/10-signals.md`](../02-linux/10-signals.md).
- Rust Atomics and Locks (Mara Bos) — nền tảng thực sự cho
  [`03-rust/04-sync.md`](../03-rust/04-sync.md); đọc trước khi động vào nội bộ `Arc`/`Mutex` hay
  các connection pool lock-free.
- Programming Rust (Blandy, Orendorff, Tierney) — tài liệu tham khảo Rust
  rộng hơn; hữu ích song song với [`03-rust/01-ownership.md`](../03-rust/01-ownership.md) và
  [`03-rust/02-lifetimes.md`](../03-rust/02-lifetimes.md) nếu hai file đó cảm thấy chưa đủ.
- Zero To Production In Rust (Luca Palmieri) — tương đồng thực tế gần
  nhất với mạch dự án của handbook này (xây một web service Rust
  production từ đầu tới cuối); đáng đối chiếu khi bạn tới
  [`proxy/README.md`](../../proxy/README.md).
- Database Internals (Petrov) — không chuyên về proxy, nhưng các chương
  về caching và consistency khái quát hóa trực tiếp sang
  [`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md) và [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md).
- Site Reliability Engineering (Google, đọc miễn phí online) — nguồn cho
  các khái niệm vận hành trong [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md),
  [`09-architecture/06-canary-deploy.md`](../09-architecture/06-canary-deploy.md), và [`12-testing/03-chaos.md`](../12-testing/03-chaos.md).
