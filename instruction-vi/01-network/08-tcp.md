# TCP/IP

Các chủ đề:
- 3-way handshake
- Congestion Control
- Flow Control
- Keepalive
- TIME_WAIT
- Vòng đời socket

## What to learn

### 3-way handshake
`SYN` -> `SYN-ACK` -> `ACK` thiết lập một kết nối và đồng bộ hóa sequence
number ban đầu trước khi bất kỳ dữ liệu ứng dụng nào chảy. Round trip này
là overhead latency thuần túy mà proxy của bạn trả cho mỗi kết nối
upstream mới — đó là luận điểm cốt lõi cho việc pooling/tái sử dụng kết
nối tới upstream (xem [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)) thay vì dial mới cho mỗi
request.

### TIME_WAIT và vòng đời socket
Bên gửi `FIN` đầu tiên (active closer) rơi vào `TIME_WAIT` trong 2×MSL
(thường khoảng ~60s trên Linux) trước khi fd được giải phóng hoàn toàn, để
đảm bảo bất kỳ packet trùng lặp lạc nào từ kết nối cũ không bị nhầm với
một kết nối mới tái sử dụng cùng 4-tuple. Một proxy bận rộn đóng nhiều kết
nối upstream ngắn hạn có thể tích lũy hàng nghìn socket `TIME_WAIT` và cạn
kiệt ephemeral port — một lý do nữa để tái sử dụng kết nối upstream thay
vì mở-rồi-đóng cho mỗi request.

### Nagle's algorithm và TCP_NODELAY
Thuật toán Nagle gom các write nhỏ lại để tránh gửi nhiều packet tí hon,
đánh đổi latency lấy ít packet hơn trên đường truyền. Kết hợp với delayed
ACK ở phía nhận, việc này có thể cộng thêm tới ~40ms latency không cần
thiết cho các cuộc trao đổi request/response nhỏ. Các proxy (và hầu hết
HTTP server) tắt nó bằng `TCP_NODELAY` vì framing request/response của
HTTP đã tự gom dữ liệu một cách hợp lý ở tầng ứng dụng rồi.

```rust
let stream = tokio::net::TcpStream::connect(addr).await?;
stream.set_nodelay(true)?;
```

### Congestion control (cơ bản)
Bên gửi duy trì một congestion window lớn dần (slow start, rồi congestion
avoidance) cho tới khi mất packet báo hiệu congestion, rồi thu nhỏ lại.
Các thuật toán như Cubic (mặc định trên Linux) hay BBR đánh đổi khác nhau
giữa tín hiệu congestion dựa trên loss vs dựa trên latency. Một proxy
không tự implement cái này (đó là lãnh địa của kernel/TCP-stack) nhưng một
kết nối *mới* luôn bắt đầu từ một congestion window nhỏ — đó là lý do tái
sử dụng kết nối tới upstream quan trọng cho throughput, không chỉ latency.

### Flow control
Khác với congestion control: flow control (receive window của TCP) bảo vệ
*bên nhận* khỏi bị quá tải, độc lập với congestion của mạng. Nếu read loop
của proxy bạn bị chậm lại trong việc drain receive buffer của một socket,
window sẽ thu nhỏ và bên gửi sẽ khựng lại — đây là cách backpressure tự
nhiên lan truyền qua một chuỗi proxy nếu bạn không buffer vô hạn.

### Keepalive
TCP keepalive (`SO_KEEPALIVE` + `TCP_KEEPIDLE`/`TCP_KEEPINTVL`/
`TCP_KEEPCNT`) định kỳ thăm dò một kết nối rảnh để phát hiện một peer đã
chết mà chưa bao giờ gửi `FIN` (ví dụ máy đó bị crash, hoặc một
NAT/firewall âm thầm drop mapping). Điều này khác với HTTP keep-alive ở
*tầng ứng dụng* ([`05-http-stack/04-keepalive.md`](../05-http-stack/04-keepalive.md)) — TCP keepalive phát
hiện một peer đã chết, HTTP keep-alive quyết định có tái sử dụng một kết
nối cho request khác hay không.

## Practice

1. Capture một handshake và một lần teardown kết nối bằng
   `tcpdump -i lo port 8080` trong khi gọi tới [`labs/00-tcp-server`](../../labs/00-tcp-server), và
   xác định chuỗi SYN/SYN-ACK/ACK và FIN/FIN-ACK.
2. Chạy `ss -tn state time-wait | wc -l` trong khi dồn dập gọi tới
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) bằng các kết nối ngắn hạn (không keep-alive),
   rồi lại chạy với tái sử dụng kết nối bật lên — so sánh số lượng.
3. Benchmark latency của request có và không có `set_nodelay(true)` với
   các payload request/response nhỏ và đo sự khác biệt.
4. Cấu hình TCP keepalive trên các kết nối client tới upstream trong
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) và xác nhận (bằng cách kill một upstream
   process mà không đóng socket của nó, ví dụ qua rule drop của
   `iptables`) rằng proxy của bạn cuối cùng phát hiện được peer đã chết.
