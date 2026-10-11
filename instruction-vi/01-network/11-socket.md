# Socket Programming

File này nói về các syscall của socket: bind, listen, accept, connect, send, recv,
shutdown.

## What to learn

### Các syscall theo thứ tự
- `socket()` tạo một file descriptor.
- `bind()` gắn fd đó với một địa chỉ và port cục bộ.
- `listen()` đánh dấu nó là một socket thụ động (passive), chuyên để nhận connection,
  kèm một backlog queue.
- `accept()` lấy một connection đã hoàn tất ra khỏi backlog queue đó và trả về một fd
  *mới* cho connection ấy. fd đang listen vẫn tiếp tục listen.
- Ở phía client, `connect()` thực hiện 3-way handshake của TCP.

Trong Rust, chúng ánh xạ trực tiếp sang `TcpListener::bind` + `.accept()` và
`TcpStream::connect`. Tuy vậy, bạn vẫn nên biết các syscall thô. Nhờ đó, vòng lặp epoll
dùng `libc` thô (bài tập của [`02-linux/14-epoll.md`](../02-linux/14-epoll.md)) sẽ dễ
hiểu thay vì giống như phép màu.

### Blocking và non-blocking
Với socket blocking, `read`/`write`/`accept` làm thread gọi bị treo cho đến khi có
dữ liệu hoặc connection sẵn sàng. Với socket non-blocking, các lời gọi đó trả về ngay
`EWOULDBLOCK`/`EAGAIN`.

Đây là nền tảng của event loop (epoll, xem
[`02-linux/14-epoll.md`](../02-linux/14-epoll.md)):
1. Đăng ký các fd.
2. Block trên *nhiều* fd cùng lúc bằng `epoll_wait`.
3. Chỉ gọi `read`/`write` khi được báo fd đã sẵn sàng.

`TcpListener` và `TcpStream` của tokio là non-blocking ở bên dưới, và tự tích hợp với
reactor của tokio.

### SO_REUSEADDR và SO_REUSEPORT
`SO_REUSEADDR` cho phép bạn bind lại một port đang ở trạng thái `TIME_WAIT` từ lần chạy
trước của process. Nó rất quan trọng để restart nhanh. Nếu không có nó, proxy của bạn
không bind được trong khoảng 60 giây sau khi restart.

`SO_REUSEPORT` cho phép *nhiều* socket độc lập cùng bind *một* port. Kernel tự load
balance các connection đến giữa chúng. Cách dùng thường gặp: mỗi worker thread/process
có một listener riêng, thay vì cả nhóm dùng chung một fd và phải tranh nhau một lock.

```rust
let socket = tokio::net::TcpSocket::new_v4()?;
socket.set_reuseaddr(true)?;
socket.bind("0.0.0.0:8080".parse()?)?;
let listener = socket.listen(1024)?;
```

### Backlog và các đợt connection dồn dập
Backlog của `listen()` là một queue có giới hạn. Nó chứa các connection đã hoàn tất TCP
handshake nhưng chưa được `accept()`. Nếu accept loop của bạn chậm lại (ví dụ vì đang
bận việc khác), backlog đầy lên. Khi đó kernel bắt đầu drop hoặc reset các SYN mới.

Phía client sẽ thấy những connection reset khó hiểu khi tải cao, còn trong proxy của
bạn không có lỗi rõ ràng nào. Vì vậy, việc chọn kích thước backlog và không bao giờ
block accept loop quan trọng hơn vẻ bề ngoài khi quy mô còn nhỏ.

### shutdown() và close()
`shutdown(fd, SHUT_WR)` gửi một TCP FIN. Nó báo cho peer rằng "tôi đã ghi xong", nhưng
vẫn giữ fd mở để đọc. Đây là cách báo hiệu hết request body mà không cúp cả connection.
`close()` thì giải phóng hoàn toàn fd. Tokio cung cấp việc này qua
`AsyncWriteExt::shutdown`.

Làm sai chỗ này là nguồn gốc phổ biến của bug "response bị cắt mất một nửa" trong
proxy tự viết.

### Địa chỉ, và hỏi một socket về chính nó
Một socket address là một `sockaddr`, gồm: family (`AF_INET`, `AF_INET6`, `AF_UNIX`),
địa chỉ và port. Địa chỉ và port được lưu theo **network byte order** (big-endian). Vì
vậy code `libc` thô phải gọi `htons`/`htonl`, còn `SocketAddr` của Rust che chuyện đó
đi.

- Bind vào `0.0.0.0` (hoặc `[::]`) nghĩa là listen trên mọi interface. Bind vào
  `127.0.0.1` chỉ listen trên loopback. Một server bind như vậy không thể được truy cập
  từ máy khác, hoặc từ network namespace riêng của container
  ([`02-linux/19-netfilter-and-linux-networking.md`](../02-linux/19-netfilter-and-linux-networking.md)).
