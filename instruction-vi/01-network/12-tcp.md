# TCP: Connection, State, và Teardown

TCP hứa những gì, một connection ra đời và chết đi như thế nào, và state
machine điều khiển nó. Các cơ chế làm cho lời hứa trở thành thật (retransmission,
window, congestion control) nằm ở [`13-tcp-reliability.md`](13-tcp-reliability.md); phần cài đặt trong
kernel nằm ở [`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md).

## What to learn

### TCP hứa gì — và không hứa gì
TCP cho hai process một **byte stream hai chiều, tin cậy, đúng thứ tự** trên
nền packet best-effort của IP ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)): mỗi byte gửi đi tới nơi
đúng một lần, đúng thứ tự, hoặc connection báo lỗi. Nó cũng thích ứng tốc độ
với bên nhận (flow control) và với mạng (congestion control).

Thứ nó **không** cho: ranh giới message (hai lần `write` có thể tới thành một
lần `read`, [`03-byte-streams.md`](03-byte-streams.md)); bất kỳ giới hạn nào về *thời gian* (một
packet mất làm đứng cả stream cho tới khi được retransmit); xác nhận giao hàng
tới *application* (một ACK nghĩa là kernel của peer đã có byte, không có nghĩa
là code của peer đã xử lý chúng); hay bảo mật ([`19-tls.md`](19-tls.md)). Một `write()`
thành công chỉ có nghĩa là "kernel đã nhận các byte này vào send buffer."

### Segment header
Mỗi TCP segment mang (20 byte cộng options):

```text
source port | destination port
sequence number         (32 bit: vị trí của byte payload đầu tiên)
acknowledgment number   (32 bit: byte kế tiếp tôi mong đợi từ bạn)
data offset | flags: SYN ACK FIN RST PSH URG | window size
checksum | urgent pointer
options: MSS, window scale, SACK-permitted, timestamps ...
```

Sequence number đếm **byte**, không phải segment. Một segment có `seq=1000`
và 500 byte payload bao phủ byte 1000..1499; bên nhận ack nó bằng `ack=1500`
— "tôi có mọi thứ trước 1500, gửi 1500 tiếp." ACK mang tính **cumulative**:
`ack=1500` xác nhận mọi thứ dưới 1500 cùng lúc. `SYN` và `FIN` mỗi cái tiêu
thụ một sequence number dù không mang data. **4-tuple** (src IP, src port, dst
IP, dst port) định danh một connection; đó là cách một listening port của proxy
phục vụ hàng nghìn client.

### Three-way handshake, kèm con số
```text
client                                   server
  SYN   seq=1000                  ->       (SYN_RECV; entry trong SYN queue)
        <-  SYN+ACK seq=5000 ack=1001
  ACK   seq=1001 ack=5001         ->       (ESTABLISHED; chuyển vào accept queue)
```

Mỗi bên chọn một **initial sequence number** (ISN) ngẫu nhiên — ngẫu nhiên để
packet lạc từ connection cũ, và kẻ giả mạo mù, không dễ rơi trúng vào window.
SYN/SYN-ACK cũng trao đổi **options**: maximum segment size, có hỗ trợ SACK
không, hệ số window-scale và timestamp ([`13-tcp-reliability.md`](13-tcp-reliability.md)). Chúng chỉ
thương lượng được ở đây.

Phía server có hai queue ([`11-socket.md`](11-socket.md)): connection half-open đang chờ ACK
cuối (**SYN queue**) và connection đã hoàn tất đang chờ `accept()` của bạn
(**accept queue**). Một trận flood SYN không bao giờ hoàn tất làm đầy queue
thứ nhất; accept loop chậm làm đầy queue thứ hai. **SYN cookie** cho phép
kernel sống sót qua SYN flood bằng cách mã hóa state vào ISN thay vì lưu nó
([`07-security/09-ddos.md`](../07-security/09-ddos.md)).

Một round trip trôi qua trước khi client gửi được data (nhiều hơn với TLS,
[`19-tls.md`](19-tls.md)): connect của client hoàn tất khi SYN-ACK tới, tức 1 RTT
([`04-latency-throughput.md`](04-latency-throughput.md)). Đó là chi phí mà connection pooling tránh được
([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)).

### Teardown: FIN lịch sự, RST thì không
Mỗi chiều đóng độc lập. `shutdown(SHUT_WR)` (hoặc `close`) gửi `FIN`: "tôi
không còn gì để gửi nữa." Peer ACK nó; peer có thể tiếp tục gửi (**half-close**)
cho tới khi gửi `FIN` của riêng mình, rồi bên đầu ACK cái đó. Một lần đóng
sạch là bốn segment, dù hai cái giữa thường được gộp.

`RST` là hủy bỏ: "connection này không tồn tại / không hợp lệ, quên nó đi."
Bên nhận vứt hết dữ liệu đã buffer và `read`/`write` kế tiếp của nó fail với
`ECONNRESET` (hoặc `EPIPE`). RST được gửi khi một segment tới port không có
listener (`ECONNREFUSED` cho bên đang connect), cho connection không còn tồn
tại, khi app `close()` một socket mà trong buffer còn **dữ liệu nhận chưa
đọc**, hoặc khi nó set `SO_LINGER` bằng 0. Trường hợp thứ ba là bug proxy kinh
điển: đóng client socket trước khi đọc hết request của nó thì client thấy một
reset, và *mất response bạn đã gửi*. Một lần đóng có trật tự thì drain trước,
hoặc `shutdown(SHUT_WR)` rồi đọc tới EOF ([`11-socket.md`](11-socket.md)).

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

Hãy đọc `ss -tan` qua sơ đồ này. Các state quan trọng trong production:

- **`ESTABLISHED`**: bình thường.
- **`CLOSE_WAIT`**: *peer* đã đóng và gửi `FIN`; kernel đang chờ **code của
  bạn** nhận ra (một `read` trả về 0) và `close()`. Một đống `CLOSE_WAIT` tăng
  dần **luôn là bug của application** — leak socket hoặc file descriptor —
  không bao giờ là vấn đề mạng, và kết thúc bằng `EMFILE`
  ([`02-linux/20-limits-and-proc.md`](../02-linux/20-limits-and-proc.md)).
- **`FIN_WAIT_2`**: bạn đã đóng, peer đã ACK nhưng chưa đóng. Linux cho
  timeout (`tcp_fin_timeout`, 60s) để peer im lặng không giữ nó mãi.
- **`TIME_WAIT`**: xem bên dưới.
- **`SYN_RECV`/`SYN_SENT`** số lượng lớn: một SYN flood, hoặc peer bạn dial
  không trả lời.

### TIME_WAIT và ephemeral port
Bên đóng trước (**active closer**) kết thúc ở `TIME_WAIT` trong 2×MSL (60s
trên Linux) để segment trùng lặp trễ từ connection cũ không bị nhầm với
connection mới dùng lại cùng 4-tuple, và để ACK cuối bị mất có thể được gửi
lại. Một proxy dial connection upstream mới cho mỗi request và đóng nó trước
sẽ tích lũy hàng nghìn socket `TIME_WAIT`; mỗi cái giữ một source port, mà bạn
chỉ có ~28.000 (`net.ipv4.ip_local_port_range`) cho mỗi IP:port đích. Chạm
trần thì `connect()` fail với `EADDRNOTAVAIL`. Cách sửa là tái sử dụng
connection (pooling, [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)); `tcp_tw_reuse` (chỉ phía
client) giúp ở rìa; `SO_REUSEADDR` ([`11-socket.md`](11-socket.md)) cho một server *đang
listen* bind lại xuyên qua `TIME_WAIT`. Đừng bao giờ "sửa" bằng
`tcp_tw_recycle` — nó đã bị gỡ vì làm hỏng client đứng sau NAT.

### Keepalive: phát hiện peer đã biến mất
Một TCP connection không có traffic vẫn `ESTABLISHED` mãi mãi, kể cả khi peer
crash hoặc NAT/firewall đã quên mapping — không có packet thì không có lỗi.
TCP keepalive (`SO_KEEPALIVE` với `TCP_KEEPIDLE`/`TCP_KEEPINTVL`/
`TCP_KEEPCNT`) gửi probe sau khi idle và tuyên bố peer chết nếu không được trả
lời. Mặc định là 2 giờ, dài hơn nhiều so với idle timeout của hầu hết NAT
(vài phút), nên proxy phải tự set. Nó khác với HTTP keep-alive
([`05-http-stack/05-keepalive.md`](../05-http-stack/05-keepalive.md)), vốn nói về việc *tái sử dụng* connection.

```rust
// socket2 với feature `all` được bật
use socket2::{SockRef, TcpKeepalive};
let ka = TcpKeepalive::new()
    .with_time(std::time::Duration::from_secs(30))
    .with_interval(std::time::Duration::from_secs(10));
SockRef::from(&stream).set_tcp_keepalive(&ka)?;
```

### Short read, short write, và EOF
Vì TCP là stream, một lần gọi `read` có thể trả về ít byte hơn peer đã write,
và `write` có thể nhận ít hơn bạn đưa khi send buffer đầy. `read` trả về `0`
nghĩa là peer gửi `FIN` (EOF) — một kết thúc *bình thường*, không phải lỗi;
`ECONNRESET` mới là bất thường. Code viết tay phải lặp cho tới khi write hết
byte (`write_all`) và phải coi "read 0" là tín hiệu để kết thúc và `close`.
Hệ quả ở mức wire format là parser phải chịu được một message bị cắt ở bất kỳ
ranh giới byte nào ([`16-http1-wire-format.md`](16-http1-wire-format.md)).

## Practice

1. Bắt trọn vòng đời một connection bằng `tcpdump -i lo -n -S port <p>`
   (`-S` = sequence number tuyệt đối) trong khi gọi
   [`labs/00-tcp-server`](../../labs/00-tcp-server) bằng `nc`. Trên giấy, kiểm chứng phép tính:
   `ack` = `seq` của peer + độ dài payload (+1 cho SYN/FIN).
2. Cố tình tạo CLOSE_WAIT leak: trong một tokio server scratch, `read` một
   connection cho tới khi trả về 0 rồi `std::mem::forget` stream thay vì drop
   nó. Kết nối bằng `nc`, nhấn Ctrl-D (gửi FIN), và cho thấy socket phía server
   kẹt ở `ss -tan state close-wait`; rồi bỏ `forget` và xem nó biến mất.
3. Chạy `ss -tn state time-wait | wc -l` trong khi dội
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) bằng connection upstream ngắn hạn (không
   keep-alive), rồi lại với reuse bật — so sánh số lượng và giải thích bên nào
   giữ `TIME_WAIT`.
4. Gây ra một RST: từ client gửi một request, rồi cho server `close()` mà không
   đọc nó (một scratch server có sleep nhỏ), và cho thấy `ECONNRESET` ở client
   và cờ `R` trong `tcpdump`.
5. Cấu hình TCP keepalive cho các upstream client connection trong
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) và kiểm chứng (kill một upstream process mà
   không đóng socket của nó, ví dụ `sudo iptables -A INPUT -p tcp --dport <p>
   -j DROP` ở phía upstream) rằng proxy của bạn cuối cùng phát hiện được peer
   đã chết; đọc lịch probe từ `tcpdump`.
6. Thu nhỏ `net.ipv4.ip_local_port_range` còn vài trăm port, mở và đóng
   connection trong một vòng lặp không reuse, và bắt lấy `EADDRNOTAVAIL` xuất
   hiện; khôi phục setting sau đó.
