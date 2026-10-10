# Alerting

Thứ thực sự page một con người, so với thứ chỉ là một dòng trên dashboard.
Các SLI và error budget mà mọi thứ ở đây dựa vào nằm trong
[`08-observability/05-slo.md`](05-slo.md).

## What to learn
### Cái này được xây trên nền tảng nào
Mọi alert dưới đây bắn dựa trên một **error budget burn rate**, thứ chỉ có
ý nghĩa khi bạn đã có một SLI và một SLO để suy ra nó — bao gồm cả các
quyết định về việc 4xx có tính không, một thành công chậm có phải là thành
công không, và request nào thậm chí là hợp lệ. Một proxy cũng cần hai SLI
riêng biệt, vì "proxy thất bại" và "upstream thất bại" có chủ sở hữu khác
nhau.

Xem [`08-observability/05-slo.md`](05-slo.md); file này giả định những thứ đó đã tồn
tại.

### Alert theo triệu chứng, không theo nguyên nhân
Page dựa trên những gì *người dùng* trải nghiệm (tỉ lệ lỗi tăng cao, p99
latency tăng cao, bản thân proxy sập) — không phải mọi điều kiện nội bộ
*có thể* gây ra một triệu chứng (một trong ba upstream không healthy, một
lần retry đã xảy ra, một lần GC pause 50ms). Nếu load balancer trong
[`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md) và health check trong
[`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md) đang làm đúng việc của chúng, mất một
upstream không nên page ai cả; mất tất cả thì nên. Các tín hiệu ở mức
nguyên nhân vẫn quan trọng — hãy giữ chúng như metrics/dashboard
([`08-observability/02-metrics.md`](02-metrics.md)) để tìm nguyên nhân gốc của một sự cố
sau khi alert ở mức triệu chứng đã wake up ai đó rồi.

### Các ngoại lệ: những thứ vô hình cho tới khi đã quá muộn
"Chỉ triệu chứng" có một nhóm ngoại lệ cụ thể và quan trọng — các điều
kiện không có triệu chứng *ngay bây giờ* nhưng chắc chắn sẽ có sau này.
Chúng đáng để có một alert mức ticket chính xác vì chờ triệu chứng nghĩa
là chờ sự cố xảy ra:
- **Certificate expire** ([`01-network/19-tls.md`](../01-network/19-tls.md),
  [`05-http-stack/12-vhost-routing.md`](../05-http-stack/12-vhost-routing.md)) — alert trước vài tuần, theo từng
  certificate. Triệu chứng là thất bại toàn bộ tại một thời điểm dự đoán
  được chính xác.
- **Config reload thất bại** ([`09-architecture/03-config.md`](../09-architecture/03-config.md)) — proxy vẫn
  chạy trên config cũ và trông hoàn toàn healthy trong khi lệch dần khỏi
  cái mà người vận hành nghĩ là đang được deploy.
- **Ổ đĩa đầy dần** vì log ([`08-observability/01-logging.md`](01-logging.md)), số fd tiến
  gần giới hạn, và độ bão hòa connection pool đang có xu hướng tăng.
- **Error budget burn rate**, là dạng tổng quát hóa của tất cả những cái
  trên: chưa hỏng bây giờ, nhưng đang trên đà hỏng.

### Alert theo burn-rate nhiều window
Một alert ngây thơ kiểu "tỉ lệ lỗi > 1% trong 5 phút" hoặc bắn trên những
đợt tăng nhỏ tạm thời (ồn) hoặc bỏ lỡ một rò rỉ chậm không bao giờ vượt
ngưỡng tức thời. Alert theo burn-rate thay vào đó hỏi "với tỉ lệ lỗi hiện
tại, chúng ta đang tiêu error budget nhanh cỡ nào?" và kiểm tra nó qua hai
window: một window ngắn (5 phút) với ngưỡng burn-rate cao bắt được các sự
cố nhanh, nghiêm trọng một cách nhanh chóng; một window dài (1 giờ hoặc 6
giờ) với ngưỡng thấp hơn bắt được một sự xuống cấp chậm, kéo dài trước khi
nó ăn hết ngân sách cả tháng. Yêu cầu *cả hai* window đồng ý trước khi
page — đây là thứ ngăn một alert một-window khỏi nhấp nháy liên tục.

```promql
# page nếu đang đốt error budget nhanh gấp >14 lần tốc độ bền vững, kéo dài
# qua cả window 5m lẫn 1h (một pattern multi-burn-rate hai window phổ biến)
(
  sum(rate(proxy_requests_total{status=~"5.."}[5m]))
  /
  sum(rate(proxy_requests_total[5m]))
) > (14 * 0.001)
and
(
  sum(rate(proxy_requests_total{status=~"5.."}[1h]))
  /
  sum(rate(proxy_requests_total[1h]))
) > (14 * 0.001)
```
Gotcha: mẫu số của tỉ lệ này có thể gần bằng 0 lúc traffic thấp (ví dụ 3
giờ sáng), tạo ra các dao động dữ dội chỉ từ một vài request — hãy bảo vệ
bằng một sàn số-lượng-request tối thiểu (`sum(rate(...[5m])) > N`) trước
khi đánh giá tỉ lệ này.

Trong thực tế bạn muốn một thang gồm nhiều rule như thế này thay vì một
rule duy nhất, để cái nhanh-và-nghiêm-trọng page ngay lập tức trong khi
cái chậm-và-nhẹ mở một ticket. Các cặp trong SRE workbook, cho một ngân
sách 30 ngày, đại khái là: **14.4x** burn trong 1h (với window ngắn 5m) →
page, tiêu 2% ngân sách; **6x** trong 6h (window ngắn 30m) → page; **1x**
trong 3 ngày (window ngắn 6h) → ticket. Window ngắn trong mỗi cặp tồn tại
để alert *tự hết* nhanh chóng khi vấn đề dừng lại, thay vì sáng đèn suốt
độ dài của window dài.

