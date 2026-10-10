# HTTP Keep-Alive & Connection Reuse

## What to learn

### Persistent connection
Trong HTTP/1.0, mỗi request mặc định mở một connection TCP mới — tốn kém vì TCP handshake ([`01-network/12-tcp.md`](../01-network/12-tcp.md)) và, với HTTPS, còn thêm một TLS handshake đầy đủ nữa ([`01-network/19-tls.md`](../01-network/19-tls.md)). HTTP/1.1 làm connection persistent theo mặc định: sau một response, cùng connection đó ở lại mở cho request tiếp theo trừ khi một trong hai bên gửi `Connection: close`.

### Header hop-by-hop: những gì một proxy phải strip
Trạng thái keep-alive là theo connection, và một tập header mô tả nó cũng
vậy — `Connection`, `Keep-Alive`, `Transfer-Encoding`, `TE`, `Trailer`,
`Upgrade`, `Proxy-*`. Một proxy kết thúc một connection và mở một
connection khác, nên nó phải sinh lại các header này thay vì forward
chúng; forward `Transfer-Encoding` đặc biệt là một trong những cách chuẩn
để tự tạo ra lỗ hổng request-smuggling
([`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md)).

Xem [`05-http-stack/03-hop-by-hop-headers.md`](03-hop-by-hop-headers.md) — bao gồm cả phần `Connection`
nêu tên *thêm* các header để strip, và vì sao việc strip phải xảy ra trước
khi các header đáng tin của chính bạn được áp vào.

### Pipelining (và vì sao nó gần như đã chết)
Pipelining nghĩa là gửi nhiều request trên một connection mà không chờ từng response — được spec cho phép nhưng response vẫn phải quay về đúng thứ tự (head-of-line blocking), và một intermediary hành xử sai trên đường đi có thể làm hỏng cả luồng. Gần như không còn client HTTP/1.1 production nào pipeline nữa; các stream đa hợp của HTTP/2 ([`01-network/17-http2.md`](../01-network/17-http2.md)) giải quyết đúng cùng bài toán đó thay thế.

Gotcha: "không client nào pipeline" không phải lý do để phía *server* của
bạn xử lý sai nó. Nếu byte của một request thứ hai đến trong khi bạn vẫn
đang trả lời request thứ nhất, các hành vi an toàn là xử lý chúng theo
đúng thứ tự hoặc đóng connection — không bao giờ đan xen response, và
không bao giờ âm thầm bỏ byte đã buffer. Một parser bỏ chúng đi là cách
một request bị smuggle biến mất khỏi log của bạn trong khi vẫn tới được
upstream.

### Connection pooling tới upstream
Một reverse proxy nói chuyện với upstream ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)) nên tái sử dụng connection thay vì mở một cái mới cho mỗi request client — pool một tập connection đang mở nhưng rảnh cho mỗi upstream, phát ra một cái cho mỗi request, trả nó về pool khi response hoàn tất (chỉ khi response được framing rõ ràng không mơ hồ và connection không bị đánh dấu `close`).

```rust
// sketch: a bounded per-upstream connection pool
struct Pool {
    idle: VecDeque<Connection>,
    max_idle: usize,
    max_idle_time: Duration, // evict connections idle longer than this
}
```

Việc sizing pool, cuộc đua dead-connection, và khi nào retry trên một
connection mới là an toàn được bàn kỹ ở [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) — file
này nói về *vòng đời* connection ở cả hai phía của proxy.

Gotcha: không bao giờ trả một connection về pool nếu bạn không chắc về
framing của response nó vừa nhận — một EOF bất ngờ, một length không khớp,
một bất thường lúc parse. Bất cứ gì còn lại trong buffer của connection đó
trở thành phần mở đầu của request tiếp theo, chính là desync trong
[`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md). Khi nghi ngờ, hãy đóng nó; một
connection bị bỏ đi tốn một handshake, một connection bị nhiễm độc tốn
một sự cố bảo mật.

### Tinh chỉnh idle timeout
Hai timeout độc lập đáng quan tâm: proxy giữ một connection client mở
không có request bao lâu (quá lâu lãng phí fd/memory cho client rảnh; quá
ngắn gây reconnect không cần thiết), và giữ một connection upstream trong
pool không có traffic bao lâu (phải ngắn hơn idle timeout của chính
upstream, nếu không proxy sẽ phát ra một connection mà upstream đã âm thầm
đóng — một bug "connection reset" kinh điển).

Gotcha: idle timeout phía client không được áp lên các connection cố tình
sống lâu và im lặng theo thiết kế — một WebSocket rảnh
([`05-http-stack/10-websocket.md`](10-websocket.md)), một cuộc gọi gRPC server-streaming
([`05-http-stack/11-grpc.md`](11-grpc.md)), một stream SSE. Áp một timeout "không request
mới trong 60 giây" lên chúng giết các connection đang hoạt động theo một
cái đồng hồ, và các báo cáo bug kết quả ("nó ngắt kết nối mỗi phút") là
một thể loại quen thuộc. Timeout phải theo từng *chế độ* connection, không
phải toàn cục.

### Giới hạn tuổi thọ connection, không chỉ độ rảnh
Một connection luôn bận không bao giờ chạm idle timeout và có thể sống mãi
mãi — gây ra hai vấn đề đáng để có một giới hạn `max_requests` và/hoặc
max-lifetime cố ý:
- **Nó không bao giờ rebalance.** Scale out upstream pool
  ([`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md)) và các connection sống lâu hiện có
  vẫn tiếp tục đi tới host chúng đã gắn với; capacity mới chỉ nhận
  connection mới. Một giới hạn lifetime là thứ cuối cùng phân phối lại
  tải.
- **State theo connection tích tụ.** Buffer lớn dần tới kích thước request
  lớn nhất từng thấy trên connection đó, allocator arena bị phân mảnh
  ([`14-memory/06-fragmentation.md`](../14-memory/06-fragmentation.md)). Recycle connection định kỳ giới hạn
  điều này.

nginx gọi hai cái này là `keepalive_requests` và `keepalive_time`; cả hai
mặc định là giá trị hữu hạn chính vì những lý do trên.

### Phát hiện một connection mà peer đã đóng
TCP không báo cho bạn biết peer đã đóng một connection rảnh cho tới khi bạn thử dùng nó (hoặc một keepalive probe nổ). Một pool phải xử lý "tôi lấy ra một connection nhưng viết vào nó fail ngay lập tức" bằng cách retry trên một connection mới thay vì trả lỗi ra cho client — gắn điều này với [`06-proxy/05-retry.md`](../06-proxy/05-retry.md).

Gotcha: TCP keepalive (`SO_KEEPALIVE`) không phải cái này. Giá trị mặc
định của nó tính bằng *giờ* (`tcp_keepalive_time` là 7200 giây trên
Linux), nên ngay từ đầu nó không phát hiện được gì ở khung thời gian bạn
quan tâm. Nó hữu ích khi được chỉnh xuống vài phút để phát hiện peer chết
sau một NAT hoặc firewall đã âm thầm bỏ state — nhưng nó không thay thế
được idle timeout ở tầng application hay việc xử lý một lần viết fail lúc
lấy connection ra.

### Đóng một connection mà không làm rớt request
Có một cuộc đua không thể tránh: bạn quyết định đóng một connection keep-alive đang rảnh đúng lúc client gửi một request mới trên nó. Request của client đã đang bay; `FIN` của bạn và byte của họ giao nhau trên dây, và họ thấy một reset cho một request server chưa bao giờ xử lý.

Cách giảm nhẹ trên đường response là báo trước: gửi `Connection: close`
trên response *cuối cùng* bạn định phục vụ, để client biết đừng tái sử
dụng connection thay vì phát hiện bằng cách thất bại. HTTP/2 giải quyết
đúng đắn hơn bằng `GOAWAY`, nêu tên stream ID cuối cùng server sẽ xử lý,
cho phép client retry an toàn bất cứ gì trên nó
([`01-network/17-http2.md`](../01-network/17-http2.md)); đây cũng là cơ chế mà graceful shutdown phụ
thuộc vào ([`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md)).

Gotcha: client vẫn có thể đua, và một số không tuân theo
`Connection: close` kịp thời. Một request fail trên một *connection rảnh
được tái sử dụng* với zero byte response nhận được là trường hợp duy nhất
mà ngay cả một retry non-idempotent theo quy ước được coi là an toàn — xem
[`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) để biết vì sao quy ước đó tồn tại và nó ngừng an
toàn ở đâu.

## Practice
Làm theo thứ tự này.

1. Trong [`labs/02-http-server`](../../labs/02-http-server), xác minh bằng `tcpdump` hoặc connection
   logging rằng hai request tuần tự từ một client tái sử dụng một
   connection TCP. **Xong khi** bạn thấy một handshake cho hai request, và
   `Connection: close` tạo ra một `FIN` sau response.
2. Làm các bài tập của [`05-http-stack/03-hop-by-hop-headers.md`](03-hop-by-hop-headers.md). **Xong
   khi** header hop-by-hop bị strip trong một stage sớm và header framing
   được sinh lại từ body bạn thực sự gửi.
3. Thêm timeout theo từng chế độ: một idle timeout cho connection
   keep-alive thông thường mà *không* áp lên các connection đã upgrade
   hoặc streaming. **Xong khi** một connection keep-alive rảnh bị đóng
   đúng lịch và một WebSocket rảnh trên cùng server sống vô thời hạn.
4. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), xây pool theo từng upstream với idle
   eviction. **Xong khi** các connection trong pool cũ hơn ngưỡng bị đóng,
   và pool không bao giờ vượt quá giới hạn cấu hình dưới tải.
5. Mô phỏng upstream âm thầm đóng một connection idle trong pool (đóng nó
   ngoài băng, rồi gửi một request). **Xong khi** proxy phát hiện lần viết
   fail và retry trên một connection mới thay vì trả lỗi ra client.
6. Thêm giới hạn `max_requests` và max-lifetime lên connection trong pool.
   **Xong khi** thêm một upstream mới giữa lúc load-test khiến traffic
   dịch sang nó trong cửa sổ lifetime, thay vì chỉ connection mới tìm ra
   nó.
7. Thêm `Connection: close` trên response cuối cùng trước một lần đóng có
   kế hoạch. **Xong khi** một test lặp đi lặp lại việc idle-out connection
   dưới tải đồng thời tạo ra zero lỗi client thấy được — chạy nó trước khi
   không có thông báo và đếm số lỗi, để biết cuộc đua đó là có thật.
8. Đo tác động của việc reuse: so sánh pooled với một-connection-mỗi-request
   dưới [`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md). **Xong khi** bạn có số p50/p99 cho
   cả hai, trên plain HTTP và TLS riêng biệt (khoảng cách ở TLS mới là nơi
   lợi ích thật sự nằm ở đó).
