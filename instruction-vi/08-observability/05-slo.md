# SLI, SLO, và Error Budget

Định nghĩa "hoạt động tốt" nghĩa là gì, bằng con số, trước khi quyết định
cái gì nên đánh thức một con người. [`08-observability/06-alerting.md`](06-alerting.md) xây
trên nền này — một alert không có SLO đứng sau chỉ là một ngưỡng ai đó
đoán ra.

## What to learn
### Ba thuật ngữ
Một **SLI** (service level indicator) là một tỉ lệ được đo, ví dụ
`successful_requests / total_requests` hoặc tỉ lệ request dưới 300ms. Một
**SLO** là một target cho SLI đó trong một window, ví dụ "99.9% request
thành công trong 30 ngày." **Error budget** là `1 - SLO`: trong 30 ngày ở
mức 99.9%, bạn được phép ~43 phút downtime hoàn toàn (hoặc một lượng
tương đương các thất bại một phần rải rác) trước khi bạn tiêu hết toàn bộ
ngân sách.

Ngân sách này định hình lại độ tin cậy như một tài nguyên để tiêu một cách
có chủ đích thay vì một mục tiêu trừu tượng — đốt nó khi ứng phó một sự cố
thật, không phải vì alert quá mức trên nhiễu. Nó cũng làm rõ sự đánh đổi:
một team còn ngân sách có thể ship nhanh hơn; một team đã tiêu hết nên tập
trung sửa độ tin cậy thay vì thế.

### Viết nó dưới dạng good events trên valid events
Định nghĩa SLI một cách chính xác là phần lớn công việc, và định nghĩa đó
phải trụ vững qua một cuộc tranh cãi trong lúc xảy ra sự cố. Hãy chốt cả
hai nửa:

