# Observability

Phase 8. Trả lời câu hỏi "proxy đang làm gì ngay lúc này" và "vì sao
request đó chậm" — cho một component nằm giữa mọi thứ, nơi câu trả lời
thường gặp là "lỗi của phía bên kia" và bạn cần dữ liệu để chứng minh điều
đó.

## Files

- `01-logging.md` — structured event, redaction, log injection, sampling mà vẫn giữ được tín hiệu
- `02-metrics.md` — counter/gauge/histogram, RED cho một proxy, cardinality, vì sao percentile không thể lấy trung bình
- `03-tracing.md` — span, propagate `traceparent`, sampling, và bug `enter()` xuyên qua `await`
- `04-profiling.md` — perf và flamegraph, vì sao chúng nói dối về causality trong async, `tokio-console`, off-CPU time
- `05-slo.md` — định nghĩa SLI chính xác, target và window, error budget
- `06-alerting.md` — alert theo triệu chứng, thang burn-rate, alert khi dữ liệu biến mất, routing

## Thứ tự đọc

`01-logging.md` → `02-metrics.md` → `03-tracing.md` xây dựng ba loại tín
hiệu theo thứ tự chi phí tăng dần, và chúng dùng chung instrumentation
(span request bạn thêm cho logging trở thành trace span). Luôn đọc
`05-slo.md` trước `06-alerting.md` — một ngưỡng alert không có SLO đứng
sau chỉ là một con số đoán mò. `04-profiling.md` dành cho lúc thứ gì đó đã
chậm và bạn cần tìm ra chỗ nào.

Các file này hỗ trợ `labs/15-prometheus` và `labs/16-opentelemetry`. Điều
cần mang theo vào `proxy/`: đo latency ở cả hai đầu, vì phần chênh lệch
giữa thời gian client quan sát được và thời gian upstream quan sát được là
con số duy nhất nói cho bạn biết việc chậm lại là do bạn hay không.
