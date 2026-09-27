# Logging

## What to learn
### Structured logging thay vì string logging
`println!`/`log::info!` dạng text thuần thì ổn cho một dự án nhỏ, nhưng một
proxy xử lý hàng nghìn req/s trên nhiều connection — bạn cần các field mà
máy đọc được (method, path, status, upstream, latency), không phải những
câu văn để grep. Dùng crate `tracing` với formatter JSON để mỗi dòng log là
một structured event có thể filter/aggregate ở downstream (Loki, ELK,
CloudWatch Insights).

```rust
use tracing_subscriber::fmt;

fn init_logging() {
    fmt()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .init();
}

// per-request:
tracing::info!(method = %req.method(), path = %req.uri().path(), status = 200, latency_ms = 4, "request completed");
```
Gotcha: layer JSON của `tracing` cấp phát bộ nhớ cho mỗi field trong mỗi
event — dưới tải thật, chi phí này trở nên đo được rõ ràng trên CPU. Hãy
sample hoặc batch log ở QPS cao thay vì log vô điều kiện mọi request.

### Log level và cái gì thuộc về đâu
`ERROR` = bản thân proxy thất bại trong việc làm nhiệm vụ của nó (upstream
không kết nối được, panic bị bắt lại, config không hợp lệ). `WARN` =
xuống cấp nhưng đã được xử lý (retry thành công ở lần thử thứ 2, circuit
breaker mở ra). `INFO` = một dòng cho mỗi request trong production, hoặc
cho mỗi sự kiện lifecycle đáng chú ý (config được reload, listener đã
bind). `DEBUG`/`TRACE` = dump header, nội tình connection pool — được
compile vào nhưng bị lọc bỏ mặc định qua `RUST_LOG`/`EnvFilter`, vì ngay cả
việc *đánh giá* có nên log hay không cũng có chi phí nếu argument không
lazy.
Gotcha: không bao giờ log toàn bộ body của request/response hay header
Authorization ở mức INFO — đó là cách các proxy làm rò rỉ credential vào
các hệ thống log aggregator mà một team rộng hơn có thể đọc được.

Gotcha: bài test hữu ích cho `ERROR` là "mình có muốn bị page vì cái này
không?" Một proxy log mọi lần kết nối upstream thất bại ở mức `ERROR` sẽ
tạo ra hàng nghìn dòng trong một lần restart upstream thông thường, và
level đó không còn mang thông tin gì nữa — nghĩa là lỗi thực sự mới lạ duy
nhất trở nên vô hình. Những thất bại mà proxy đã *xử lý được* (một retry
thành công, một circuit mở ra đúng như thiết kế) nhiều nhất chỉ nên là
`WARN`; tỉ lệ tổng hợp thuộc về metrics ([`08-observability/02-metrics.md`](02-metrics.md)),
không phải một dòng log cho mỗi lần xảy ra.

### Redact theo allowlist, không phải denylist
"Đừng log header `Authorization`" là một quy tắc bạn sẽ vô tình vi phạm,
vì secret xuất hiện ở những chỗ bạn không liệt kê hết: `Cookie`,
`Proxy-Authorization`, một token trong query string (`?api_key=...`), một
`X-Api-Key` tùy biến, một signed URL, một JWT nằm trong path segment.

Hãy đảo ngược lại: log một *allowlist* tường minh gồm các header và query
parameter, và bỏ qua tất cả những gì còn lại. Danh sách này ngắn
(`content-type`, `user-agent`, `content-length`, request ID của riêng
bạn), và failure mode trở thành "thiếu một field trong log" thay vì "một
credential nằm mãi trong log aggregator."

Gotcha: `#[derive(Debug)]` trên một struct config hay request sẽ vô tư
in ra một field bí mật khi ai đó log nó bằng `{:?}` lúc debug. Bọc secret
trong một newtype có `Debug`/`Display` in ra `[redacted]`, để mặc định của
compiler là an toàn thay vì rò rỉ. `Secret<T>` của crate `secrecy` làm
đúng việc này.

