# Distributed Tracing
OpenTelemetry span/trace so với metrics và logs.

## What to learn
### Ba trụ cột, và mỗi cái thực sự dùng để làm gì
**Logs** trả lời "chuyện gì đã xảy ra" cho một sự kiện. **Metrics** trả
lời "xu hướng tổng hợp là gì" với chi phí rẻ ở quy mô lớn. **Traces** trả
lời "*request cụ thể này* đã tốn thời gian ở đâu qua mọi hop nó đi qua" —
một cây các span (proxy → auth service → upstream A → upstream B) kèm
timing cho từng cái. Một proxy thường là điểm vào của một trace, nên nó
chịu trách nhiệm hoặc bắt đầu một trace mới hoặc tiếp tục một trace client
đã bắt đầu sẵn.

### Propagate trace context (W3C `traceparent`)
Distributed tracing chỉ hoạt động nếu mọi hop chuyển tiếp cùng một trace
ID về phía trước. Đặc tả W3C Trace Context định nghĩa một header
`traceparent`: `00-<32 hex trace-id>-<16 hex parent-span-id>-<flags>`.
Nếu request đến đã có một cái, proxy tạo một span *con* dưới trace ID đó
và forward một `traceparent` mới (với span ID của chính nó làm parent)
tới upstream. Nếu không, nó tự sinh một trace ID mới.

```rust
// pseudo: trích xuất/propagate W3C traceparent
fn child_traceparent(incoming: Option<&str>, new_span_id: &str) -> String {
    match incoming {
        Some(tp) => {
            let parts: Vec<&str> = tp.split('-').collect(); // [version, trace_id, parent_id, flags]
            format!("00-{}-{}-{}", parts[1], new_span_id, parts[3])
        }
        None => format!("00-{}-{}-01", generate_trace_id(), new_span_id),
    }
}
```
Gotcha: nếu chỉ một hop duy nhất trong chuỗi làm rớt hoặc không forward
`traceparent`, trace vỡ vụn thành các mảnh rời rạc và bạn mất khả năng
thấy toàn bộ đường đi của request — đây là bug distributed tracing phổ
biến nhất trong thực tế.

Gotcha: đoạn code trên index `parts[1]` và `parts[3]` trên input do kẻ
tấn công cung cấp — một `traceparent` sai định dạng làm panic task của
request. Hãy validate nghiêm ngặt: đúng bốn field ngăn cách bởi dấu gạch
ngang, đúng độ dài, chỉ chứa hex, và một trace ID hay span ID toàn số 0 là
không hợp lệ theo spec. Từ chối-và-tự-sinh-mới là phản ứng đúng với bất cứ
thứ gì sai định dạng, không bao giờ là "cứ dùng nó thôi."

Gotcha: `tracestate` (header đi kèm mang dữ liệu key-value đặc thù theo
vendor) cũng phải được forward, và nó có giới hạn kích thước riêng. Bỏ nó
đi không làm vỡ cây trace nhưng làm mất các quyết định sampling và context
vendor mà các hệ thống downstream phụ thuộc vào.

### Bug Rust async phá vỡ tất cả: `enter()` xuyên qua `.await`
Đây là failure mode đặc thù của Rust, và nó âm thầm tạo ra các trace *sai*
thay vì thiếu — điều này còn tệ hơn.

```rust
// SAI trong code async
let span = tracing::info_span!("request", request_id = %id);
let _guard = span.enter();        // đặt current span cho THREAD NÀY
upstream.send(req).await;          // task nhường quyền; một task khác chạy tiếp trên thread này
                                   // ...và kế thừa span của chúng ta
```

`Span::enter()` trả về một guard đặt current span dạng thread-local. Khi
future nhường quyền tại một `.await`, guard vẫn còn được giữ, nên bất kỳ
task nào executor lên lịch tiếp theo trên thread đó sẽ log và span *bên
trong span request của bạn*. Bạn có các request lồng dưới các request
không liên quan, attribute gắn vào sai trace, và correlation ID trỏ vào
request của người khác.

