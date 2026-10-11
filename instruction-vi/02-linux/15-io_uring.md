# io_uring

Submission queue và completion queue.

## What to learn

### Mô hình dựa trên completion
epoll nói với bạn "fd này đã sẵn sàng, giờ tự gọi read/write đi". Đó vẫn là API kiểu *readiness*. `io_uring` là API kiểu
*completion*:
1. Bạn nộp một thao tác (read, write, accept, ...) vào một Submission Queue Entry (SQE).
2. Kernel thực hiện nó một cách bất đồng bộ.
3. Một Completion Queue Entry (CQE) xuất hiện khi xong.

Cả hai queue là ring buffer lock-free, được kernel và userspace chia sẻ qua `mmap`. Nhờ vậy việc nộp và thu completion có thể
tránh hẳn syscall trong trường hợp phổ biến (chế độ `SQPOLL`).

### Vì sao nó thắng epoll ở một số workload
epoll vẫn tốn một syscall cho mỗi lần read/write, cộng thêm một syscall cho `epoll_wait`. io_uring gộp được nhiều thao tác vào
một lời gọi `io_uring_enter`.

Ngoài ra, file I/O (khác với socket) hoàn toàn không có khái niệm readiness dưới epoll. io_uring là API đầu tiên của Linux cho
bạn async file I/O thật sự.

Với một L7 proxy chủ yếu truyền dữ liệu từ socket tới socket, lợi ích nhỏ hơn so với workload nặng về storage. io_uring đáng
giá nhất khi bạn còn phục vụ static file ([`05-http-stack/06-static.md`](../05-http-stack/06-static.md)) hoặc cache nặng dựa
trên đĩa.

### Bối cảnh các crate Rust
- `io-uring`: binding mỏng, hơi unsafe, gần với layout ring thô. Bạn tự dựng SQE và thu CQE.
- `tokio-uring`: một *runtime* thay thế (không phải một feature cắm thẳng vào tokio), được xây hoàn toàn quanh io_uring. Nó
  không tương thích đầy đủ với các type `tokio::net` thông thường. Điều này quan trọng nếu bạn muốn dùng nó trong một proxy đang
  chạy tokio.
- `glommio`: một runtime thread-per-core, native io_uring. Đây là một lựa chọn kiến trúc hoàn toàn khác với mô hình
  work-stealing của tokio.

```rust
// Hình dạng thô khi dùng io-uring trực tiếp (crate `io-uring`):
// let mut ring = IoUring::new(256)?;
// let sqe = opcode::Read::new(fd, buf.as_mut_ptr(), buf.len() as _).build();
// unsafe { ring.submission().push(&sqe)?; }
// ring.submit_and_wait(1)?;
// let cqe = ring.completion().next().unwrap();
```

### Khi nào không đáng dùng
Phiên bản kernel rất quan trọng. io_uring dùng được đòi hỏi một kernel khá mới (5.11+ để có hỗ trợ networking chắc chắn). Các
kernel cũ hơn có lỗ hổng bảo mật, nên vài distro tắt nó theo mặc định. Nó cũng từng có CVE thật. Một số môi trường hardened (ví
dụ Docker/Kubernetes với seccomp profile, một số cloud sandbox) chặn hẳn nó. Một proxy *yêu cầu* io_uring có thể đơn giản là không
khởi động được ở đó.

Với một L7 proxy, nút thắt thường là TLS handshake, header parsing và việc reuse upstream connection, không phải số syscall thô.
Vì vậy epoll + tokio gần như luôn là lựa chọn thực dụng. Hãy coi io_uring là một tối ưu hóa để làm sau khi profiling cho thấy
overhead syscall thực sự là nút thắt, không phải mặc định.

## Practice
1. Đọc docs của crate `io-uring` và viết một chương trình tối thiểu đọc một file bằng đúng một vòng SQE/CQE.
2. Mở rộng nó: nộp nhiều lần đọc trước khi thu bất kỳ completion nào, và quan sát việc gộp lời gọi trong `strace`.
3. Chuyển echo server epoll thô từ bài tập của [`02-linux/14-epoll.md`](14-epoll.md) sang `tokio-uring`, và so sánh độ phức tạp
   của code cùng hành vi khi connection liên tục mở và đóng.
4. Kiểm tra `uname -r` trên máy dev của bạn và trên mọi môi trường triển khai đích. Xác nhận io_uring có sẵn và được bật ở đó
   không.
5. Viết ra bằng lời của bạn vì sao [`proxy`](../../proxy) nên mặc định dùng reactor dựa trên epoll của tokio thay vì io_uring.