Gotcha: log chịu sự điều chỉnh của các quy định bảo vệ dữ liệu (GDPR và
tương đương) ngay khi chúng chứa một địa chỉ IP hay định danh người dùng.
Điều đó biến việc retention thành một quyết định tuân thủ, không chỉ là
chuyện chi phí, và "chúng ta giữ mọi log trong hai năm" có thể là một
gánh nặng pháp lý hơn là một tài sản.

### Log injection: các field là do kẻ tấn công kiểm soát
Path, `User-Agent`, `Referer`, và bất kỳ header nào bạn log đều do người
gửi request viết ra. Nếu formatter của bạn xuất ra plain text, một
`User-Agent` chứa `\n` cho phép kẻ tấn công tiêm hẳn *một dòng log giả* —
làm giả một entry trông giống một lần đăng nhập admin thành công, hoặc
tách một event thành hai để phá vỡ một parser.

Structured JSON logging xử lý đúng chuyện này ngay từ kiến trúc, vì
serializer escape các ký tự điều khiển bên trong giá trị chuỗi — đây là
một trong những lý do thuyết phục hơn để dùng JSON thay vì một định dạng
text tự viết. Nếu bạn vẫn xuất ra text, hãy escape hoặc loại bỏ ký tự
điều khiển khỏi mọi field do kẻ tấn công kiểm soát, và giới hạn độ dài
field (một `User-Agent` 1 MB là cách rẻ tiền để lấp đầy ổ đĩa của bạn).

Gotcha: điều này cũng áp dụng ở downstream. Một dòng log là JSON hợp lệ
vẫn có thể mang một payload tấn công bất cứ thứ gì *đọc* nó — một dashboard
render field log ra HTML có một lỗ hổng XSS được nạp bởi chính traffic đi
qua proxy của bạn.

### Correlation ID / request ID
Một reverse proxy thường là hop đầu tiên, nên nó nên tự sinh một request ID
nếu client không gửi (`X-Request-Id`), luồn nó qua mọi dòng log bằng một
`tracing::Span`, và forward nó tới upstream để log giữa các service có thể
join lại với nhau trên đó. Đây là tiền thân công nghệ thấp của distributed
tracing đầy đủ (xem [`08-observability/03-tracing.md`](03-tracing.md)).

```rust
let span = tracing::info_span!("request", request_id = %request_id);
let _enter = span.enter(); // mọi log bên trong kế thừa request_id
```

Gotcha: đoạn code trên đúng trong một hàm đồng bộ và **sai trong một hàm
async**. — giữ một guard `Entered` xuyên qua một `.await` gắn span đó vào
bất kỳ task nào executor chạy tiếp theo trên thread đó. Hãy dùng
`.instrument(span)` trên future thay vì thế;
[`08-observability/03-tracing.md`](03-tracing.md) giải thích chi tiết vì sao. Đây là bug
instrumentation phổ biến nhất trong async Rust và nó âm thầm phá hỏng
đúng cái correlation mà bạn xây request ID để có được.

Gotcha: một `X-Request-Id` do client cung cấp là input không đáng tin.
Validate độ dài và tập ký tự của nó (chẳng hạn dạng UUID) trước khi chấp
nhận, nếu không bạn đã trao cho kẻ tấn công một kênh vào mọi dòng log — và
vào bất kỳ hệ thống nào đánh index trên nó.

### Log volume là một chi phí, không phải một tác dụng phụ miễn phí
Ở 10k req/s, một dòng log cho mỗi request là 10k dòng/s cần ship, parse,
index, và lưu trữ. Disk I/O và backpressure của log shipper tự nó có thể
trở thành bottleneck làm chậm chính proxy (một sự cố kinh điển tự gây ra).
Hãy lên kế hoạch retention, sampling (ví dụ log 100% lỗi, 1% của mã 200),
và writer async/non-blocking (`non_blocking` của `tracing-appender`) ngay
từ ngày đầu.

