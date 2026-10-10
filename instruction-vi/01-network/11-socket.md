# Socket Programming

Nghiên cứu bind/listen/accept/connect/send/recv/shutdown.

## What to learn

### Vòng đời syscall
`socket()` tạo một file descriptor; `bind()` gắn nó với một địa chỉ/port
cục bộ; `listen()` đánh dấu nó là một passive listening socket với một
backlog queue; `accept()` lấy một connection đã hoàn tất ra khỏi backlog đó
và trả về một fd *mới* cho connection đó (fd đang listen vẫn tiếp tục lắng
nghe). Ở phía client, `connect()` thực hiện 3-way handshake của TCP. Việc
này ánh xạ trực tiếp tới `TcpListener::bind` + `.accept()` và
`TcpStream::connect` trong Rust, nhưng biết các syscall thô là thứ khiến
một vòng lặp epoll dùng `libc` thô (xem bài tập của [`02-linux/14-epoll.md`](../02-linux/14-epoll.md))
trở nên dễ hiểu thay vì như phép màu.

### Blocking vs non-blocking
Một socket blocking khiến `read`/`write`/`accept` treo thread gọi cho tới
khi có dữ liệu/connection sẵn sàng. Một socket non-blocking trả về
`EWOULDBLOCK`/`EAGAIN` ngay lập tức thay vào đó — đây chính là nền tảng mà
một event loop (epoll, xem [`02-linux/14-epoll.md`](../02-linux/14-epoll.md)) được xây trên đó: đăng
ký fd, block trên *nhiều* fd cùng lúc trong `epoll_wait`, và chỉ gọi
`read`/`write` khi được báo fd đã sẵn sàng. `TcpListener`/`TcpStream` của
tokio là non-blocking bên dưới và tự động tích hợp với reactor của nó.

### SO_REUSEADDR và SO_REUSEPORT
`SO_REUSEADDR` cho phép bạn rebind một port vẫn còn ở `TIME_WAIT` từ một
process instance trước đó (thiết yếu cho việc restart nhanh — không có
nó, proxy của bạn sẽ không bind được trong khoảng 60s sau khi restart).
`SO_REUSEPORT` cho phép *nhiều* socket độc lập bind *cùng* một port, với
kernel tự load-balance các connection đến giữa chúng — dùng để chạy một
listener cho mỗi worker thread/process thay vì một listener dùng chung
một fd với một lock.

```rust
let socket = tokio::net::TcpSocket::new_v4()?;
socket.set_reuseaddr(true)?;
socket.bind("0.0.0.0:8080".parse()?)?;
let listener = socket.listen(1024)?;
```

### Backlog và các cơn bão connection
Backlog của `listen()` là một queue có giới hạn chứa các connection đã
hoàn tất TCP handshake nhưng chưa được `accept()`. Nếu accept loop của bạn
bị chậm lại (ví dụ đang bận làm việc khác), backlog đầy và kernel bắt đầu
drop hoặc reset các SYN mới — điều này biểu hiện ra như những connection
reset bí ẩn ở phía client dưới tải, chứ không phải một lỗi rõ ràng trong
proxy của bạn. Định cỡ backlog và không bao giờ block accept loop quan
trọng hơn vẻ ngoài của nó ở quy mô nhỏ.

### shutdown() vs close()
`shutdown(fd, SHUT_WR)` gửi một TCP FIN, báo cho peer "tôi đã ghi xong",
trong khi vẫn giữ fd mở để đọc — đây là cách bạn báo hiệu kết thúc request
body mà không cúp cả connection. `close()` giải phóng hoàn toàn fd. Tokio
expose việc này qua `AsyncWriteExt::shutdown`. Làm sai chỗ này là nguồn
gốc phổ biến của bug kiểu "response bị cắt mất một nửa" trong một proxy tự
viết.

### Địa chỉ, và hỏi một socket về chính nó
Một socket address là một `sockaddr`: family (`AF_INET`, `AF_INET6`, `AF_UNIX`), địa chỉ và port, theo **network byte order** (big-endian) — vì thế code `libc` thô gọi
`htons`/`htonl` còn `SocketAddr` của Rust ẩn nó đi. Bind vào `0.0.0.0` (hoặc `[::]`) là listen trên mọi interface; `127.0.0.1` chỉ trên loopback, nên một server bind ở đó không
với tới được từ máy khác hay từ network namespace riêng của container
([`02-linux/19-netfilter-and-linux-networking.md`](../02-linux/19-netfilter-and-linux-networking.md)). Port `0` nhờ kernel chọn một port trống (`local_addr()` sau đó cho bạn biết là port nào) — cách dùng quen thuộc trong
test. `getsockname` trả về đầu *của bạn* của một connection và `getpeername` đầu *bên kia* (`peer_addr()`); đứng sau NAT hay load balancer thì peer là middlebox, không phải client
([`15-http.md`](15-http.md)).

