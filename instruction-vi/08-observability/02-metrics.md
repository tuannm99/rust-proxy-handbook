# Metrics
Prometheus/OpenTelemetry.

## What to learn
### Counter, gauge, histogram
Một **counter** chỉ tăng (tổng số request, tổng số lỗi) — hữu ích với
`rate()` trong PromQL để có requests/sec. Một **gauge** có thể tăng hoặc
giảm (số connection đang mở, số upstream hiện đang healthy). Một
**histogram** gom các quan sát (thời lượng request) vào các bucket để bạn
có thể suy ra p50/p95/p99 ở phía server mà không cần ship raw sample. Một
proxy sống chết nhờ histogram — trung bình cộng che giấu mất 1% chậm mà
người dùng của một load balancer thực sự cảm nhận.

```rust
use prometheus::{register_histogram_vec, HistogramVec};

static REQUEST_DURATION: once_cell::sync::Lazy<HistogramVec> = once_cell::sync::Lazy::new(|| {
    register_histogram_vec!(
        "proxy_request_duration_seconds",
        "Request duration",
        &["method", "status"]
    ).unwrap()
});
```

Gotcha: counter reset về 0 khi process restart, đó là lý do bạn hầu như
không bao giờ đọc giá trị thô của một counter — `rate()` và `increase()`
hiểu về các lần reset và tính toán xuyên qua chúng. Một panel dashboard
hiển thị `_total` thô đang thể hiện "thời gian kể từ lần deploy gần nhất"
nhiều như bất cứ điều gì khác có ý nghĩa thật.

### Percentile không lấy trung bình được — đây là điều cần nắm đúng
Một histogram cho bạn percentile *vì* các bucket có thể cộng dồn. Mười
instance proxy mỗi cái export số đếm bucket riêng; bạn cộng các bucket qua
các instance rồi mới tính quantile:

```promql
histogram_quantile(0.99, sum by (le) (rate(proxy_request_duration_seconds_bucket[5m])))
```

Điều bạn **không được** làm là tính p99 trên từng instance rồi lấy trung
bình các con số đó. Trung bình của mười p99 không phải là p99 của cả fleet
và hoàn toàn không có ý nghĩa thống kê — nó sai một cách có hệ thống,
thường là lạc quan hơn thực tế, và trông hoàn toàn hợp lý trên một
dashboard. Điều này cũng đúng theo thời gian: bạn không thể lấy trung bình
p99 trong một giờ để ra p99 của giờ đó.

Gotcha: đây chính xác là lý do vì sao kiểu **Summary** của Prometheus là
một cái bẫy cho một service nhiều instance. Một Summary tính quantile *ở
phía client*, bên trong từng process, nên những gì nó export đã là các con
số đã bị "sụp" lại, không thể re-aggregate qua các instance. Hãy dùng
histogram cho bất cứ thứ gì bạn sẽ có nhiều hơn một bản sao — với một
proxy, đó là mọi thứ.

### Ranh giới bucket là một quyết định thiết kế
Độ phân giải của một histogram hoàn toàn do các bucket của nó quyết định,
được chọn trước. Các bucket mặc định của Prometheus dừng ở khoảng 10 giây
và được giãn cách cho công việc web chung chung — nếu p99 của proxy bạn là
3ms, mọi request rơi vào bucket đầu tiên và `histogram_quantile` nội suy
bên trong nó, cho ra một con số trông chắc chắn nhưng về cơ bản là bịa ra.

Hãy chọn bucket quanh khoảng latency mà bạn thực sự quan tâm, và đặt một
ranh giới đúng ngay ngưỡng SLO của bạn ([`08-observability/06-alerting.md`](06-alerting.md)):
với một cạnh bucket ở 250ms, "tỉ lệ request dưới 250ms" trở thành một con
số đếm chính xác thay vì một phép nội suy.

Gotcha: bucket có cái giá là cardinality. Một histogram với 20 bucket và 3
label mỗi label 10 giá trị là 20 × 1000 = 20.000 series chỉ từ một metric.
Histogram dạng native (exponential) trong các phiên bản Prometheus mới hơn
tránh được sự đánh đổi này, nhưng chừng nào bạn chưa dùng chúng, "thêm
bucket" không hề miễn phí.

