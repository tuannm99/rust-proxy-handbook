# TCP: Connection, State và Teardown

File này nói về: TCP hứa những gì, một connection ra đời và kết thúc như thế nào, và
state machine điều khiển nó. Các cơ chế giúp lời hứa đó thành sự thật (retransmission,
window, congestion control) nằm ở [`13-tcp-reliability.md`](13-tcp-reliability.md).
Phần cài đặt trong kernel nằm ở [`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md).

## What to learn

### TCP hứa gì, và không hứa gì
TCP cho hai process một **byte stream hai chiều, tin cậy, đúng thứ tự**, chạy trên nền
packet best-effort của IP ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)). Mỗi byte gửi đi
hoặc tới nơi đúng một lần và đúng thứ tự, hoặc connection báo lỗi. TCP cũng tự điều
chỉnh tốc độ theo khả năng của bên nhận (flow control) và của mạng (congestion
control).

Những thứ TCP **không** cho bạn:

- **Ranh giới message.** Hai lần `write` có thể tới thành một lần `read`
  ([`03-byte-streams.md`](03-byte-streams.md)).
- **Giới hạn về *thời gian*.** Một packet bị mất làm cả stream đứng lại cho đến khi
  packet đó được gửi lại.
- **Xác nhận dữ liệu đã tới *application*.** Một ACK chỉ nghĩa là kernel của peer đã
  nhận byte, không có nghĩa code của peer đã xử lý chúng.
- **Bảo mật** ([`19-tls.md`](19-tls.md)).

Một `write()` thành công chỉ có nghĩa là "kernel đã nhận các byte này vào send buffer".

### Segment header
Mỗi TCP segment mang các field sau (20 byte, cộng thêm options):

```text
source port | destination port
sequence number         (32 bit: vị trí của byte payload đầu tiên)
acknowledgment number   (32 bit: byte kế tiếp tôi mong đợi từ bạn)
data offset | flags: SYN ACK FIN RST PSH URG | window size
checksum | urgent pointer
options: MSS, window scale, SACK-permitted, timestamps ...
```

Sequence number đếm **byte**, không đếm segment. Một segment có `seq=1000` và 500 byte
payload bao phủ byte 1000..1499. Bên nhận ack bằng `ack=1500`, nghĩa là "tôi đã có mọi
thứ trước 1500, hãy gửi tiếp từ 1500". ACK có tính **cumulative**: `ack=1500` xác nhận
mọi thứ dưới 1500 cùng một lúc. `SYN` và `FIN` mỗi cái chiếm một sequence number, dù
chúng không mang data.

**4-tuple** (src IP, src port, dst IP, dst port) định danh một connection. Nhờ vậy một
listening port của proxy phục vụ được hàng nghìn client.

### Three-way handshake, kèm con số
```text
client                                   server
  SYN   seq=1000                  ->       (SYN_RECV; entry trong SYN queue)
        <-  SYN+ACK seq=5000 ack=1001
  ACK   seq=1001 ack=5001         ->       (ESTABLISHED; chuyển vào accept queue)
```

Mỗi bên chọn một **initial sequence number** (ISN) ngẫu nhiên. Chọn ngẫu nhiên để
packet lạc từ connection cũ, hoặc kẻ giả mạo đoán mò, khó có thể trùng đúng vào window
đang dùng.

SYN và SYN-ACK cũng trao đổi **options**: maximum segment size, có hỗ trợ SACK không,
hệ số window-scale, timestamp ([`13-tcp-reliability.md`](13-tcp-reliability.md)). Các
option này chỉ thương lượng được ở bước này.

Phía server có hai queue ([`11-socket.md`](11-socket.md)):

- **SYN queue**: các connection half-open, đang chờ ACK cuối cùng.
- **Accept queue**: các connection đã hoàn tất, đang chờ `accept()` của bạn.

Một trận SYN flood (SYN gửi tới mà không bao giờ hoàn tất handshake) làm đầy queue thứ
nhất. Accept loop chậm làm đầy queue thứ hai. **SYN cookie** giúp kernel chịu được SYN
flood: thay vì lưu state, nó mã hóa state vào ISN
([`07-security/09-ddos.md`](../07-security/09-ddos.md)).

Phải mất một round trip trước khi client gửi được data (nhiều hơn nếu có TLS,
[`19-tls.md`](19-tls.md)). `connect()` của client hoàn tất khi SYN-ACK tới, tức là mất
1 RTT ([`04-latency-throughput.md`](04-latency-throughput.md)). Connection pooling giúp
tránh chi phí này ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)).

### Teardown: FIN thì lịch sự, RST thì không
Mỗi chiều của connection đóng độc lập. `shutdown(SHUT_WR)` (hoặc `close`) gửi `FIN`
với ý "tôi không còn gì để gửi nữa". Peer ACK nó. Peer vẫn có thể tiếp tục gửi dữ liệu
(**half-close**) cho đến khi gửi `FIN` của chính nó, rồi bên đóng đầu tiên ACK `FIN`
đó. Một lần đóng sạch gồm bốn segment, dù hai segment ở giữa thường được gộp làm một.

`RST` là hủy bỏ: "connection này không tồn tại hoặc không hợp lệ, hãy quên nó đi". Bên
nhận bỏ hết dữ liệu đã buffer, và `read`/`write` tiếp theo của nó fail với `ECONNRESET`
(hoặc `EPIPE`). Kernel gửi RST trong các trường hợp:

- Segment tới một port không có ai listen (bên connect nhận `ECONNREFUSED`).
- Segment thuộc về một connection không còn tồn tại.
- App `close()` một socket khi buffer còn **dữ liệu nhận về nhưng chưa đọc**.
- App đặt `SO_LINGER` bằng 0.

Trường hợp thứ ba là bug proxy kinh điển. Nếu bạn đóng client socket trước khi đọc hết
request của nó, client nhận một reset, và *mất luôn response mà bạn đã gửi*. Cách đóng
đúng là drain dữ liệu trước, hoặc `shutdown(SHUT_WR)` rồi đọc cho đến EOF
([`11-socket.md`](11-socket.md)).

### State machine
```text
            LISTEN                      CLOSED
              | recv SYN, send SYN+ACK    | connect: send SYN
           SYN_RECV <---------------- SYN_SENT
              | recv ACK                  | recv SYN+ACK, send ACK
              +----------> ESTABLISHED <--+
      active close | send FIN        | recv FIN, send ACK  (passive close)
         FIN_WAIT_1                CLOSE_WAIT
           | recv ACK                 | app gọi close(): send FIN
         FIN_WAIT_2                 LAST_ACK
           | recv FIN, send ACK       | recv ACK
         TIME_WAIT --2*MSL-->  CLOSED
```

Hãy đọc output của `ss -tan` đối chiếu với sơ đồ này. Các state quan trọng trong
production:

- **`ESTABLISHED`**: trạng thái bình thường.
- **`CLOSE_WAIT`**: *peer* đã đóng và gửi `FIN`. Kernel đang chờ **code của bạn** nhận
  ra điều đó (một lần `read` trả về 0) rồi gọi `close()`. Số lượng `CLOSE_WAIT` tăng dần
  **luôn là bug của application**: socket hoặc file descriptor bị leak. Nó không bao giờ
  là vấn đề mạng, và cuối cùng dẫn đến `EMFILE`
  ([`02-linux/20-limits-and-proc.md`](../02-linux/20-limits-and-proc.md)).
- **`FIN_WAIT_2`**: bạn đã đóng, peer đã ACK nhưng chưa đóng phía của nó. Linux có
  timeout (`tcp_fin_timeout`, 60 giây) để một peer im lặng không giữ socket mãi mãi.
- **`TIME_WAIT`**: xem phần bên dưới.
- **Nhiều `SYN_RECV`/`SYN_SENT`**: có thể là SYN flood, hoặc peer mà bạn đang dial
  không trả lời.

### TIME_WAIT và ephemeral port
Bên đóng trước (**active closer**) kết thúc ở trạng thái `TIME_WAIT` trong 2×MSL (60
giây trên Linux). Có hai lý do:

- Segment trùng lặp bị trễ từ connection cũ không bị nhầm với connection mới dùng lại
  cùng 4-tuple.
- ACK cuối nếu bị mất thì có thể gửi lại được.

Giả sử proxy của bạn dial một upstream connection mới cho mỗi request và là bên đóng
trước. Nó sẽ tích lũy hàng nghìn socket `TIME_WAIT`. Mỗi socket giữ một source port,
trong khi bạn chỉ có khoảng 28.000 port (`net.ipv4.ip_local_port_range`) cho mỗi cặp
IP:port đích. Khi hết port, `connect()` fail với `EADDRNOTAVAIL`.

Cách sửa chính là reuse connection (pooling,
[`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)). `tcp_tw_reuse` (chỉ áp dụng
phía client) giúp thêm một chút. `SO_REUSEADDR` ([`11-socket.md`](11-socket.md)) cho
phép một server *đang listen* bind lại port dù còn `TIME_WAIT`. Đừng bao giờ "sửa" bằng
`tcp_tw_recycle`: nó đã bị gỡ khỏi kernel vì làm hỏng các client đứng sau NAT.

