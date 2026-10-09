# Socket Programming

Nghiên cứu bind/listen/accept/connect/send/recv/shutdown.

## What to learn

### Vòng đời syscall
`socket()` tạo một file descriptor; `bind()` gắn nó với một địa chỉ/port
cục bộ; `listen()` đánh dấu nó là một passive listening socket với một
backlog queue; `accept()` lấy một kết nối đã hoàn tất ra khỏi backlog đó
và trả về một fd *mới* cho kết nối đó (fd đang listen vẫn tiếp tục lắng
nghe). Ở phía client, `connect()` thực hiện 3-way handshake của TCP. Việc
này ánh xạ trực tiếp tới `TcpListener::bind` + `.accept()` và
`TcpStream::connect` trong Rust, nhưng biết các syscall thô là thứ khiến
một vòng lặp epoll dùng `libc` thô (xem bài tập của [`02-linux/07-epoll.md`](../02-linux/07-epoll.md))
trở nên dễ hiểu thay vì như phép màu.

### Blocking vs non-blocking
Một socket blocking khiến `read`/`write`/`accept` treo thread gọi cho tới
khi có dữ liệu/kết nối sẵn sàng. Một socket non-blocking trả về
`EWOULDBLOCK`/`EAGAIN` ngay lập tức thay vào đó — đây chính là nền tảng mà
một event loop (epoll, xem [`02-linux/07-epoll.md`](../02-linux/07-epoll.md)) được xây trên đó: đăng
ký fd, block trên *nhiều* fd cùng lúc trong `epoll_wait`, và chỉ gọi
`read`/`write` khi được báo fd đã sẵn sàng. `TcpListener`/`TcpStream` của
tokio là non-blocking bên dưới và tự động tích hợp với reactor của nó.

### SO_REUSEADDR và SO_REUSEPORT
`SO_REUSEADDR` cho phép bạn rebind một port vẫn còn ở `TIME_WAIT` từ một
process instance trước đó (thiết yếu cho việc restart nhanh — không có
nó, proxy của bạn sẽ không bind được trong khoảng 60s sau khi restart).
`SO_REUSEPORT` cho phép *nhiều* socket độc lập bind *cùng* một port, với
kernel tự load-balance các kết nối đến giữa chúng — dùng để chạy một
listener cho mỗi worker thread/process thay vì một listener dùng chung
một fd với một lock.

```rust
let socket = tokio::net::TcpSocket::new_v4()?;
socket.set_reuseaddr(true)?;
socket.bind("0.0.0.0:8080".parse()?)?;
let listener = socket.listen(1024)?;
```

### Backlog và các cơn bão kết nối
Backlog của `listen()` là một hàng đợi có giới hạn chứa các kết nối đã
hoàn tất TCP handshake nhưng chưa được `accept()`. Nếu accept loop của bạn
bị chậm lại (ví dụ đang bận làm việc khác), backlog đầy và kernel bắt đầu
drop hoặc reset các SYN mới — điều này biểu hiện ra như những connection
reset bí ẩn ở phía client dưới tải, chứ không phải một lỗi rõ ràng trong
proxy của bạn. Định cỡ backlog và không bao giờ block accept loop quan
trọng hơn vẻ ngoài của nó ở quy mô nhỏ.

### shutdown() vs close()
`shutdown(fd, SHUT_WR)` gửi một TCP FIN, báo cho peer "tôi đã ghi xong",
trong khi vẫn giữ fd mở để đọc — đây là cách bạn báo hiệu kết thúc request
body mà không cúp cả kết nối. `close()` giải phóng hoàn toàn fd. Tokio
expose việc này qua `AsyncWriteExt::shutdown`. Làm sai chỗ này là nguồn
gốc phổ biến của bug kiểu "response bị cắt mất một nửa" trong một proxy tự
viết.

## Practice

1. Trace `strace -f` trên một phiên `nc -l` đơn giản và xác định các
   syscall `socket`/`bind`/`listen`/`accept` theo đúng thứ tự.
2. Implement [`labs/00-tcp-server`](../../labs/00-tcp-server) dùng `TcpListener` của tokio, rồi so
   sánh nó với echo server epoll-thô-dùng-`libc` bạn sẽ xây trong bài tập
   của [`02-linux/07-epoll.md`](../02-linux/07-epoll.md) (một scratch project, không thuộc workspace
   này) — cùng hành vi, code rất khác nhau.
3. Trong phiên bản epoll thô đó, cố tình gọi `read()` non-blocking trước
   khi dữ liệu sẵn sàng và xác nhận bạn nhận được `EAGAIN`; xử lý nó đúng
   cách thay vì coi nó là một lỗi.
4. Đặt `SO_REUSEADDR` trên echo server của bạn và xác nhận (qua `SIGKILL`
   + restart ngay lập tức) rằng nó không còn thất bại với "Address
   already in use".
5. Cố tình làm quá tải accept loop của chính bạn (sleep trước mỗi
   `accept`) và dùng `ss -ltn` để quan sát backlog queue đầy lên.