Cách sửa là gắn span vào *future* thay vì vào thread:

```rust
use tracing::Instrument;
async move { upstream.send(req).await }
    .instrument(tracing::info_span!("request", request_id = %id))
    .await;
```

`#[tracing::instrument]` trên một async fn tự động làm đúng việc này. Quy
tắc: trong code async, không bao giờ giữ một guard `Entered` xuyên qua một
`.await`; `clippy::await_holding_span_guard` bắt được lỗi này, vậy hãy bật
nó lên.

### Nên đặt gì trên span của một proxy
Một span chỉ ghi lại tên và thời lượng cho bạn biết một request đã chậm,
điều bạn đã biết từ metrics rồi. Giá trị nằm ở các attribute giải thích
*vì sao*, và với một proxy, những cái thú vị đều liên quan tới các quyết
định nó đã đưa ra:

- upstream nào được chọn, và bằng thuật toán nào
  ([`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md))
- việc lấy connection mất bao lâu so với thời gian phản hồi thực của
  upstream ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)) — hai cái này thường bị nhầm lẫn
  trong các buổi review sự cố, và phép tách này giải quyết dứt điểm
- số lần retry và circuit có đang mở không ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md))
- cache hit/miss/stale ([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md))
- request có bị rate-limit hay bị shed không, và bởi rule nào
- thời gian dành cho kiểm tra WAF ([`07-security/06-waf.md`](../07-security/06-waf.md))

Các span con cho từng giai đoạn (handshake TLS, WAF, gọi upstream) làm cho
biểu đồ waterfall tự giải thích được — ai đó đọc trace nên thấy được thời
gian đi đâu mà không cần biết code của bạn.

Gotcha: attribute của span chịu cùng các quy tắc như field của log
([`08-observability/01-logging.md`](01-logging.md)) — không credential, không raw body,
không chuỗi do kẻ tấn công kiểm soát mà không giới hạn. Trace thường được
đọc rộng rãi hơn log, chứ không phải hẹp hơn.

### Sampling
Trace mọi request ở QPS cao thì đắt để lưu trữ và phần lớn là dư thừa
(hàng nghìn mã 200 nhanh giống hệt nhau không cho bạn biết thêm điều gì
mới). Head-based sampling quyết định ngay tại điểm vào (ví dụ 1% request,
hoặc mọi request đã có flag `traceparent` được đặt là sampled bởi một
client upstream). Tail-based sampling trì hoãn quyết định tới khi trace
kết thúc, nên bạn có thể giữ 100% các trace *chậm hoặc lỗi* và bỏ hầu hết
các trace nhanh — tín hiệu tốt hơn, nhưng đòi hỏi buffer span trước khi
quyết định.

Quyết định này phải được propagate, và phải nhất quán. Flag sampled ở byte
cuối của `traceparent` là cách các hop downstream biết được điều gì đã
được quyết định — một hop tự quyết định lại độc lập sẽ tạo ra các trace có
lỗ hổng, vì một số span của cùng một trace được giữ và một số bị bỏ. Hãy
tôn trọng flag đến; chỉ quyết định khi bạn là điểm vào.

Gotcha: tail sampling không thể chỉ làm ở proxy. Quyết định "giữ trace này
vì nó chậm" đòi hỏi thấy *toàn bộ* span của trace, điều mà chỉ một
collector đứng sau mọi service mới làm được. Phần việc của proxy là export
mọi thứ (hoặc một head sample rộng rãi) và để collector đưa ra quyết định
thật — một lựa chọn thiết kế có chi phí thực, nên hãy đưa ra nó một cách
có chủ đích.

Gotcha: một kẻ tấn công kiểm soát `traceparent` kiểm soát luôn quyết định
sampling của bạn, và có thể ép 100% sampling bằng cách đặt flag sampled
trên mọi request — biến pipeline tracing của bạn thành một mục tiêu
khuếch đại ([`07-security/09-ddos.md`](../07-security/09-ddos.md)). Rate-limit việc tôn trọng sampling
do client đặt từ các client không đáng tin, hoặc bỏ qua flag trừ khi đến
từ các peer đáng tin (lại chính ranh giới tin cậy của
[`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md)).

