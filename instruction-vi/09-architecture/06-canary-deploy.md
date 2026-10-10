# Canary & Blue-Green Deployment

## What to learn
### Blue-green vs canary — các đánh đổi risk/rollback khác nhau
**Blue-green**: hai môi trường đầy đủ (cũ "blue," mới "green"); chuyển
100% traffic cùng lúc (ví dụ lật xem upstream pool nào là "active").
Rollback tức thì (lật lại) nhưng bạn không có tín hiệu gì về hành vi thật
của phiên bản mới trên traffic thật trước khi nó live hoàn toàn.
**Canary**: chuyển một phần trăm nhỏ traffic (1%, 5%, 25%...) sang phiên
bản mới trong khi phần lớn traffic ở lại phiên bản cũ, theo dõi tỷ lệ lỗi/
latency trước khi tăng phần trăm. Chậm hơn, nhưng bắt được các bản release
tồi trong khi bán kính ảnh hưởng còn nhỏ.

Gotcha: "rollback tức thì" của blue-green chỉ đúng cho các thay đổi
*không trạng thái*. Nếu môi trường green đã ghi vào một database dùng
chung với một schema mới, lật lại để lại blue đọc dữ liệu nó không hiểu —
đó không phải một rollback, đó là một sự cố thứ hai. Cả hai chiến lược đều
đòi hỏi hai phiên bản có thể chạy **đồng thời trên state dùng chung**, đó
là một ràng buộc lên ứng dụng (thay đổi schema kiểu
expand-migrate-contract, định dạng message tương thích ngược), không phải
thứ proxy có thể cung cấp.

### Cài đặt weighted traffic splitting ở lớp proxy
Cái này xây trực tiếp trên [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md): thay vì một
upstream pool, router giữ hai pool (stable, canary) với một weight, và
chọn theo từng request bằng weighted random selection hoặc một hash tất
định (để cùng một client luôn rơi vào cùng một phiên bản — hữu ích cho
test nhạy cảm với session).

```rust
fn pick_pool(stable_weight: u32, canary_weight: u32) -> Pool {
    let roll = rand::random::<u32>() % (stable_weight + canary_weight);
    if roll < canary_weight { Pool::Canary } else { Pool::Stable }
}
```
Gotcha: weighted-random splitting nghĩa là các request của một client duy
nhất có thể nảy giữa stable và canary qua các lần request — ổn cho API
không trạng thái, hỏng cho bất cứ thứ gì phụ thuộc session, nơi bạn cần
sticky routing (hash trên một cookie/client-id) thay vào đó.

Gotcha: sticky-by-hash chỉ hoạt động nếu mọi instance proxy tính cùng một
hash. Một seed ngẫu nhiên theo-từng-process (cảnh báo `DefaultHasher` của
[`13-algorithms/hashmap.md`](../13-algorithms/hashmap.md)) nghĩa là instance A gửi một người dùng tới
canary và instance B gửi cùng người dùng đó tới stable — tạo ra chính
xác hiện tượng nảy-phiên-bản mà sự dính (stickiness) lẽ ra phải ngăn
chặn. Dùng một hash fixed-seed, và không bao gồm gì ngoài phép so sánh
ngưỡng trong các weight của pool, để một thay đổi weight di chuyển số
lượng người dùng tối thiểu.

Gotcha: một client nảy giữa các phiên bản tệ hơn vẻ ngoài của nó khi các
phiên bản khác nhau về hành vi — một trình duyệt load `index.html` từ
canary và bundle JS đã hash của nó từ stable nhận một 404 (tài sản bất
biến của [`05-http-stack/06-static.md`](../05-http-stack/06-static.md)), và người dùng thấy một trang hỏng
thay vì một lỗi sạch sẽ.

### Trigger rollback tự động
Một canary chỉ hữu ích nếu có gì đó đang theo dõi nó. So sánh tỷ lệ lỗi /
p99 latency của pool canary (từ [`08-observability/02-metrics.md`](../08-observability/02-metrics.md)) với
pool stable, trên cùng một cửa sổ thời gian, và tự động chuyển weight về
0% nếu tỷ lệ lỗi của canary vượt một ngưỡng (ví dụ gấp 2x stable) trong N
khoảng liên tiếp. Rollback chỉ-thủ-công tệ hơn hẳn — con người nhận ra một
regression canary chậm hơn một ngưỡng metric.

### Vấn đề thống kê không ai cảnh báo bạn
Một canary 1% nhận 1% traffic, và do đó 1% các *mẫu*. Ở 1000 req/s tổng
thể, canary thấy 10 req/s — nên trong một cửa sổ 1 phút nó có 600 request,
và một đợt 12 lỗi duy nhất là "tỷ lệ lỗi 2%, gấp đôi 1% của stable." Phép
so sánh đó là nhiễu, và một tự động hóa rollback tin vào nó sẽ rollback
các bản release khỏe mạnh gần như mãi mãi, huấn luyện mọi người bỏ qua nó.

Hai guard, cả hai đều bắt buộc:
- **Số mẫu tối thiểu** trước khi bất kỳ phép so sánh nào được đánh giá —
  cùng cái sàn như tỷ lệ alerting trong
  [`08-observability/06-alerting.md`](../08-observability/06-alerting.md) và circuit breaker dựa trên tỷ lệ
  trong [`06-proxy/05-retry.md`](../06-proxy/05-retry.md). Dưới nó, phán quyết đúng là "chưa đủ dữ
  liệu," không phải "healthy" và không phải "failing."