### Keepalive: phát hiện peer đã biến mất
Một TCP connection không có traffic vẫn ở trạng thái `ESTABLISHED` mãi mãi, kể cả khi
peer đã crash hoặc NAT/firewall đã quên mapping. Không có packet thì không có lỗi nào
được báo.

TCP keepalive (`SO_KEEPALIVE` cùng `TCP_KEEPIDLE`/`TCP_KEEPINTVL`/`TCP_KEEPCNT`) giải
quyết việc này: sau một khoảng idle, nó gửi probe, và nếu không được trả lời thì coi
peer đã chết. Giá trị mặc định là 2 giờ. Con số đó dài hơn nhiều so với idle timeout
của hầu hết NAT (vài phút), nên proxy phải tự đặt lại.

Đừng nhầm TCP keepalive với HTTP keep-alive
([`05-http-stack/05-keepalive.md`](../05-http-stack/05-keepalive.md)). HTTP keep-alive
nói về việc *reuse* connection.

```rust
// socket2 với feature `all` được bật
use socket2::{SockRef, TcpKeepalive};
let ka = TcpKeepalive::new()
    .with_time(std::time::Duration::from_secs(30))
    .with_interval(std::time::Duration::from_secs(10));
SockRef::from(&stream).set_tcp_keepalive(&ka)?;
```