### Non-blocking connect và các buffer đằng sau một socket
`connect()` trên một socket non-blocking trả về `EINPROGRESS` ngay lập tức; handshake hoàn tất ở nền và socket trở nên **writable** khi xong (kiểm tra `SO_ERROR` để biết thất bại)
— chính là thứ `TcpStream::connect(..).await` của tokio bọc lại, và là lý do một connect *timeout* phải do bạn thêm vào ([`02-linux/11-time-and-timers.md`](../02-linux/11-time-and-timers.md)). Mỗi socket có một **send buffer** và **receive
buffer** trong kernel: `write` thành công khi byte vừa vào send buffer (không phải khi peer đã nhận, [`12-tcp.md`](12-tcp.md)); `read` drain receive buffer. Kích thước do `SO_SNDBUF`/`SO_RCVBUF`
quy định, nhưng đặt chúng sẽ tắt autotuning của Linux ([`13-tcp-reliability.md`](13-tcp-reliability.md)) — đừng đụng tới trừ khi đang đo.

### Backlog, bằng con số
`listen(fd, backlog)` đặt kích thước accept queue, nhưng kernel **âm thầm clamp nó vào `net.core.somaxconn`** (4096 trên kernel gần đây, 128 trên kernel cũ), nên `listen(1024)` có thể cho ít hơn
bạn xin. Hãy theo dõi: `ss -ltn` hiện `Recv-Q` (connection đang chờ) so với `Send-Q` (giới hạn hiệu dụng) ([`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md)); `nstat -az TcpExtListenOverflows TcpExtListenDrops` đếm connection bị drop vì
queue đầy. Một bộ đếm overflow tăng dần nghĩa là accept loop quá chậm ([`04-runtime/`](../04-runtime)), không phải mạng tệ.

### SO_LINGER và `close` thực sự làm gì
Theo mặc định `close()` trả về ngay và kernel tiếp tục gửi mọi dữ liệu chưa gửi ở nền, rồi thực hiện FIN có trật tự. `SO_LINGER` với timeout khác 0 làm `close` block cho tới khi dữ liệu được gửi hoặc hết timeout; với
timeout **bằng 0** nó hủy connection bằng một **RST** tức thì, vứt dữ liệu chưa gửi ([`12-tcp.md`](12-tcp.md)) — thỉnh thoảng được dùng để loại các client hành xử sai mà không chất đống `TIME_WAIT`, nguy hiểm trong trường hợp khác.

## Practice

1. Trace `strace -f` trên một phiên `nc -l` đơn giản và xác định các
   syscall `socket`/`bind`/`listen`/`accept` theo đúng thứ tự.
2. Implement [`labs/00-tcp-server`](../../labs/00-tcp-server) dùng `TcpListener` của tokio, rồi so
   sánh nó với echo server epoll-thô-dùng-`libc` bạn sẽ xây trong bài tập
   của [`02-linux/14-epoll.md`](../02-linux/14-epoll.md) (một scratch project, không thuộc workspace
   này) — cùng hành vi, code rất khác nhau.
3. Trong phiên bản epoll thô đó, cố tình gọi `read()` non-blocking trước
   khi dữ liệu sẵn sàng và xác nhận bạn nhận được `EAGAIN`; xử lý nó đúng
   cách thay vì coi nó là một lỗi.
4. Đặt `SO_REUSEADDR` trên echo server của bạn và xác nhận (qua `SIGKILL`
   + restart ngay lập tức) rằng nó không còn thất bại với "Address
   already in use".
5. Cố tình làm quá tải accept loop của chính bạn (sleep trước mỗi
   `accept`) và dùng `ss -ltn` để quan sát backlog queue đầy lên.
6. Bind vào port `0`, in `local_addr()`, rồi so sánh `ss -ltn` cho `listen(5)` vs
   `listen(65535)` với `sysctl net.core.somaxconn`, và xác nhận `Send-Q` hiệu dụng
   là giá trị đã bị clamp.
