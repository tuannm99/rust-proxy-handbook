# Chaos Testing

## What to learn
### Tiêm fault: latency, mất gói, thất bại upstream
Chaos testing nghĩa là cố tình phá vỡ những thứ mà proxy của bạn phụ thuộc
vào trong khi nó đang chịu tải, thay vì chỉ test happy path. Ba loại fault
đáng bắt đầu: thêm latency (một upstream chậm có gây ra queueing không
giới hạn không?), mất gói/connection bị reset (phía client-facing có
xuống cấp một cách nhẹ nhàng không?), và thất bại toàn bộ của upstream
(proxy có thực sự failover không?). Những cái này ánh xạ trực tiếp tới các
failure mode mà [`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md) và [`06-proxy/05-retry.md`](../06-proxy/05-retry.md)
được thiết kế để xử lý — đây là nơi bạn tìm ra liệu code đó có thực sự
hoạt động không.

### toxiproxy
`toxiproxy` (Shopify) nằm giữa proxy của bạn và các upstream của nó như
một TCP proxy có thể lập trình: bạn có thể tiêm latency, giới hạn bandwidth, và reset connection trên một connection đang sống qua HTTP API của
nó, và bật/tắt chúng giữa lúc test. Đây là cách dễ nhất để test circuit
breaker của [`06-proxy/05-retry.md`](../06-proxy/05-retry.md) mà không cần đụng tới công cụ ở mức
kernel — trỏ config upstream của [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) vào một instance
toxiproxy thay vì upstream thật.

### tc netem
`tc qdisc add dev lo root netem delay 200ms loss 5%` tiêm latency/mất gói
ở mức network-interface của kernel — thực tế hơn toxiproxy (mất gói thật,
không chỉ reset connection) nhưng thô hơn (ảnh hưởng mọi traffic trên
interface đó, khó bật/tắt theo từng connection giữa lúc test). Tốt cho một
lượt kiểm tra cuối "cái này có sống sót qua một mạng tệ không" sau khi các
bài chaos test ở mức unit dựa trên toxiproxy đã pass.

### Kiểm chứng hành vi retry/circuit-breaker dưới fault thật
Điểm của chaos testing ở đây không phải là tìm bug mới một cách mù quáng —
mà là kiểm chứng một tuyên bố cụ thể: "circuit breaker mở ra sau N lần
thất bại và proxy dừng gửi traffic tới một upstream đã chết." Hãy viết
chaos test như một assertion nhắm vào tuyên bố đó (ví dụ "sau khi kill
upstream A, 95%+ request trong 2 giây được phục vụ bởi upstream B"), không
chỉ "chạy chaos và xem chuyện gì xảy ra."

## Practice
1. Đặt [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) trước hai upstream được route qua
   `toxiproxy`; tiêm 500ms latency vào một cái và xác nhận load
   balancer/health check của bạn nhận ra và chuyển traffic (hoặc ít nhất
   p99 phản ánh điều đó nếu bạn chưa xây adaptive routing).
2. Dùng toxiproxy để cắt hoàn toàn connection của một upstream giữa lúc
   load test và assert tỉ lệ lỗi client-quan-sát-được vẫn gần 0% (retry +
   circuit breaker hấp thụ nó) thay vì tăng vọt lên ~50%.
3. Dùng `tc netem` để thêm 5% mất gói trên `lo` và chạy lại cùng load
   test; so sánh tỉ lệ lỗi/latency với lần chỉ dùng toxiproxy.
4. Kill và khởi động lại một process upstream lặp đi lặp lại trong một
   load test kéo dài ("flapping") và xác nhận hysteresis của
   [`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md) ngăn proxy đảo quyết định rotation của nó
   mỗi giây.
5. Gửi `SIGTERM` cho [`proxy`](../../proxy) giữa lúc chaos test và xác nhận graceful
   shutdown ([`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md)) vẫn drain đúng các
   request đang xử lý dở ngay cả khi các upstream đang không healthy.
