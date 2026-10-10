# Upstream Pool

## What to learn
### Model một upstream
Một upstream không chỉ là một socket address — nó cần health state, số
connection đang active, và (tùy chọn) một weight. Model nó như một struct
đứng sau `Arc` để load balancer và health checker có thể share nó mà không
phải clone cả pool trên mỗi request.

```rust
struct Upstream {
    addr: std::net::SocketAddr,
    weight: u32,
    healthy: std::sync::atomic::AtomicBool,
    active_conns: std::sync::atomic::AtomicUsize,
}

struct UpstreamPool {
    upstreams: Vec<std::sync::Arc<Upstream>>,
}
```
Gotcha: đừng bọc cả `Vec` trong một `Mutex` nếu bạn chỉ cần lật một health
flag — `Mutex<Vec<Upstream>>` serialize việc chọn upstream của *mọi*
request đằng sau một lock duy nhất. Dùng atomic cho từng upstream, và chỉ
dùng `arc-swap`/`RwLock` cho việc thay đổi membership của pool (xem
[`07-service-discovery.md`](07-service-discovery.md)).

`Ordering::Relaxed` là lựa chọn đúng cho `healthy` và `active_conns` cụ
thể vì không flag nào trong hai flag đó "publish" dữ liệu khác — reader chỉ
cần đúng giá trị của nó, không cần đảm bảo gì về những gì đã được ghi
trước đó. Ngay khi một flag "canh gác" cho các field khác ("healthy nghĩa
là `last_probe_result` hợp lệ"), `Relaxed` là sai và bạn cần
`Release`/`Acquire`; xem [`03-rust/04-sync.md`](../03-rust/04-sync.md).

### Giữ `active_conns` chính xác dưới cancellation
Counter này chỉ hữu ích nếu nó cân bằng chính xác, và phiên bản ngây thơ
thì không:

```rust
upstream.active_conns.fetch_add(1, Ordering::Relaxed);
let resp = forward(&upstream, req).await;   // <-- nếu future này bị drop ở đây...
upstream.active_conns.fetch_sub(1, Ordering::Relaxed);  // <-- ...dòng này không bao giờ chạy
```

Trong một proxy async, một client ngắt kết nối giữa chừng sẽ drop task, và
một future bị drop đơn giản là dừng lại ở `.await` cuối cùng — phép trừ
sau đó không bao giờ thực thi ([`03-rust/05-async.md`](../03-rust/05-async.md): drop *chính là*
cancel). Mỗi request bị cancel sẽ làm phồng counter lên vĩnh viễn.
Least-connection balancing ([`02-load-balancer.md`](02-load-balancer.md)) sau đó sẽ route *tránh
xa* một upstream hoàn toàn khỏe mạnh mãi mãi, và lỗi này im lặng: không
error, không log, chỉ có traffic bị lệch dần trông như một bug thuật toán.

Cách sửa là RAII — giảm counter trong `Drop`, thứ chạy cả trên đường đi
cancellation:

```rust
struct ConnGuard(std::sync::Arc<Upstream>);
impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.0.active_conns.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}
```
Gotcha: đây cùng loại bug với một entry của object pool không được trả lại
([`14-memory/03-object-pool.md`](../14-memory/03-object-pool.md)) — bất kỳ cặp "tăng, làm việc, giảm" thủ
công nào trong code async cũng là một leak đang chờ lần cancellation đầu
tiên.

### Connection reuse tới upstream
Mở một kết nối TCP (+ TLS) mới cho mỗi request được proxy là đắt: một RTT
cho TCP handshake, thêm một hoặc hai RTT nữa cho TLS
([`01-network/19-tls.md`](../01-network/19-tls.md)), phải trả trước khi một byte request nào được
chuyển đi. Giữ một connection pool nhỏ cho mỗi upstream và tái sử dụng các
kết nối idle (`hyper-util`'s `client-legacy` pool làm điều này cho bạn,
nhưng bạn nên biết vì sao nó tồn tại).

Theo dõi riêng HTTP/1.1 keep-alive và HTTP/2 multiplexing: một kết nối
HTTP/1.1 mang đúng một request tại một thời điểm, nên N request đồng thời
tới một upstream cần N kết nối. Một kết nối HTTP/2 mang nhiều stream đồng
thời, nên cùng N request đó có thể chỉ cần một kết nối — bị giới hạn bởi
`SETTINGS_MAX_CONCURRENT_STREAMS` mà upstream công bố
([`01-network/17-http2.md`](../01-network/17-http2.md)), quá ngưỡng đó các stream mới sẽ xếp hàng sau
các stream đã xong thay vì mở kết nối thứ hai, trừ khi bạn cho phép rõ
ràng.

### Sizing cái pool
Kích thước pool thực chất là một giới hạn concurrency được ngụy trang: một
pool HTTP/1.1 giới hạn 16 kết nối idle tới một upstream nghĩa là tối đa 16
request in-flight tới nó, và request thứ 17 phải chờ. Little's law cho ra
cận dưới — concurrency cần thiết = throughput × latency trung bình, nên
1000 req/s ở 20ms là 20 request đồng thời, và một cap 16 kết nối sẽ âm
thầm throttle bạn xuống dưới capacity thật.

Sizing theo hướng ngược lại cũng sai không kém: một pool không giới hạn để
một traffic spike mở hàng nghìn socket tới một upstream, và mỗi socket đó
tốn một fd ở phía bạn và một socket buffer của kernel ở cả hai phía
([`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md)). Lúc đó chính accept backlog của upstream,
chứ không phải pool của bạn, mới là thứ fail.

Gotcha: idle timeout phải *ngắn hơn* keep-alive timeout của chính upstream,
nếu không bạn sẽ thua trong cuộc đua được mô tả tiếp theo. Nếu upstream
đóng các kết nối idle sau 60s, một idle timeout 75s ở phía bạn đảm bảo bạn
sẽ thường xuyên trao ra những kết nối mà upstream đã đóng.

### Kết nối chết trong pool, và khi nào retry nó là an toàn
Một kết nối trong pool mà upstream đã đóng sẽ tạo ra một lỗi ở request kế
tiếp dùng nó — thường trước khi một byte response nào tới. Đây không phải
chuyện hiếm; nó xảy ra mỗi khi keep-alive timeout của upstream, một lần
deploy, hoặc một idle reaper của load balancer kích hoạt giữa hai request
của bạn.

Quy tắc thông thường: một lỗi trên một kết nối *được tái sử dụng*, với
**không có** byte response nào nhận được, được coi là an toàn để retry một
lần trên một kết nối mới ngay cả với một method không idempotent — lý do
là request gần như chắc chắn chưa bao giờ tới được code ứng dụng của
upstream. Chú ý chữ "gần như chắc chắn" đang làm gì ở đó: upstream trên
thực tế có thể đã đọc request, hành động theo nó, rồi chết trước khi
response, khi đó retry sẽ nhân đôi một side effect. Trình duyệt và hầu hết
HTTP client chấp nhận rủi ro này cho các kết nối tái sử dụng; một proxy
thanh toán thì không nên. Hãy quyết định có chủ đích, và xem [`05-retry.md`](05-retry.md)
để biết quy tắc idempotency chung mà đây là một ngoại lệ của nó.

Gotcha: đường retry-một-lần-trên-kết-nối-mới này không được tiêu tốn retry
budget từ [`05-retry.md`](05-retry.md) — đây là retry vì connection-liveness, không phải
retry vì failure, và tính nó vào budget nghĩa là một upstream reap
keep-alive quá tích cực sẽ làm cạn budget của bạn ngay trong vận hành bình
thường.

### Timeout theo từng upstream
"Timeout" thực ra là ít nhất ba con số khác nhau, và gộp chúng thành một
là nguồn gốc phổ biến của cả request bị treo lẫn lỗi giả:
- **Connect timeout** — chờ bao lâu cho TCP (+TLS) handshake. Nên ngắn
  (vài trăm ms trên LAN); một kết nối chậm nghĩa là upstream không thể tới
  được hoặc backlog của nó đầy, không phải nó đang hoạt động.
- **Read/idle timeout** — chờ bao lâu cho byte *tiếp theo* từ upstream.
  Bảo vệ trước một upstream đã accept kết nối rồi treo.
- **Total request timeout** — giới hạn trên cho toàn bộ giao dịch. Cần
  thiết vì một upstream độc hại hoặc hỏng có thể nhỏ giọt một byte ngay
  dưới read timeout mãi mãi, giữ request sống vô thời hạn (bản đối xứng
  phía upstream của tấn công Slowloris trong [`07-security/09-ddos.md`](../07-security/09-ddos.md)).

Gotcha: total request timeout phải tính đến response dạng streaming. Một
total timeout 30s âm thầm phá vỡ một lượt tải file lớn hợp lệ hoặc một
endpoint SSE/long-poll — những cái đó cần read timeout (miễn là có tiến
triển) mà không cần trần tổng, nghĩa là phải route chúng qua một chính
sách timeout khác thay vì một con số toàn cục.

### Weighting và đánh dấu upstream down
Weight thiên vị tần suất một upstream được chọn so với những upstream
khác (một máy to hơn nhận traffic tỷ lệ nhiều hơn). Đánh dấu một upstream
"down" phải rẻ và ít lock vì nó xảy ra trên hot path khi một request fail,
và phải hiển thị ngay lập tức cho lượt chọn tiếp theo của load balancer.

Gotcha: weight và health tương tác tệ nếu balancer đọc chúng riêng rẽ — nó
có thể chọn một upstream weight-10 vì trọng số của nó, rồi phát hiện nó
unhealthy, và hoặc rơi về một lựa chọn thứ hai tồi, hoặc (tệ hơn) lặp lại
việc chọn trong một vòng lặp xoay tròn khi *tất cả* upstream đều unhealthy.
Quyết định trước "tất cả upstream down" thì làm gì: fail fast với 503, hay
vẫn gửi tới cái vừa fail lâu nhất. Cả hai đều hợp lý; xoay tròn thì không.

## Practice
Xây theo thứ tự — tiêu chí hoàn thành của mỗi bước là thứ làm bước tiếp
theo có ý nghĩa.

1. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), định nghĩa `Upstream`/`UpstreamPool` như
   trên với 2-3 địa chỉ tĩnh hardcode. **Xong khi** một request được
   forward tới một trong số chúng và response tới client không đổi.
2. Thêm `active_conns` với một `ConnGuard` kiểu RAII. **Xong khi** một test
   cancel 1000 request giữa chừng (drop future của client, hoặc kill
   client) để lại `active_conns` đúng bằng 0 sau đó — viết phiên bản
   `fetch_sub`-sau-await ngây thơ trước và xem nó kết thúc ở một số khác 0,
   để bạn thấy bug trước khi sửa nó.
3. Nối connection reuse qua `hyper_util::client::legacy::Client`. **Xong
   khi** log (hoặc `ss -tan` nhắm vào upstream) cho thấy request thứ hai
   tái sử dụng một kết nối thay vì mở mới.
4. Đặt idle timeout của pool có chủ đích *dài hơn* keep-alive timeout của
   một upstream giả, rồi chạy traffic ngắt quãng. **Xong khi** bạn có thể
   tái tạo lỗi kết-nối-chết-trong-pool theo yêu cầu; rồi sửa nó theo cả hai
   cách (idle timeout ngắn hơn, cộng với retry-một-lần-trên-kết-nối-mới) và
   xác nhận nó biến mất.
5. Thêm ba loại timeout như các giá trị riêng biệt. **Xong khi** một
   upstream accept rồi treo mãi mãi bị fail bởi read timeout, và một
   upstream nhỏ giọt một byte mỗi giây bị fail bởi total timeout — còn một
   lượt tải streaming 100 MB hợp lệ thì *không* bị fail bởi cái nào cả.
6. Mô phỏng một upstream bị sập (kill backend). **Xong khi** pool đánh dấu
   nó unhealthy và không thread request nào bị block trên nó — đo p99
   latency trong lúc kill và xác nhận nó không tăng vọt quá connect
   timeout của bạn.
