# Load Testing

## What to learn
### Công cụ: wrk, vegeta, k6
`wrk` là một công cụ nhỏ viết bằng C, tốt cho throughput HTTP/1.1 thô với
scripting Lua để tự tạo request tùy biến. `vegeta` (Go) báo cáo latency
dưới dạng một histogram đúng nghĩa và dễ script từ CI hơn (`echo "GET
http://..." | vegeta attack -rate=500 | vegeta report`). `k6` nặng hơn
nhưng cho phép bạn script các luồng người dùng nhiều bước thực tế bằng JS.
Với handbook này, hãy bắt đầu với `wrk` hoặc `vegeta` nhắm vào
`labs/05-reverse-proxy` — bạn không cần các luồng đã script để tìm ra các
bottleneck cơ bản.

### Throughput so với percentile latency
Chỉ riêng requests/sec che giấu mất tail latency — một proxy có thể đạt
RPS cao trong khi p99 rất tệ vì một vài lệnh gọi upstream chậm xếp hàng
đằng sau các lệnh gọi nhanh. Luôn báo cáo p50/p90/p99/p999, không chỉ mean
hay RPS. Một bug load balancer (ví dụ một upstream nhận gấp 10 lần
traffic) thường hiện ra như một cái đuôi p99 béo từ rất lâu trước khi nó
hiện ra ở mức trung bình.

### Load generator không miễn phí
`wrk`/`vegeta` chạy trên cùng máy với proxy cạnh tranh CPU và có thể tự
trở thành bottleneck của chính nó — các con số khi đó đo generator, không
phải proxy của bạn. Hãy theo dõi CPU/network usage của chính generator, và
ưu tiên chạy nó trên một máy riêng (hoặc ít nhất một CPU affinity riêng)
để có các con số đáng tin. Một generator với quá ít connection/thread cũng
sẽ đẩy tải vào proxy không đủ mạnh và báo cáo latency "tốt" một cách giả
tạo.

### Tải closed-loop so với open-loop
Hầu hết các công cụ đơn giản (`wrk` mặc định) là closed-loop: chúng chờ
một response trước khi gửi request tiếp theo, nghĩa là một server chậm tự
động tiết chế tải được đưa ra và che giấu backlog thực sự. `vegeta attack
-rate=N` và `wrk2` là open-loop: chúng gửi ở một tốc độ cố định bất kể
thời gian phản hồi, đúng như traffic thật (và các sự cố thật) trông như
thế nào. Ưu tiên open-loop khi bạn muốn biết "chuyện gì xảy ra ở 500
req/s" thay vì "cái này có thể nhanh tới đâu end to end."

## Practice
1. Chạy `wrk -t4 -c100 -d30s` nhắm vào `labs/02-http-server` đang phục vụ
   một file tĩnh; ghi lại RPS và p50/p99.
2. Chạy cùng bài test đó nhắm vào `labs/05-reverse-proxy` với 2 upstream
   và so sánh — hop proxy nên cộng thêm latency, hãy định lượng bao nhiêu.
3. Chuyển sang `vegeta attack -rate=200` (open-loop) nhắm vào cùng target
   và so sánh p99 với lần chạy `wrk` closed-loop ở throughput tương tự.
4. Đưa vào một đường chậm nhân tạo ở một upstream (ví dụ `sleep` trước khi
   phản hồi 10% request) và xác nhận nó hiện ra ở p99 rất lâu trước khi nó
   làm dịch chuyển mean — đối chiếu `08-observability/02-metrics.md` để
   biết cách bạn sẽ alert việc này trong production.
5. Theo dõi `top`/`htop` trên chính load generator trong một lần chạy tốc
   độ cao và xác nhận nó không bị bão hòa CPU (điều này sẽ làm vô hiệu kết
   quả).
