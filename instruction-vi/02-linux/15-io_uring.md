# io_uring

Submission/completion queues.

## What to learn

### Mô hình dựa trên completion
epoll báo cho bạn "fd này sẵn sàng, giờ tự gọi read/write đi" — vẫn là một
API readiness. `io_uring` là một API *completion*: bạn nộp một thao tác
(read, write, accept, ...) vào một Submission Queue Entry (SQE), kernel
thực hiện nó một cách bất đồng bộ, và một Completion Queue Entry (CQE)
xuất hiện khi xong. Cả hai queue là ring buffer lock-free được chia sẻ qua
`mmap` giữa kernel và userspace, nên việc nộp/thu completion có thể tránh
hoàn toàn một syscall trong trường hợp phổ biến (chế độ `SQPOLL`).

### Vì sao nó thắng epoll ở một số workload
Epoll vẫn tốn một syscall mỗi lần read/write cộng một cho `epoll_wait`.
io_uring có thể gộp nhiều thao tác vào một lời gọi `io_uring_enter`, và
file I/O (không như socket) hoàn toàn không có khái niệm readiness dưới
epoll — io_uring là API đầu tiên của Linux cho bạn async file I/O thật
sự. Với một proxy L7 chủ yếu là socket-tới-socket, phần thắng nhỏ hơn so
với một workload nặng về storage; io_uring đáng giá nhất khi bạn cũng
phục vụ static file ([`05-http-stack/06-static.md`](../05-http-stack/06-static.md)) hoặc làm caching nặng
dựa trên disk.

### Bối cảnh crate Rust
- `io-uring`: binding mỏng, hơi unsafe, gần với layout ring thô — bạn tự
  xây SQE và thu CQE.
- `tokio-uring`: một *runtime* thay thế (không phải một feature cắm thẳng
  vào tokio) được xây hoàn toàn quanh io_uring; nó không tương thích đầy
  đủ với các type `tokio::net` thông thường, điều này quan trọng nếu bạn
  muốn trộn nó vào một proxy đang dùng tokio.
- `glommio`: một runtime thread-per-core, native io_uring, một canh bạc
  kiến trúc hoàn toàn khác so với mô hình work-stealing của tokio.

```rust
// Hình dạng thô của việc dùng io-uring trực tiếp (crate `io-uring`):
// let mut ring = IoUring::new(256)?;
// let sqe = opcode::Read::new(fd, buf.as_mut_ptr(), buf.len() as _).build();
// unsafe { ring.submission().push(&sqe)?; }
// ring.submit_and_wait(1)?;
// let cqe = ring.completion().next().unwrap();
```

### Khi nào nó không đáng dùng
Phiên bản kernel quan trọng rất nhiều: io_uring dùng được cần một kernel
khá mới (5.11+ để hỗ trợ networking chắc chắn; các kernel cũ hơn có lỗ
hổng bảo mật khiến vài distro tắt nó theo mặc định). Nó cũng từng có CVE
thật, và một số môi trường hardened (ví dụ Docker/Kubernetes với seccomp
profile, một số cloud sandbox) block nó hoàn toàn — một proxy *yêu cầu*
io_uring có thể đơn giản là không khởi động được ở đó. Với một proxy L7
mà bottleneck thường là TLS handshake, header parsing, và tái sử dụng connection upstream chứ không phải số lượng syscall thô, epoll + tokio gần như
luôn là lựa chọn thực dụng; coi io_uring là một tối ưu để với tới sau khi
profiling cho thấy overhead syscall thực sự là bottleneck của bạn, không
phải một mặc định.

## Practice
1. Đọc docs crate `io-uring` và viết một chương trình tối thiểu đọc một
   file với một vòng SQE/CQE duy nhất.
2. Mở rộng nó để nộp nhiều lần đọc trước khi thu bất kỳ completion nào,
   và quan sát việc gộp trong `strace`.
3. Chuyển echo server raw-epoll từ bài tập [`02-linux/14-epoll.md`](14-epoll.md) sang
   `tokio-uring` và so sánh độ phức tạp code và hành vi dưới connection
   churn.
4. Kiểm tra `uname -r` trên máy dev của bạn và bất kỳ môi trường triển
   khai đích nào; xác nhận io_uring có khả dụng/bật ở đó không.
5. Viết ra, bằng lời của bạn, vì sao [`proxy`](../../proxy) nên mặc định dùng reactor
   dựa trên epoll của tokio thay vì io_uring.
