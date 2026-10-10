# Kernel Space, Syscalls, and File Descriptors

Một phần của chuỗi fundamentals từ-con-số-0 — xem
[`02-linux/01-fundamentals.md`](01-fundamentals.md) để có index đầy đủ.

## What to learn

### Cross ranh giới
Khi chương trình của bạn cần làm một việc chỉ kernel mới làm được — đọc
file, gửi packet, allocate memory, tạo socket — nó thực hiện một
**syscall**: một yêu cầu có kiểm soát, được định nghĩa rõ ràng, chuyển
CPU sang kernel mode, để kernel làm công việc có đặc quyền, rồi chuyển
lại. `bind()`, `listen()`, `accept()`, `read()`, `write()` — mọi thứ
trong [`01-network/11-socket.md`](../01-network/11-socket.md) — đều là syscall, hoặc các wrapper mỏng
quanh chúng.

```rust
// lời gọi trông vô hại này thực ra là một chuyến khứ hồi user-space -> kernel-space
let n = socket.read(&mut buf).await?;
```

### Một syscall thực sự tốn bao nhiêu
Mỗi syscall tốn một **context switch** — thời gian CPU thật, đo được, để
chuyển mode (và thường là chuyển thread nào đang thực sự chạy), độc lập
với việc syscall đó làm được bao nhiêu việc hữu ích. CPU phải lưu trạng
thái chương trình của bạn, chuyển sang mode có đặc quyền với một tập
quyền memory khác, kiểm tra yêu cầu, làm việc, rồi chuyển lại. Không có
bước nào trong đó "miễn phí" như một lời gọi hàm thuần túy, dù từ góc nhìn
của Rust một wrapper syscall *trông* giống hệt một lời gọi hàm khác.

Đây là lý do cụ thể khiến [`01-network/11-socket.md`](../01-network/11-socket.md), [`02-linux/18-zerocopy.md`](18-zerocopy.md),
và phần thảo luận về vectored I/O trong file đó quan tâm đến *số lượng*
syscall, không chỉ số byte di chuyển — `writev` với ba buffer tốn một
context switch; ba lời gọi `write` riêng lẻ tốn ba. Ở request rate cao,
overhead syscall là một phần CPU thật, đo được, chính là thứ mà flamegraph
trong [`08-observability/04-profiling.md`](../08-observability/04-profiling.md) cho bạn thấy khi profile của một
proxy bị chi phối bởi các frame syscall thay vì logic của chính bạn.

### File descriptor: mọi thứ đều là một con số
Ý tưởng thống nhất của Unix là gần như mọi thứ bạn có thể đọc từ hoặc ghi
vào — một file thường, một TCP socket, một pipe, một timer, một eventfd —
đều được biểu diễn với process của bạn theo cùng một cách: một số nguyên
không âm nhỏ gọi là **file descriptor (fd)**. Đó là một index vào một
bảng mà kernel giữ *theo từng process*, và mỗi entry trỏ tới object thật
của kernel (một file đang mở, trạng thái connection của một socket, v.v.)
cùng với một offset và một số flag.

Sự thống nhất này là lý do [`02-linux/14-epoll.md`](14-epoll.md) có thể đăng ký một
listening socket, một client socket, *và* một pipe thuần trên cùng một
epoll instance với cùng một API — với epoll, chúng chỉ là các fd có thể
trở nên "ready." Đây cũng là lý do [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) và
[`07-security/09-ddos.md`](../07-security/09-ddos.md) nói về giới hạn fd (`ulimit -n`) như một trần
tài nguyên cứng: mỗi socket đang mở, dù vào hay ra, tiêu tốn một entry
trong bảng theo-process đó, và bảng có một giới hạn tối đa được cấu hình.

Một process khởi động với ba fd đã sẵn mở theo quy ước: `0` (stdin), `1`
(stdout), `2` (stderr) — mọi fd chương trình bạn mở sau đó nhận số tự do
tiếp theo, được tái sử dụng khi đóng.

```rust
use std::os::unix::io::AsRawFd;
let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
println!("fd = {}", listener.as_raw_fd()); // chỉ là một số nguyên
```

### Kernel object tồn tại độc lập với số fd
Số fd *cục bộ theo process của bạn* — hai process khác nhau đều có thể có
"fd 5" đang mở, trỏ tới các kernel object hoàn toàn không liên quan. Cái
thực sự quan trọng là kernel object đứng sau con số đó là gì và bao nhiêu
fd (trong process này hoặc process khác, sau một `fork()`) hiện đang tham
chiếu tới nó — object chỉ thực sự được giải phóng khi tham chiếu cuối
cùng bị đóng. Đây là nền tảng cho một gotcha bạn sẽ gặp trực tiếp trong
[`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md): drop handle của bạn tới một
socket không nhất thiết có nghĩa connection bên dưới đóng ngay lập tức nếu
thứ khác vẫn còn tham chiếu tới nó.

### Kernel mode vs một user có đặc quyền (root) — hai trục khác nhau
Đáng tách bạch vì cả hai đều bị gọi là "privileged": **kernel vs user
mode** là một phân biệt ở cấp CPU về những instruction và memory nào được
phép — mọi process, kể cả process chạy bởi root, đều thực thi ở user mode
và phải syscall vào kernel cho các thao tác có đặc quyền. **Root vs
non-root** là một phân biệt về *quyền* do kernel enforce, hoàn toàn nằm
trong user mode — process của root vẫn chạy ở user mode và vẫn thực hiện
cùng những syscall, nhưng các kiểm tra quyền của kernel (ví dụ "process
này có được bind port 443 không," ghi chú về well-known-ports trong
[`01-network/02-addressing.md`](../01-network/02-addressing.md)) cho phép nhiều syscall trong số đó thành
công hơn.

## Practice
1. Chạy `ls /proc/self/fd` trong shell (hoặc viết một chương trình Rust
   nhỏ, in `std::process::id()`, và kiểm tra `/proc/<pid>/fd` từ một
   terminal khác trong khi nó chạy) và xác nhận fd 0/1/2 có mặt; mở một
   file và một connection TCP trong chương trình và xem các entry số mới
   xuất hiện.
2. `strace -c` một lần chạy [`labs/00-tcp-server`](../../labs/00-tcp-server) xử lý vài request và đọc
   bảng tổng hợp — xác định syscall nào chiếm ưu thế về số lượng, và nối
   ít nhất ba trong số đó về lại các dòng trong code của bạn.
3. Viết một chương trình mở 5 file mà không đóng bất kỳ file nào, in số
   fd của chúng, rồi đóng file thứ ba đã mở và mở một file mới — xác nhận
   file mới tái sử dụng số vừa được giải phóng.
4. So sánh output `strace` giữa một lần `write()` của một `Vec<u8>` gộp
   sẵn từ 3 buffer với `writev` (vectored write, `IoSlice`) của cùng ba
   buffer chưa gộp — xác nhận số syscall khác nhau đúng bằng lượng mà
   đoạn văn bản trên dự đoán.