### Short read, short write và EOF
Vì TCP là stream, một lần `read` có thể trả về ít byte hơn lượng peer đã write. Một
lần `write` cũng có thể chỉ nhận một phần dữ liệu bạn đưa, khi send buffer đầy.

`read` trả về `0` nghĩa là peer đã gửi `FIN` (EOF). Đây là một kết thúc *bình thường*,
không phải lỗi. `ECONNRESET` mới là bất thường.

Code tự viết phải lặp cho đến khi write hết byte (`write_all`), và phải coi "read trả
về 0" là tín hiệu để kết thúc và `close`. Về mặt wire format, hệ quả là parser phải
chịu được việc một message bị cắt ở bất kỳ vị trí byte nào
([`16-http1-wire-format.md`](16-http1-wire-format.md)).

## Practice

1. Bắt toàn bộ vòng đời một connection bằng `tcpdump -i lo -n -S port <p>` (`-S` in
   sequence number tuyệt đối), trong lúc dùng `nc` gọi
   [`labs/00-tcp-server`](../../labs/00-tcp-server). Trên giấy, kiểm chứng phép tính:
   `ack` = `seq` của peer + độ dài payload (+1 nếu là SYN/FIN).
2. Cố tình tạo một CLOSE_WAIT leak. Trong một tokio server scratch, `read` một
   connection cho đến khi trả về 0, rồi gọi `std::mem::forget` trên stream thay vì drop
   nó. Connect bằng `nc`, nhấn Ctrl-D (gửi FIN), và cho thấy socket phía server bị kẹt
   ở `ss -tan state close-wait`. Sau đó bỏ `forget` đi và xem nó biến mất.
3. Chạy `ss -tn state time-wait | wc -l` trong lúc dội tải vào
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) khi connection tới upstream
   ngắn hạn (không keep-alive). Rồi chạy lại khi bật reuse. So sánh số lượng và giải
   thích bên nào giữ `TIME_WAIT`.
4. Tạo một RST. Từ client gửi một request, rồi cho server `close()` mà không đọc nó (một
   scratch server có sleep nhỏ). Cho thấy `ECONNRESET` ở client và cờ `R` trong
   `tcpdump`.
5. Cấu hình TCP keepalive cho các upstream client connection trong
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy). Kiểm chứng rằng proxy cuối
   cùng phát hiện được peer đã chết: làm upstream "chết" mà không đóng socket của nó, ví
   dụ `sudo iptables -A INPUT -p tcp --dport <p> -j DROP` ở phía upstream. Đọc lịch gửi
   probe từ `tcpdump`.
6. Thu hẹp `net.ipv4.ip_local_port_range` còn vài trăm port. Mở và đóng connection trong
   một vòng lặp mà không reuse, và bắt lấy lỗi `EADDRNOTAVAIL`. Sau đó khôi phục lại
   setting.

---

<!-- nav -->
← Trước: [`01-network/11-socket.md`](11-socket.md) · [Mục lục](00-README.md) · Tiếp theo: [`01-network/13-tcp-reliability.md`](13-tcp-reliability.md) →