### Phương pháp RED cho một proxy
Với mỗi hop (client-facing và mỗi upstream) theo dõi: **R**ate
(request/giây), **E**rrors (tỉ lệ non-2xx hoặc kết nối thất bại),
**D**uration (histogram latency). Đây là dashboard tối thiểu để trả lời
"proxy có healthy không" và "upstream X có healthy không" mà không cần
đoán. Kết hợp với USE (Utilization/Saturation/Errors) cho chính cái máy
(CPU, số fd, độ bão hòa connection pool).

Với riêng một proxy, đo duration ở **cả hai** đầu và export phần chênh
lệch. Tổng latency client quan sát được trừ đi thời gian phản hồi của
upstream chính là overhead của riêng proxy — queueing, TLS, kiểm tra WAF
([`07-security/06-waf.md`](../07-security/06-waf.md)), lấy connection từ pool. Không có phép trừ đó,
mọi cuộc điều tra latency đều bắt đầu bằng "do mình hay do họ?" mà không
có dữ liệu để trả lời.

Các tín hiệu đặc thù của proxy nên có ngay từ ngày đầu, không cái nào nằm
trong một dashboard RED chung chung:
- **Độ bão hòa connection pool** trên mỗi upstream
  ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)) — việc chờ một connection trong pool là vô
  hình trong thời gian phản hồi của upstream.
- **Trạng thái retry và circuit-breaker** ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)) — tỉ lệ
  retry, cạn ngân sách retry, các lần chuyển trạng thái circuit.