Gotcha: burn rate được tính dựa trên SLO của bạn, nên thay đổi SLO âm thầm
thay đổi mọi ngưỡng alert suy ra từ nó. Giữ target SLO trong một recording
rule duy nhất mà các alert tham chiếu tới, thay vì hard-code `0.001` ở năm
chỗ (như ví dụ trên làm, và như bạn không nên làm).

### Alert khi dữ liệu biến mất
Mọi alert ở trên bắn dựa trên các metric *do proxy export*. Nếu proxy chết,
treo, hoặc không thể tới được backend metrics, các metric đó ngừng đến —
và một alert cần dữ liệu để bắn sẽ im lặng nói rằng không có gì sai. Đây
là khoảng trống đáng xấu hổ nhất trong một hệ thống monitoring và nó dễ
lấp:
- **Alert khi dữ liệu thiếu** một cách tường minh (`absent()`, hoặc `up ==
  0` cho scrape target). Một proxy ngừng báo cáo là một sự cố cho tới khi
  chứng minh được điều ngược lại.
- **Probe từ bên ngoài** bằng một kiểm tra black-box/synthetic thực sự gửi
  một request qua proxy từ một mạng khác. Nó bắt được mọi thứ mà metrics
  nội bộ về bản chất không thể: process vẫn chạy nhưng listener bị kẹt,
  DNS cho hostname của bạn bị hỏng, một rule firewall thay đổi, certificate
  expire.

Gotcha: kiểm tra xem chính đường đi alert của bạn phụ thuộc vào cái gì.
Nếu thông báo alert đi qua hạ tầng nằm sau proxy này, một sự cố của proxy
sẽ block luôn cái alert về sự cố của proxy. Probe synthetic nên chạy ở một
nơi có đường đi độc lập ra ngoài.

### Mệt mỏi vì alert và routing
Mỗi alert cần một chủ sở hữu, một mức độ nghiêm trọng, và một link runbook
— một alert mà không ai có thể hành động lúc 3 giờ sáng chỉ huấn luyện
mọi người bỏ qua page. Route theo mức độ nghiêm trọng: page (wake up ai
đó, mức triệu chứng, đang đốt ngân sách), ticket (giờ hành chính, mức
nguyên nhân, chưa hiện ra với người dùng), và chỉ-metric (không thông báo,
chỉ để dashboard/hỗ trợ debug). Nếu một page bắn và phản ứng luôn là
"không có gì để làm, nó tự hết" quá vài lần, đó là tín hiệu để hạ nó xuống
một bậc, không phải lý do để tiếp tục bỏ qua nó.

Gotcha: route theo *ai có thể sửa nó*, không phải ai phát hiện ra nó. Một
proxy báo cáo đúng "mọi upstream cho service X đang trả về 500" nên page
chủ sở hữu của service X — page team proxy, những người chỉ có thể chuyển
tiếp thông báo, thêm một bước latency con người vào mọi sự cố. Đây là lý do
vì sao hai SLI ở trên cần là hai metric riêng biệt: chúng có lịch trực
on-call khác nhau.

## Practice
Xây dựng theo thứ tự sau.

1. Làm qua [`08-observability/05-slo.md`](05-slo.md) trước. **Xong khi** hai SLI và
   target của chúng tồn tại dưới dạng recording rule, và bạn có thể nói
   mỗi cái page team nào.
2. Viết thang burn-rate (14.4x/1h, 6x/6h, 1x/3ngày) với window ngắn tương
   ứng, tham chiếu target SLO từ một recording rule duy nhất. **Xong khi**
   thay đổi target ở một chỗ di chuyển mọi ngưỡng.
3. Thêm sàn số-lượng-request. **Xong khi** một mô phỏng 3 giờ sáng với 5
   request và 1 lỗi không page.
4. Thêm alert dữ liệu-thiếu và một probe synthetic bên ngoài. **Xong khi**
   kill hẳn [`proxy`](../../proxy) page trong khoảng thời gian mục tiêu của bạn — hãy
   test điều này, vì "alert bắn khi mọi thứ đã chết" là cái dễ bị hỏng
   nhất.
5. Kiểm chứng tính độc lập của đường đi alert. **Xong khi** bạn có thể nói
   việc gửi thông báo của bạn phụ thuộc vào cái gì, và xác nhận không cái
   nào đi qua proxy đang được giám sát.
6. Thêm alert mức ticket cho certificate expire, config reload thất bại, và
   độ bão hòa fd/pool. **Xong khi** một certificate còn 7 ngày là expire và
   một config reload bị cố tình làm hỏng mỗi cái tạo ra một ticket mà
   không page ai.
7. Chạy một chaos test ([`12-testing/03-chaos.md`](../12-testing/03-chaos.md)) dưới tải
   ([`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)). **Xong khi** page burn-rate bắn
   trong khoảng thời gian mong đợi, đúng SLI di chuyển (proxy hay
   end-to-end, tùy fault nào bạn tiêm vào), và mọi thứ hết cảnh báo sau
   khi fault được gỡ bỏ.
8. Viết runbook cho mỗi page: nó nghĩa là gì, ba bước kiểm tra đầu tiên,
   và cách giảm nhẹ. **Xong khi** link runbook nằm ngay trong định nghĩa
   alert và một người không quen hệ thống có thể làm theo nó.