Gotcha: ghi đồng bộ vào một file log là một syscall blocking trên request
path. Khi ổ đĩa chậm — hoặc chính log volume đã lấp đầy page cache bằng
các trang dirty (`16-kernel/09-page-cache.md`) — write đó chặn một tokio
worker thread và làm khựng mọi connection multiplex trên nó.
`tracing_appender::non_blocking` chuyển việc ghi sang một thread riêng
đứng sau một queue có giới hạn; queue đó là một ring buffer
([`13-algorithms/ring-buffer.md`](../13-algorithms/ring-buffer.md)), và bạn phải biết nó làm gì khi đầy.
Drop bớt dòng log khi bị áp lực là mặc định đúng cho một proxy — nhưng
chỉ khi bạn *đếm* số dòng bị drop, nếu không bạn sẽ tin vào một log
không đầy đủ mà không hề biết.

### Sampling mà vẫn giữ tín hiệu
Sampling ngây thơ ("log 1% request") tệ hơn vẻ ngoài của nó trong một hệ
thống phân tán: mỗi hop sample độc lập, nên một request được log ở proxy
có thể *không* được log ở upstream, và bạn không bao giờ tái dựng lại
được đường đi đầy đủ.

Hãy sample **một cách xác định (deterministic) dựa trên request ID hoặc
trace ID** — hash nó và giữ lại request nếu hash rơi dưới tỉ lệ của bạn —
để mọi hop đưa ra cùng một quyết định và một request đã được sample thì
được log ở mọi nơi. Sau đó xếp thêm các quy tắc thực sự quan trọng: giữ
100% mã 5xx, 100% request chậm (vượt một ngưỡng latency nào đó), và một
phần nhỏ xác định của phần còn lại.

## Practice
Xây dựng theo thứ tự sau.

1. Trong [`proxy`](../../proxy), nối `tracing` + `tracing-subscriber` với formatter JSON
   cấu hình được qua `RUST_LOG`. **Xong khi** một request phát ra một dòng
   JSON parse được chứa method, path, status, và latency.
2. Thêm một span cho mỗi request mang `request_id`, gắn bằng
   `.instrument()` thay vì `enter()`. **Xong khi** một load test với
   request đồng thời cho thấy mọi dòng log mang *đúng* request ID — hãy
   xây dựng phiên bản `enter()`-xuyên-`await` trước và quan sát các ID lẫn
   lộn nhau dưới điều kiện đồng thời.
3. Chấp nhận hoặc tự sinh `X-Request-Id`, validate giá trị do client cung
   cấp. **Xong khi** một `X-Request-Id` 10 KB hoặc chứa newline bị từ chối
   hoặc thay thế, và một giá trị hợp lệ được forward nguyên vẹn lên
   upstream.
4. Chuyển sang log field theo allowlist và bọc secret trong config bằng
   một newtype redact. **Xong khi** một request có `Authorization`,
   `Cookie`, và `?api_key=` tạo ra các dòng log không chứa bất kỳ cái nào
   trong số đó, và `{:?}` trên config in ra `[redacted]` cho các field bí
   mật.
5. Thử log injection: gửi một `User-Agent` chứa newline và JSON giả mạo.
   **Xong khi** output vẫn là đúng một bản ghi log hợp khuôn dạng cho mỗi
   request, với payload nằm gọn trong một string field.
6. Chuyển sang `tracing_appender::non_blocking` và đếm số dòng bị drop.
   **Xong khi** một load test nhắm vào một writer cố tình chậm cho thấy
   latency request không bị ảnh hưởng và bộ đếm drop khác 0, *nhìn thấy
   được*.
7. Đo chi phí của chính việc logging. **Xong khi** bạn có p50/p99 latency
   request với logging bật hoàn toàn, sample, và tắt, theo
   [`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md) — khoảng chênh lệch chính là ngân sách
   logging của bạn.
8. Thêm sampling xác định cùng các quy tắc luôn-log cho mã 5xx và request
   chậm. **Xong khi** cùng một request hoặc được log ở mọi hop hoặc không
   ở đâu cả, và mọi lỗi trong một load test đều xuất hiện đầy đủ.