- **Số lượng upstream healthy** dưới dạng gauge
  ([`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md)).
- **Tỉ lệ cache hit** ([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)), giải thích các thay
  đổi tải upstream không liên quan gì tới traffic của client.
- **Độ sâu queue / số lượng bị shed** ([`07-security/09-ddos.md`](../07-security/09-ddos.md)), tín
  hiệu sớm nhất của tình trạng quá tải.

### Bùng nổ cardinality
Mỗi tổ hợp giá trị label duy nhất tạo ra một time series mới. Một label
như `path` trên một proxy forward các path tùy ý do người dùng cung cấp
(`/users/12345`, `/users/12346`, ...) có thể tạo ra cardinality không giới
hạn và làm sập backend metrics của bạn (Prometheus bị OOM, hoặc hóa đơn
của bạn nổ tung). Hãy chuẩn hóa các đoạn path động thành route template
(`/users/:id`) trước khi dùng chúng làm label, và không bao giờ đưa raw
user input, request ID, hay IP vào một label.
Gotcha: đây là cách phổ biến nhất khiến một PR "chỉ thêm một metric" biến
thành một cuộc gọi page lúc 3 giờ sáng cho team observability.

Gotcha: cardinality có tính *nhân*. Bốn label với 10, 20, 5, và 50 giá trị
là 50.000 series cho mỗi metric — mỗi series tốn bộ nhớ cả ở phía proxy
lẫn ở backend. Trước khi thêm một label, hãy nhân nó ra; và coi "label đến
từ route table" (có giới hạn, biết trước khi load config) là quy tắc,
"label đến từ request" là ngoại lệ cần được biện minh.

Gotcha: status code làm label thì ổn (có giới hạn), nhưng *text* của
status code hay một chuỗi thông báo lỗi thì không. `status="500"` là một
series; `error="connection refused to 10.0.0.7:8080"` là một series cho
mỗi upstream, mỗi port, mỗi cách diễn đạt.

### Metrics tốn chi phí trên hot path
Mỗi lần cập nhật metric xảy ra trên mỗi request. Một counter là một phép
tăng nguyên tử, rẻ nhưng không miễn phí — một counter toàn cục duy nhất
được 16 worker thread cùng chạm vào là một cache line bị tranh chấp
([`17-performance/02-false-sharing.md`](../17-performance/02-false-sharing.md)), và ở tốc độ request cao điều này
sẽ hiện ra trong một profile.

Chi phí lớn hơn thường là *tra cứu label*: `with_label_values(&["GET",
"200"])` hash các chuỗi label để tìm đúng metric con trên mỗi lần gọi. Hãy
resolve tập label một lần (theo route, lúc load config, hoặc cache theo
connection) và giữ handle kết quả, thay vì tra cứu lại mỗi request.

Gotcha: hãy đo thay vì đoán. Overhead của metrics thường đủ nhỏ để bỏ qua
và thỉnh thoảng chiếm 5% CPU — và bạn không thể biết cái nào đúng nếu
không có một flamegraph ([`08-observability/04-profiling.md`](04-profiling.md)).

### Push vs pull, và OpenTelemetry đứng ở đâu
Prometheus pull (scrape `/metrics` theo chu kỳ); metrics của OTel có thể
push tới một collector, sau đó collector export sang Prometheus/Datadog/v.v.
Một proxy thường expose một endpoint `/metrics` để scrape — đơn giản,
không thêm phụ thuộc mạng, và nó sống sót khi proxy tạm thời không kết nối
được từ collector (dữ liệu chỉ bị bỏ lỡ, không bị queue rồi mất).

Gotcha: expose `/metrics` trên một **listener riêng** tách khỏi traffic
production, bind vào một interface nội bộ. Trên listener chính, nó có thể
truy cập được bởi bất kỳ ai, và nó làm rò rỉ một bản đồ chi tiết về các
upstream, tên route, và khối lượng traffic của bạn — chưa kể việc scrape
nó trở thành một cách rẻ tiền để tiêu tốn CPU của proxy
([`07-security/09-ddos.md`](../07-security/09-ddos.md)). Nó cũng có nghĩa là metrics vẫn scrape được
khi listener chính đang bão hòa hoặc đang shed tải, đúng lúc bạn cần chúng
nhất.

Gotcha: render `/metrics` có độ phức tạp O(số lượng series). Ở cardinality
cao, đây trở thành một response chậm, tốn nhiều allocation — và một chu kỳ
scrape ngắn hơn thời gian render nghĩa là proxy luôn bận rộn serialize
metrics. Hãy theo dõi thời gian scrape như một metric riêng của nó.

## Practice
Xây dựng theo thứ tự sau.

1. Trong [`labs/15-prometheus`](../../labs/15-prometheus), expose `/metrics` bằng `TextEncoder` của
   crate `prometheus` và tự tay scrape nó. **Xong khi** `curl` trả về một
   response đúng định dạng exposition.
2. Thêm metrics RED phía client vào [`proxy`](../../proxy) trên một listener nội bộ
   riêng. **Xong khi** `/metrics` không truy cập được từ listener công
   khai và vẫn được phục vụ khi listener chính đang bão hòa.
3. Thêm RED theo từng upstream, khóa theo *tên* upstream, cộng với các tín
   hiệu đặc thù của proxy (độ bão hòa pool, tỉ lệ retry, số lượng healthy,
   tỉ lệ cache hit, số lượng bị shed). **Xong khi** bạn có thể trả lời
   "latency này là của mình hay của upstream" chỉ từ dashboard.
4. Chọn bucket histogram cho khoảng latency thực tế của bạn với một ranh
   giới ngay ngưỡng SLO. **Xong khi** p50/p99 tính từ bucket khớp với một
   phép đo trực tiếp (so với latency ghi lại bởi load generator của bạn)
   trong phạm vi một độ rộng bucket — với bucket mặc định trước, để bạn
   thấy chúng lệch nhau.
5. Aggregate đúng cách qua nhiều instance. **Xong khi** chạy ba instance
   proxy và tính p99 của cả fleet bằng `histogram_quantile(sum by (le)
   ...)` cho ra một con số bảo vệ được — và bạn cũng đã tính trung bình
   của các p99 từng instance và có thể nói nó lệch bao xa.
6. Cố tình thêm một label raw-path, chạy load test, và quan sát số lượng
   series tăng lên. **Xong khi** bạn đã thấy nó leo lên hàng nghìn, rồi sửa
   nó bằng route templating và xác nhận nó bị giới hạn bởi kích thước
   route table.
7. Pre-resolve các handle label trên hot path và profile
   ([`08-observability/04-profiling.md`](04-profiling.md)). **Xong khi** bạn có thể nói chi
   phí CPU của metrics dưới dạng phần trăm, trước và sau.
8. Theo dõi thời gian render `/metrics` như một metric riêng của nó.
   **Xong khi** bạn biết một lần scrape mất bao lâu ở số lượng series hiện
   tại và điều đó so với chu kỳ scrape của bạn thế nào.