### Đừng để telemetry làm sập proxy
Exporter là một network client nằm trong cái bóng của request path, và nó
thất bại giống một cái như vậy. Ba tính chất cần kiểm chứng thay vì giả
định:
- **Việc export phải bất đồng bộ và có giới hạn.** Một batch exporter với
  một queue không giới hạn biến một sự cố collector thành OOM; một export
  đồng bộ biến latency của collector thành latency của request.
- **Việc drop span phải được đếm.** Một queue âm thầm bỏ đi khi bị áp lực
  cho bạn một pipeline trace đang âm thầm nói dối, thường đúng vào lúc sự
  cố bạn đang cố điều tra.
- **Shutdown phải flush.** Các span đang buffer khi process thoát sẽ mất
  trừ khi đường shutdown drain chúng — hãy nối việc này vào
  [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md) thay vì phó mặc cho một
  destructor có thể không bao giờ chạy.

### Span khác với `Span` của crate tracing
Gây nhầm lẫn một chút, crate `tracing` của Rust cũng gọi các scope
structured-logging của nó là "span" — và chúng kết hợp tốt với
OpenTelemetry: `tracing-opentelemetry` bắc cầu `tracing::Span` sang OTel
span để export tới một collector (Jaeger/Tempo/Honeycomb), nên chính
instrumentation bạn thêm cho [`08-observability/01-logging.md`](01-logging.md) cũng đóng
vai trò là trace data.

### Nối pipeline trong Rust
Năm crate phối hợp với nhau, và version của chúng phải khớp nhau chính
xác. `opentelemetry`, `opentelemetry_sdk` và `opentelemetry-otlp` phát hành
cùng nhau (0.33 tại thời điểm viết), còn `tracing-opentelemetry` có số
version riêng bám theo một bản OTel (0.34 đi cặp với OTel 0.33). Trộn các
bản phát hành sẽ sinh ra lỗi khó hiểu kiểu "expected `Tracer`, found
`Tracer`", vì tồn tại hai bản sao của cùng một trait. Kiểm tra bằng
`cargo tree -i opentelemetry` rằng chỉ có một version.

Mỗi mảnh làm gì, theo thứ tự luồng dữ liệu:
1. Code của bạn tạo span của `tracing` (`#[instrument]`, `info_span!` +
   `.instrument(...)`), y như phần còn lại của file này.
2. `tracing_opentelemetry::layer().with_tracer(tracer)` là một layer của
   `tracing_subscriber` biến các span đó thành OTel span. Thêm nó vào
   `tracing_subscriber::registry()` bên cạnh fmt layer của bạn.
3. Tracer đến từ một `opentelemetry_sdk::trace::SdkTracerProvider`, dựng
   bằng `.with_batch_exporter(exporter)` (gom span thành lô, ngoài đường xử
   lý request), `.with_sampler(...)` (ví dụ
   `Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(0.1)))`, chính
   là policy "tôn trọng quyết định đi vào, nếu không thì sample 10%" ở phần
   Sampling phía trên), và `.with_resource(...)` mang tên service
   (`Resource::builder().with_service_name("proxy")`). Lấy tracer từ nó qua
   `.tracer(...)` của trait `opentelemetry::trace::TracerProvider`.
4. Exporter là `opentelemetry_otlp::SpanExporter::builder()` với
   `.with_http()` (OTLP qua HTTP, port 4318, feature mặc định của crate)
   hoặc `.with_tonic()` (gRPC, port 4317, cần feature `grpc-tonic`).
