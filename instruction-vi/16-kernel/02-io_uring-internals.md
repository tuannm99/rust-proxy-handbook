# io_uring Internals

`02-linux/08-io_uring.md` nói về việc dùng `io_uring` từ phía ứng dụng.
File này nói về cơ chế submission/completion bên dưới khiến nó khác *về
bản chất* so với epoll, chứ không chỉ là một phiên bản nhanh hơn.

## What to learn

### Hai ring buffer, chia sẻ với kernel
`io_uring` thiết lập một **submission queue (SQ)** và **completion queue
(CQ)** dưới dạng các vùng nhớ được `mmap` vào cả process lẫn kernel —
không cần copy để đưa request cho kernel hay đọc kết quả trả về, vì cả hai
phía đều đọc/ghi trên cùng một vùng nhớ. Một submission queue entry
(**SQE**) mô tả một thao tác: syscall-tương-đương nào (`read`, `write`,
`accept`, `connect`, ...), fd nào, buffer nào, offset nào. Một completion
queue entry (**CQE**) báo cáo kết quả khi kernel hoàn tất. Submit nhiều
thao tác nghĩa là ghi nhiều SQE vào ring, chứ không phải gọi nhiều syscall.

### Vì sao đây là một mô hình khác epoll, chứ không chỉ nhanh hơn
epoll chỉ báo cho bạn biết một fd đang *sẵn sàng* — `read()`/`write()`
thực tế vẫn là một syscall bình thường, có thể block, mà bạn tự gọi sau
đó. Với file I/O nói riêng, "sẵn sàng" không thực sự có định nghĩa rõ ràng
như với socket, đó là lý do async file I/O dưới mô hình epoll luôn phải
đẩy sang một thread pool blocking (vấn đề cooperative-scheduling ở
`03-rust/05-async.md` — đây chính xác là lý do `tokio::fs` làm vậy). Các
thao tác `io_uring` thực sự bất đồng bộ ở tầng kernel cho mọi thao tác nó
hỗ trợ, kể cả đọc file: bạn submit SQE và nhận CQE khi xong, không có
syscall block nào trên thread gọi ở bất kỳ thời điểm nào.

### Registered/fixed buffer
Truyền một con trỏ userspace thô trong SQE nghĩa là kernel phải validate
và pin nó lại từ đầu ở mỗi thao tác. Đăng ký trước một tập buffer một lần
(`io_uring_register_buffers`) và tham chiếu chúng bằng index trong các SQE
sau đó bỏ qua việc validate mỗi-thao-tác đó — một lợi ích throughput thực
sự ở tốc độ thao tác cao, đổi lại bạn phải tự quản lý một buffer pool cố
định (xem `14-memory/04-buffer-pool.md`).

### SQPOLL: bỏ qua hoàn toàn syscall submission
Bình thường, sau khi ghi SQE vào ring, bạn vẫn cần một syscall
`io_uring_enter` để báo kernel rằng có việc mới. Ở chế độ `SQPOLL`, một
kernel thread riêng liên tục poll ring SQ, nên một bên submit bận rộn
không bao giờ cần gọi `io_uring_enter` để submit nữa — chỉ thuần ghi ring
buffer, không có syscall nào trong loop. Đổi lại một CPU core (kernel
thread polling) để loại bỏ hoàn toàn overhead syscall submission; chỉ
đáng làm ở tốc độ thao tác rất cao.

### Gotcha: vấn đề khả dụng và bề mặt tấn công là có thật
`io_uring` có lịch sử các CVE leo thang đặc quyền (privilege escalation)
ngay trong chính code của nó — đủ nhiều để một số môi trường (seccomp
profile mặc định của Docker ở một số thời điểm, ChromeOS, một số nền tảng
cloud có quản lý) đã tắt hoặc hạn chế nó hoàn toàn. Kiểm tra phiên bản
kernel thực tế của môi trường triển khai và allow-list syscall trước khi
thiết kế `proxy/` phụ thuộc cứng vào nó; coi nó là một tối ưu có fallback,
không phải một nền tảng.

## Practice
1. Đọc source của crate `io-uring` hoặc `tokio-uring` để xem nó ánh xạ
   việc submit SQE và complete CQE lên future/waker của Rust như thế nào.
2. Benchmark latency đọc file lạnh (cold, nằm trên disk) qua đường
   `tokio::fs` truyền thống (đẩy qua thread pool) so với đường dựa trên
   `io_uring`, trên một hệ thống có cả hai.
3. Đăng ký một tập fixed buffer và so sánh overhead mỗi-thao-tác so với
   truyền con trỏ thô, ở tốc độ thao tác cao.
4. Kiểm tra xem môi trường triển khai thực tế của bạn (một container
   image, một loại cloud VM cụ thể) có cho phép syscall `io_uring` hay
   không — `docker run --security-opt seccomp=...` hoặc docs của nền tảng
   — trước khi giả định nó khả dụng trong production.