- **Lỗi 4xx có phải của bạn không?** Thường thì không — một client gửi
  request sai định dạng không nên đốt ngân sách của bạn. Nhưng một mã 429
  bạn phát ra vì bạn đang quá tải ([`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md))
  *chính là* thất bại của bạn đội lốt một status code lỗi-của-client, và
  thuộc về tử số.
- **Một thành công chậm có phải là thành công không?** Với một SLO
  latency, không — hãy định nghĩa "tốt" là "thành công *và* dưới ngưỡng,"
  với ngưỡng khớp một cạnh bucket histogram
  ([`08-observability/02-metrics.md`](02-metrics.md)) để nó là một con số đếm chính xác
  thay vì một phép nội suy.
- **Request nào là hợp lệ?** Health check, synthetic probe, và scrape
  `/metrics` nên bị loại trừ, nếu không một đêm yên tĩnh chỉ toàn health
  check sẽ báo cáo 100% availability.

Gotcha: hãy viết định nghĩa này thành một recording rule, không phải văn
xuôi trong một tài liệu. Câu query *chính là* định nghĩa; bất cứ thứ gì
khác sẽ trôi dần khỏi nó.

### Một proxy cần ít nhất hai SLI
"Proxy đã làm đúng nhiệm vụ của nó" (nó đã route, nó không tự trả 5xx do
lỗi của chính nó) và "request end-to-end đã thành công" (bao gồm cả thất
bại của upstream) là hai con số khác nhau với hai chủ sở hữu khác nhau.
Gộp chúng lại làm error budget trở nên vô dụng: team proxy đốt ngân sách
vì một lần deploy tệ của upstream mà không có gì để sửa.

Hãy đo cả hai. Page team proxy trên cái thứ nhất
([`08-observability/06-alerting.md`](06-alerting.md) bàn về việc route theo ai có thể hành
động). Cái thứ hai vẫn đáng để theo dõi — nó là những gì người dùng thực
sự trải nghiệm — nhưng chủ sở hữu của nó là service đứng sau bạn.

Gotcha: gán một thất bại cho đúng phía không phải lúc nào cũng rõ ràng.
Một mã 504 vì upstream vượt quá timeout của proxy có thể là vấn đề của
upstream hoặc một timeout bạn đặt quá gắt gao
([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)). Hãy quyết định quy tắc quy kết trước, và mã
hóa nó vào recording rule, nếu không mọi sự cố đều bắt đầu bằng cùng một
cuộc tranh cãi.

### Chọn target
Một target là một quyết định kinh doanh bị ràng buộc bởi vật lý, và cả hai
nửa đều quan trọng:
- **Cao hơn không tự động tốt hơn.** Mỗi số 9 thêm vào tốn công sức nhiều
  hơn khoảng một bậc độ lớn, và qua một điểm nào đó, thất bại chi phối là
  thứ bạn không kiểm soát được — mạng của client, DNS, internet.
- **Bạn không thể vượt quá các dependency của mình.** Một proxy đứng trước
  một upstream 99.9% không thể cung cấp 99.99% end-to-end, trừ khi nó có
  thể phục vụ mà không cần upstream đó (`stale-if-error` của
  [`05-http-stack/07-cache.md`](../05-http-stack/07-cache.md) chính là kiểu tách rời này).
- **Window quan trọng ngang với con số.** 99.9% trong 30 ngày là 43 phút;
  trong 7 ngày là 10 phút, và một lần deploy tệ duy nhất có thể tiêu hết
  toàn bộ ngân sách ngắn hơn đó.

Gotcha: đặt SLO *nội bộ* chặt hơn bất kỳ SLA nào đã hứa ra bên ngoài, để
bạn bắt đầu phản ứng trước khi bạn nợ ai đó bất cứ điều gì.

### Ngân sách là thứ điều khiển hành động
Hai tín hiệu dẫn xuất thực sự làm việc:
- **Ngân sách còn lại.** Một gauge ai cũng có thể nhìn vào. Khi nó khỏe
  mạnh, cứ ship; khi nó cạn kiệt, ưu tiên của team chuyển sang độ tin cậy
  cho tới khi nó hồi phục. Chính sách đó phải được thống nhất trước để có
  ý nghĩa.
- **Burn rate** — tốc độ bạn đang tiêu ngân sách so với tốc độ bền vững.
  Đây là thứ các alert dựa vào để bắn, vì nó bắt được cả "một sự cố ngay
  bây giờ" lẫn "một rò rỉ chậm sẽ tiêu hết cả tháng" bằng một cơ chế duy
  nhất ([`08-observability/06-alerting.md`](06-alerting.md)).

Gotcha: giữ target SLO trong *một* recording rule duy nhất mà mọi thứ
khác tham chiếu tới. Hard-code `0.001` trong năm alert rule nghĩa là thay
đổi SLO sẽ âm thầm để lại bốn cái trong số đó vẫn thực thi giá trị cũ.

## Practice
Xây dựng theo thứ tự sau.

1. Định nghĩa hai SLI cho [`proxy`](../../proxy) — thất bại do proxy gây ra và thất bại
   end-to-end — dưới dạng recording rule trên các counter từ
   [`08-observability/02-metrics.md`](02-metrics.md). **Xong khi** mỗi cái có xử lý tường
   minh cho 4xx, cho 429-do-quá-tải, và cho traffic health-check bị loại
   trừ, và bạn có thể nêu tên team sở hữu mỗi cái.
2. Viết quy tắc quy kết cho mã 504. **Xong khi** một timeout được gán cho
   một phía một cách có chủ đích và lý do nằm trong một comment ngay cạnh
   rule.
3. Thêm một SLI latency có ngưỡng khớp một cạnh bucket histogram. **Xong
   khi** "tỉ lệ dưới ngưỡng" là một tỉ lệ bucket chính xác thay vì một
   phép nội suy `histogram_quantile` — điều chỉnh bucket của bạn nếu
   chúng không khớp.
4. Chọn target và window, và tính ngân sách. **Xong khi** bạn có thể nói,
   bằng phút, mỗi SLO cho phép bao nhiêu downtime mỗi window, và đã kiểm
   tra tính hợp lý của nó so với độ tin cậy của chính upstream.
5. Publish ngân sách còn lại như một gauge. **Xong khi** một dashboard cho
   thấy còn lại bao nhiêu trong window hiện tại mà không cần ai chạy một
   câu query bằng tay.
6. Đặt target vào đúng một recording rule duy nhất. **Xong khi** thay đổi
   nó ở một chỗ đó di chuyển mọi tín hiệu dẫn xuất — kiểm chứng bằng cách
   thay đổi nó và xem tất cả di chuyển.
7. Kiểm chứng với thực tế. **Xong khi** bạn replay lại một sự cố trong quá
   khứ (hoặc tiêm một cái mới bằng [`12-testing/03-chaos.md`](../12-testing/03-chaos.md)) và xác nhận
   ngân sách bị tiêu khớp với thời lượng và mức độ nghiêm trọng thực tế
   của sự cố.
