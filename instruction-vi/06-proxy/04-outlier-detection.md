# Outlier Detection

Quyết định một upstream tồi từ traffic bạn đã đang gửi cho nó, thay vì từ
một probe chuyên dụng. [`06-proxy/03-healthcheck.md`](03-healthcheck.md) bao quát active
probing; file này bao quát nửa passive và phần damping giữ cho cả hai
không gây hại nhiều hơn chính bản thân failure.

## What to learn
### Passive health check
Đi nhờ trên traffic thật: nếu N request thật liên tiếp tới một upstream
fail hoặc timeout, đánh dấu nó down mà không chờ active probe tiếp theo.
Rẻ hơn (không traffic thêm) và phản ứng nhanh hơn nhiều — một active probe
interval 5s nghĩa là tới 5s request bị gửi vào một cái hố, trong khi
passive detection bắt được nó ở vài lần fail đầu tiên.

Hai loại này bổ sung cho nhau theo một cách cụ thể: passive check chỉ có
thể đánh dấu một upstream *down* (bạn không học được gì về một upstream
không nhận traffic), trong khi active check là thứ duy nhất có thể đưa nó
*up* trở lại. Một pool chỉ có passive checking là một cánh cửa một chiều —
một khi một upstream bị đánh dấu down nó ngừng nhận traffic để chứng minh
nó đã hồi phục.