5. Khi tắt, gọi `provider.shutdown()`. Batch exporter giữ các span chưa gửi
   trong memory, và một process thoát mà không flush sẽ mất vài giây trace
   cuối cùng, thường chính là những trace bạn cần.

**Propagation qua một hop** tách biệt với export. Đăng ký định dạng W3C một
lần bằng
`opentelemetry::global::set_text_map_propagator(opentelemetry_sdk::propagation::TraceContextPropagator::new())`.
Với request đi vào, trích context cha từ header
(`opentelemetry_http::HeaderExtractor(req.headers())` với `extract` của
propagator) và gắn nó vào span của request bằng
`OpenTelemetrySpanExt::set_parent`. Với request đi ra upstream, inject
context của span hiện tại (`span.context()`) vào header của request
upstream bằng `HeaderInjector`. Việc đó ghi ra header `traceparent` mà định
dạng của nó nằm ở đầu file này. Hai proxy instance nối nhau theo cách này
sẽ hiện ra thành một trace.

**Một viewer local.** Jaeger nhận OTLP trực tiếp:
`docker run --rm -d --name jaeger -p 16686:16686 -p 4317:4317 -p 4318:4318 jaegertracing/jaeger:latest`,
rồi mở `http://localhost:16686`. Trace xuất hiện vài giây sau request (do
gom lô). Nếu không thấy gì, kiểm tra port của exporter khớp với protocol
của nó (HTTP là 4318, gRPC là 4317) và `shutdown()` đã chạy.

## Practice
Xây dựng theo thứ tự sau.

1. Trong [`labs/16-opentelemetry`](../../labs/16-opentelemetry), ghép `tracing-opentelemetry` cùng một
   OTLP exporter phát ra một span tới một collector local (Jaeger là đủ).
   **Xong khi** span đó xuất hiện trong UI.
2. Cố tình tái tạo bug `enter()`-xuyên-`.await` trong [`proxy`](../../proxy): instrument
   một request handler với một guard được giữ và chạy các request đồng
   thời. **Xong khi** bạn thấy được các span lồng dưới sai parent — rồi
   chuyển sang `.instrument()` / `#[instrument]`, bật
   `clippy::await_holding_span_guard`, và xác nhận cây trace đúng.
3. Trích xuất và propagate `traceparent` với validate nghiêm ngặt. **Xong
   khi** một header sai định dạng (sai số field, không phải hex, trace ID
   toàn 0) bị từ chối và thay thế thay vì panic hay được propagate, và một
   cái hợp lệ tạo ra đúng quan hệ parent/child.
4. Chạy proxy trước hai instance nối tiếp của [`labs/02-http-server`](../../labs/02-http-server) và
   kiểm chứng end-to-end. **Xong khi** một trace ID duy nhất nối các span
   từ cả ba process trong UI.
5. Thêm attribute quyết định của proxy và các span con theo giai đoạn.
   **Xong khi** một trace duy nhất cho thấy thời gian lấy connection tách
   biệt với thời gian phản hồi upstream, cộng với tên upstream, số lần
   retry, và kết quả cache.
6. Thêm head-based sampling tôn trọng flag sampled đến. **Xong khi** quyết
   định propagate qua cả ba hop — xác nhận trace không bao giờ bị sample
   một phần — và một client không đáng tin không thể ép 100% sampling.
7. Cố tình phá vỡ việc propagate ở một hop. **Xong khi** bạn đã thấy trace
   bị vỡ vụn trong UI, vì đó chính là hình dạng của bug này trong
   production.
8. Kill collector giữa lúc load test. **Xong khi** proxy vẫn phục vụ với
   latency không đổi, bộ nhớ giữ nguyên, và một bộ đếm span-bị-drop tăng
   lên — rồi khởi động lại nó và xác nhận việc export tiếp tục.
9. Nối flush của exporter vào shutdown. **Xong khi** các span từ những
   request hoàn tất ngay trước `SIGTERM` vẫn tới được collector.
