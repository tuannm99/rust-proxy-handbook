# Kernel Space, Syscall và File Descriptor

File này thuộc nhóm fundamentals. Xem [`02-linux/01-fundamentals.md`](01-fundamentals.md) để có mục lục đầy đủ.

## What to learn

### Vượt qua ranh giới
Khi chương trình của bạn cần làm một việc chỉ kernel làm được (đọc file, gửi packet, allocate bộ nhớ, tạo socket), nó thực
hiện một **syscall**. Đó là một yêu cầu có kiểm soát, được định nghĩa rõ ràng. CPU chuyển sang kernel mode, kernel làm việc
cần đặc quyền, rồi chuyển lại.

`bind()`, `listen()`, `accept()`, `read()`, `write()`, tức mọi thứ trong [`01-network/11-socket.md`](../01-network/11-socket.md),
đều là syscall, hoặc là wrapper mỏng quanh syscall.

```rust
// lời gọi trông vô hại này thực ra là một chuyến đi-về user-space -> kernel-space
let n = socket.read(&mut buf).await?;
```

### Một syscall thực sự tốn bao nhiêu
Mỗi syscall tốn một **context switch**. Đó là thời gian CPU thật, đo được, để đổi mode (và thường cả đổi thread đang chạy).
Chi phí này độc lập với việc syscall đó làm được bao nhiêu việc hữu ích.

CPU phải lưu state chương trình của bạn, chuyển sang mode đặc quyền với một bộ quyền truy cập bộ nhớ khác, kiểm tra yêu cầu,
làm việc, rồi chuyển lại. Không bước nào "miễn phí" như một lời gọi hàm thường, dù trong Rust một wrapper syscall *trông*
giống hệt các lời gọi hàm khác.

Đó là lý do [`01-network/11-socket.md`](../01-network/11-socket.md), [`02-linux/18-zerocopy.md`](18-zerocopy.md) và phần về
vectored I/O trong file đó quan tâm đến *số lượng* syscall, không chỉ số byte được chuyển. `writev` với ba buffer tốn một
context switch. Ba lời gọi `write` riêng tốn ba.

Ở request rate cao, overhead của syscall là một phần CPU thật và đo được. Flamegraph trong
[`08-observability/04-profiling.md`](../08-observability/04-profiling.md) sẽ cho bạn thấy điều đó, khi profile của proxy bị
các frame syscall chiếm phần lớn thay vì logic của chính bạn.

### File descriptor: mọi thứ đều là một con số
Ý tưởng thống nhất của Unix: gần như mọi thứ bạn có thể đọc hoặc ghi (file thường, TCP socket, pipe, timer, eventfd) đều
được process của bạn biểu diễn theo cùng một cách, bằng một số nguyên không âm nhỏ gọi là **file descriptor (fd)**.

Nó là chỉ số của một bảng mà kernel giữ *riêng cho từng process*. Mỗi entry trỏ tới kernel object thật (một file đang mở,
state connection của một socket, ...), cùng với một offset và một số flag.

Sự thống nhất này là lý do [`02-linux/14-epoll.md`](14-epoll.md) đăng ký được một listening socket, một client socket, *và*
một pipe thông thường vào cùng một epoll instance, với cùng một API. Với epoll, chúng chỉ là các fd có thể trở nên "ready".

Nó cũng là lý do [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) và
[`07-security/09-ddos.md`](../07-security/09-ddos.md) coi giới hạn fd (`ulimit -n`) là một trần tài nguyên cứng. Mỗi socket
đang mở, đi vào hay đi ra, chiếm một entry trong bảng của process, và bảng đó có giới hạn tối đa được cấu hình.

Một process khởi động với sẵn ba fd theo quy ước: `0` (stdin), `1` (stdout), `2` (stderr). Mọi fd mà chương trình mở sau đó
nhận số trống tiếp theo, và số đó được dùng lại sau khi đóng.

```rust
use std::os::unix::io::AsRawFd;
let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
println!("fd = {}", listener.as_raw_fd()); // chỉ là một số nguyên
```

### Kernel object tồn tại độc lập với số fd
Số fd chỉ có nghĩa *cục bộ trong process của bạn*. Hai process khác nhau đều có thể có "fd 5" đang mở, trỏ tới hai kernel
object hoàn toàn không liên quan.

Điều thật sự quan trọng là kernel object phía sau con số, và có bao nhiêu fd (trong process này hoặc process khác, sau một
`fork()`) đang tham chiếu tới nó. Object chỉ thực sự được giải phóng khi tham chiếu cuối cùng bị đóng.

Đây là nền cho một gotcha mà bạn sẽ gặp trực tiếp trong
[`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md): drop handle của bạn tới một socket
không có nghĩa connection bên dưới đóng ngay, nếu thứ khác vẫn còn tham chiếu tới nó.

### Kernel mode và user có đặc quyền (root): hai trục khác nhau
Cả hai đều được gọi là "privileged", nhưng cần tách bạch:

- **Kernel mode và user mode** là sự phân biệt ở mức CPU về instruction và vùng nhớ nào được phép. Mọi process, kể cả
  process chạy bằng root, đều chạy ở user mode và phải syscall vào kernel cho các thao tác cần đặc quyền.
- **root và non-root** là sự phân biệt về *quyền* do kernel áp dụng, và hoàn toàn nằm trong user mode. Process của root vẫn
  chạy ở user mode và vẫn gọi cùng các syscall. Nhưng các kiểm tra quyền của kernel (ví dụ "process này có được bind port 443
  không", xem ghi chú về well-known port trong [`01-network/02-addressing.md`](../01-network/02-addressing.md)) cho phép
  nhiều syscall trong số đó thành công hơn.

## Practice
1. Chạy `ls /proc/self/fd` trong shell (hoặc viết một chương trình Rust nhỏ, in `std::process::id()`, rồi kiểm tra
   `/proc/<pid>/fd` từ terminal khác khi nó đang chạy). Xác nhận fd 0/1/2 có mặt. Mở một file và một TCP connection trong
   chương trình và xem các entry số mới xuất hiện.
2. Chạy `strace -c` trên một lần chạy [`labs/00-tcp-server`](../../labs/00-tcp-server) xử lý vài request và đọc bảng tổng hợp.
   Xác định syscall nào chiếm nhiều nhất về số lượng, và nối ít nhất ba trong số đó về các dòng code của bạn.
3. Viết một chương trình mở 5 file mà không đóng cái nào, in số fd của chúng, rồi đóng file thứ ba đã mở và mở một file mới.
   Xác nhận file mới dùng lại số vừa được giải phóng.
4. So sánh output `strace` giữa hai cách: một lần `write()` một `Vec<u8>` đã gộp sẵn từ 3 buffer, và một lần `writev`
   (vectored write, `IoSlice`) với cùng ba buffer chưa gộp. Xác nhận số syscall khác nhau đúng như đoạn văn ở trên dự đoán.