- Port `0` nghĩa là nhờ kernel chọn một port còn trống. Sau đó `local_addr()` cho bạn
  biết đó là port nào. Cách này hay dùng trong test.
- `getsockname` trả về đầu *của bạn* trong một connection. `getpeername` trả về đầu
  *bên kia* (`peer_addr()`). Nếu có NAT hoặc load balancer đứng trước, peer là
  middlebox đó chứ không phải client thật ([`15-http.md`](15-http.md)).

### Non-blocking connect và các buffer phía sau một socket
`connect()` trên socket non-blocking trả về ngay `EINPROGRESS`. Handshake hoàn tất ở
chế độ nền, và socket trở nên **writable** khi xong (kiểm tra `SO_ERROR` để biết có
thất bại không). Đó chính là thứ `TcpStream::connect(..).await` của tokio bọc lại. Cũng
vì vậy, connect *timeout* là thứ bạn phải tự thêm vào
([`02-linux/11-time-and-timers.md`](../02-linux/11-time-and-timers.md)).

Mỗi socket có một **send buffer** và một **receive buffer** trong kernel:
- `write` thành công khi byte vừa vào được send buffer, không phải khi peer đã nhận
  ([`12-tcp.md`](12-tcp.md)).
- `read` lấy dữ liệu ra khỏi receive buffer.

Kích thước của chúng do `SO_SNDBUF`/`SO_RCVBUF` quyết định. Nhưng nếu bạn tự đặt, Linux
sẽ tắt autotuning ([`13-tcp-reliability.md`](13-tcp-reliability.md)). Đừng đụng đến
chúng trừ khi bạn đang đo đạc.

### Backlog, tính bằng con số
`listen(fd, backlog)` đặt kích thước accept queue. Nhưng kernel **âm thầm giới hạn nó ở
mức `net.core.somaxconn`** (4096 trên kernel gần đây, 128 trên kernel cũ). Vì vậy
`listen(1024)` có thể cho bạn ít hơn mức bạn xin.

Cách theo dõi:
- `ss -ltn` hiện `Recv-Q` (số connection đang chờ) và `Send-Q` (giới hạn thực tế)
  ([`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md)).
- `nstat -az TcpExtListenOverflows TcpExtListenDrops` đếm số connection bị drop vì queue
  đầy.

Nếu bộ đếm overflow cứ tăng, nghĩa là accept loop quá chậm
([`04-runtime/`](../04-runtime)). Không phải do mạng tệ.

### SO_LINGER và `close` thực sự làm gì
Mặc định, `close()` trả về ngay. Kernel tiếp tục gửi nốt dữ liệu chưa gửi ở chế độ nền,
rồi thực hiện FIN đúng trình tự.

`SO_LINGER` thay đổi hành vi đó:
- Với timeout khác 0, `close` block cho đến khi dữ liệu được gửi xong hoặc hết timeout.
- Với timeout **bằng 0**, connection bị hủy ngay bằng một **RST**, và dữ liệu chưa gửi
  bị bỏ ([`12-tcp.md`](12-tcp.md)). Đôi khi người ta dùng cách này để đuổi những client
  cư xử sai mà không tạo ra hàng đống `TIME_WAIT`. Ngoài trường hợp đó nó khá nguy
  hiểm.

## Practice

1. Chạy `strace -f` trên một phiên `nc -l` đơn giản và tìm các syscall
   `socket`/`bind`/`listen`/`accept` theo đúng thứ tự.
2. Implement [`labs/00-tcp-server`](../../labs/00-tcp-server) bằng `TcpListener` của
   tokio. Sau đó so sánh với echo server epoll thô dùng `libc` mà bạn sẽ xây trong bài
   tập của [`02-linux/14-epoll.md`](../02-linux/14-epoll.md) (một scratch project, không
   nằm trong workspace này). Hai bản có cùng hành vi nhưng code rất khác nhau.
3. Trong phiên bản epoll thô đó, cố tình gọi `read()` non-blocking trước khi dữ liệu sẵn
   sàng, và xác nhận bạn nhận được `EAGAIN`. Hãy xử lý nó cho đúng, thay vì coi đó là
   lỗi.
4. Đặt `SO_REUSEADDR` trên echo server của bạn. Xác nhận rằng sau khi `SIGKILL` và
   restart ngay, nó không còn fail với "Address already in use".
5. Cố tình làm accept loop của chính bạn bị quá tải (sleep trước mỗi lần `accept`), rồi
   dùng `ss -ltn` để xem backlog queue đầy dần.
6. Bind vào port `0` và in `local_addr()`. Sau đó so sánh `ss -ltn` giữa `listen(5)` và
   `listen(65535)` với `sysctl net.core.somaxconn`, và xác nhận `Send-Q` thực tế chính
   là giá trị đã bị giới hạn.

---

<!-- nav -->
← Trước: [`01-network/10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) · [Mục lục](00-README.md) · Tiếp theo: [`01-network/12-tcp.md`](12-tcp.md) →