Gotcha: đừng tính các lỗi do client gây ra là failure của upstream. Một
404 hay 400 nghĩa là upstream hoạt động đúng và *request* mới là cái sai;
tính 4xx vào ngưỡng failure cho phép một client với một URL scheme hỏng
đánh dấu toàn bộ hạm đội của bạn unhealthy. Chỉ tính connection error,
timeout, và 5xx — và hãy có chủ đích với 503, thứ thường có nghĩa là
"upstream này đang chủ động shed load"
([`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md)) chứ không phải "hỏng".

Gotcha: đây là cùng câu hỏi phân loại failure như điều kiện trip của
[`06-proxy/06-circuit-breaker.md`](06-circuit-breaker.md), và cả hai nên đồng thuận. Một upstream
có circuit mở nhưng vẫn được passive health checking coi là healthy sẽ
tạo ra các quyết định routing mâu thuẫn.

### Ngưỡng và flapping
Đừng bao giờ lật health state chỉ vì một probe fail — dùng một ngưỡng (ví
dụ 3 lần fail liên tiếp để down, 2 lần thành công liên tiếp để lên lại).
Nếu không có điều này, một upstream gần giới hạn capacity của nó sẽ flap
lên/xuống mỗi vài giây, điều tệ hơn cho hạm đội so với việc cứ giữ nó
down. "Flap damping" này chính xác là thứ `max_fails`/`fail_timeout` của
nginx và outlier detection của Envoy cài đặt.

Làm cho các ngưỡng bất đối xứng, và theo hướng có thể khiến bạn ngạc
nhiên: xuống chậm (3+ lần fail, để một sự cố thoáng qua không rút cạn một
upstream) nhưng lên lại *thậm chí chậm hơn* (nhiều lần thành công hơn,
cộng với một thời gian tối thiểu ở trạng thái down). Lên lại quá hăm hở là
thứ tạo ra flapping, vì upstream vừa hồi phục lập tức được trao một phần
đầy đủ của traffic và sập lại lần nữa.

Gotcha: các counter fail-liên-tiếp phải được reset nguyên tử cùng với
chuyển trạng thái, nếu không hai kết quả probe đồng thời có thể cùng quan
sát "2 lần fail" và cùng tăng lên 3, đếm trùng một lần fail duy nhất. Giữ
counter và state dưới một thao tác atomic duy nhất, hoặc một mutex nhỏ cho
mỗi upstream — cái này là theo-từng-upstream, ngoài hot path của request,
nên một mutex ở đây thì ổn.

### Ejection cũng cần một sàn
Passive detection ở quy mô lớn có thể loại bỏ nhanh hơn bạn định: một
dependency dùng chung trục trặc thoáng qua, mọi upstream fail vài request
cùng lúc, và các ngưỡng trip trên toàn bộ pool đồng thời. Đó cùng hình
dạng correlated-failure như một deep health check
([`06-proxy/03-healthcheck.md`](03-healthcheck.md)), đến từ một con đường khác.

Câu trả lời của Envoy là `max_ejection_percent` — không bao giờ eject
nhiều hơn một phần cấu hình của pool, bất kể tín hiệu nói gì. Dưới phần
đó tín hiệu đáng tin; trên đó, lời giải thích khả dĩ hơn là vấn đề không
nằm ở các host.

### Slow start: cuộc stampede hồi phục
Một upstream vừa trở lại healthy có 0 connection active, điều làm nó trở
thành ứng viên hấp dẫn nhất cho least-connection balancing
([`06-proxy/02-load-balancer.md`](02-load-balancer.md)) và cho bất kỳ consistent-hash ring nào
vừa thêm lại nó. Nó nhận một đợt bùng nổ traffic không cân xứng trong vài
giây đầu sau khi hồi phục — vào một process với cache nguội, connection
pool rỗng, và một JIT/page cache chưa được làm nóng — và thường xuyên sập
lại lần nữa, tạo ra một flap mà logic ngưỡng ở trên không thể ngăn vì
upstream thực sự đang fail.

Cách sửa là một ramp: trong `T` giây đầu sau khi một upstream trở nên
healthy, tăng dần weight hiệu dụng của nó từ gần-0 lên weight cấu hình,
để nó nhận một dòng slow-drip tăng dần thay vì một trận lũ. nginx
(`slow_start=`) và Envoy (`slow_start_config`) đều cài đặt chính xác điều
này.

Gotcha: cùng ramp đó áp dụng cho một upstream *mới được thêm* từ service
discovery ([`06-proxy/07-service-discovery.md`](07-service-discovery.md)), không chỉ một cái vừa hồi
phục — một instance vừa scale-out cũng nguội theo đúng cách y hệt, và
routing least-connection thấy nó hấp dẫn không kém.

### Tích hợp với load balancer
Load balancer không bao giờ được coi một upstream unhealthy là ứng viên,
nhưng việc đánh dấu-down phải là O(1) và lock-free từ phía đọc của
balancer — nó chỉ kiểm tra `upstream.healthy.load(Relaxed)` trước/trong
khi chọn.

Gotcha: một upstream đang slow start không phải "healthy" (đầy weight)
cũng không phải "unhealthy" (bị loại trừ) — nó cần một trạng thái thứ ba,
hoặc một giá trị weight mà balancer đọc thay vì một boolean. Model health
như một `bool` chính là thứ khiến việc thêm slow start sau này trở nên
gượng gạo; một enum nhỏ hoặc một giá trị effective-weight không tốn gì
ngay bây giờ.

## Practice
Xây theo thứ tự.

1. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), thêm passive health checking trên các
   lỗi request thật, chỉ đếm connection error, timeout, và 5xx. **Xong
   khi** một load test nhắm vào một upstream đã bị kill đánh dấu nó down
   trong dưới một active probe interval, và một test gửi 1000 request tới
   một path không tồn tại (toàn bộ là 404) để nó vẫn healthy.
2. Thêm ngưỡng fail-liên-tiếp/thành-công-liên-tiếp (3 xuống, 2 lên) với
   một thời gian tối thiểu ở trạng thái down, và log mọi chuyển trạng
   thái. **Xong khi** kill và restart một upstream cho thấy đúng hai lần
   chuyển trạng thái — không phải một loạt — và hồi phục mất ít nhất thời
   gian down tối thiểu của bạn.
3. Làm cho counter và chuyển trạng thái nguyên tử. **Xong khi** các kết
   quả probe đồng thời không thể đếm trùng một lần fail duy nhất — test
   với nhiều thread báo cáo fail đồng thời.
4. Thêm một tỷ lệ ejection tối đa. **Xong khi** một fault khiến mọi
   upstream fail đồng thời chỉ eject tối đa phần đã cấu hình của bạn, để
   phần còn lại tiếp tục phục vụ.
5. Model health như một effective weight thay vì một boolean, và thêm slow
   start. **Xong khi** bạn có thể vẽ biểu đồ request-mỗi-giây tới một
   upstream đang hồi phục trong 30s đầu và thấy một ramp thay vì một bước
   nhảy.
6. Xác minh flap đã biến mất. **Xong khi** flap hồi phục từ bước 2 biến
   mất dưới một load test đẩy upstream gần giới hạn capacity của nó.
7. Áp dụng ramp cho các upstream mới được phát hiện nữa
   ([`06-proxy/07-service-discovery.md`](07-service-discovery.md)). **Xong khi** thêm một instance
   giữa lúc load test cho nó một ramp thay vì một phần đầy đủ ngay lập
   tức.