- **So sánh cùng loại với cùng loại.** Canary và stable phải được đo trên
  cùng cửa sổ, và lý tưởng là trên cùng hỗn hợp traffic — nếu canary của
  bạn tình cờ nhận một phần không cân xứng của một endpoint đắt (vì hash
  stickiness, hay vì nó ở một region), sự khác biệt bạn đo là traffic,
  không phải code.

Gotcha: p99 đặc biệt nhiễu ở số mẫu thấp — bách phân vị thứ 99 của 600
request là request tệ thứ 6. Ưu tiên tỷ lệ lỗi và p50 cho các canary sớm,
nhỏ; chờ khối lượng có ý nghĩa trước khi tin vào các so sánh tail latency.

### Những gì một canary không thể bắt được
Đáng biết để một canary xanh không bị nhầm là bằng chứng:
- **Rò rỉ tài nguyên.** Một memory leak ([`14-memory/06-fragmentation.md`](../14-memory/06-fragmentation.md))
  hoặc fd leak mất lâu hơn 100 lần để hiện ra ở 1% traffic. Một canary
  chạy một giờ không nói gì cho bạn về một leak giết một instance
  full-traffic trong một ngày.
- **Failure phụ thuộc load.** Lock contention, cạn kiệt connection pool
  ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)), và thundering herd chỉ xuất hiện gần
  capacity — thứ mà một canary 1% còn lâu mới chạm tới.
- **Bug phụ thuộc thời gian.** Một batch job hàng ngày, hết hạn
  certificate ([`01-network/19-tls.md`](../01-network/19-tls.md)), một phép tính ranh giới tháng.
- **Bất cứ thứ gì downstream.** Nếu canary chia sẻ upstream và một
  database với stable, nó không thể tiết lộ một vấn đề trong dependency
  dùng chung — và có thể *gây ra* một vấn đề gây hại cho cả traffic
  stable.

Các biện pháp giảm nhẹ là giữ ở một phần trăm có ý nghĩa (25-50%) trong
một khoảng thời gian có ý nghĩa trước khi lên 100%, và tiếp tục theo dõi
bản release sau khi nó đã rollout hoàn toàn — hầu hết các bản release
fail, fail sau khi lần deploy được công bố hoàn thành.

### Nơi điều này phụ thuộc vào service discovery
Nếu các instance upstream được đăng ký động
([`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md)), gắn thẻ mỗi instance với một
version/pool label lúc đăng ký để router có thể query "cho tôi các
instance stable khỏe mạnh" vs "cho tôi các instance canary khỏe mạnh"
thay vì hardcode địa chỉ.

Gotcha: pool canary nhỏ — thường là một instance duy nhất — nên logic
panic-threshold và health-check từ [`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md) hành xử
khác ở đó. Một instance unhealthy trong một pool stable 20-instance là
không đáng kể; một instance unhealthy trong một pool canary 1-instance là
100% của pool đó, và chế độ panic fail-open của bạn có thể route traffic
*stable* vào nó. Đánh giá health theo từng pool, không phải trên tập đã
gộp.

## Practice
Xây theo thứ tự.

1. Mở rộng [`labs/06-load-balancer`](../../labs/06-load-balancer) (hoặc [`proxy`](../../proxy)) để hỗ trợ hai pool có
   tên với weight cấu hình được, lấy từ config
   ([`09-architecture/03-config.md`](03-config.md)). **Xong khi** weight có thể thay đổi
   mà không cần restart.
2. Cài đặt weighted-random selection. **Xong khi** một test trên 100k
   request cho thấy sự chia tách trong phạm vi một phần trăm của weight
   đã cấu hình.
3. Thêm sticky-by-hash selection với một hash fixed-seed. **Xong khi**
   cùng một client ID rơi vào cùng một pool qua một lần restart process
   *và* qua ba instance proxy chạy đồng thời — test tường minh trường hợp
   nhiều instance, vì đó là chỗ bug seed ẩn náu.
4. Thêm RED metrics theo từng pool với pool như một label. **Xong khi** tỷ
   lệ lỗi và latency của stable và canary có thể so sánh trực tiếp trên
   một dashboard.
5. Cài đặt rollback tự động với một sàn số mẫu tối thiểu. **Xong khi** một
   canary ở weight 1% với 3 lỗi trong một phút *không* rollback, và một
   canary thực sự trả 50% lỗi thì có — trường hợp đầu là cái chứng minh
   guard hoạt động.
6. Mô phỏng một canary tồi dưới load ([`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md))
   với một upstream trả 500. **Xong khi** rollback tự động kích hoạt
   trong cửa sổ mục tiêu của bạn và tổng số lỗi thấy được ở client bị
   giới hạn bởi weight canary, không phải bởi thời gian một con người
   phản ứng.
7. Đánh giá health theo từng pool. **Xong khi** instance canary duy nhất
   trở nên unhealthy đưa pool canary về weight zero mà không kích hoạt
   panic-mode route traffic stable vào nó.
8. Ghi lại những gì quy trình canary của bạn không thể bắt được, và bạn
   làm gì thay vào đó. **Xong khi** runbook triển khai nêu tên một
   khoảng thời gian giữ ở một phần trăm có ý nghĩa và một khoảng theo
   dõi sau-rollout, kèm lý luận đi cùng.
