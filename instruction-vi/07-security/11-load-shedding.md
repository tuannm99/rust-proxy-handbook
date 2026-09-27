# Load Shedding

Phải làm gì khi công việc đến vượt capacity. Câu trả lời ngắn là "reject
một phần ngay lập tức" — và lý do vì sao xếp hàng thay vào đó lại tệ hơn
mới là điều file này nói tới.

## What to learn
### Xếp hàng là cái bẫy
Khi demand vượt capacity, hai lựa chọn là xếp hàng phần dư hoặc reject nó.
Xếp hàng cảm giác tử tế hơn và lại là cái thất bại.

Hàng đợi lớn dần, nên latency lớn dần theo. Đến lúc một request trong hàng
tới lượt, client đã timeout rồi và — tệ hơn — đã retry
(`06-proxy/05-retry.md`), thứ vừa thêm *nhiều hơn* load. Giờ bạn đang tốn
capacity còn lại để tính ra câu trả lời cho những request chẳng ai còn
nghe nữa, làm giảm capacity hiệu dụng, làm hàng đợi dài thêm. Vòng lặp đó
tự nuôi sống nó: throughput của công việc *hữu ích* sụp về gần 0 trong khi
hệ thống vẫn bận rộn hoàn toàn.

Tính chất hồi phục cũng quan trọng. Một hệ thống shedding trở lại bình
thường ngay khi load giảm. Một hệ thống xếp hàng phải xử lý hết một backlog
các request đã chết từ trước, nên nó vẫn suy giảm rất lâu sau khi nguyên
nhân đã hết.

### Shed sớm, shed rẻ
Một request bị reject nên tốn càng ít càng tốt — đó là toàn bộ điểm mấu
chốt (sự bất cân xứng chi phí của `07-security/09-ddos.md`). Vậy shedding
nên nằm sớm trong pipeline (`09-architecture/01-components.md`): trước
auth, trước WAF inspect body, trước lời gọi upstream.

```rust
// ở đầu pipeline, trước bất kỳ stage đắt đỏ nào
if inflight.load(Ordering::Relaxed) > shed_threshold {
    return Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::RETRY_AFTER, "1")
        .body(Body::empty());
}
```

Gotcha: một 503 tốn một lượt tra database, một dòng log có cấu trúc đầy
đủ với context request, và một trang lỗi được render cho attacker một tỷ
lệ chi phí *tốt hơn* so với được serve. Giữ đường shed không allocate ở
những chỗ có thể, và sample logging (`08-observability/01-logging.md`)
thay vì ghi một dòng cho mỗi lần reject.

Gotcha: đảm bảo logic retry của chính bạn không retry các response bị
shed. Retry một 503 nghĩa là "tôi đang overload" chính là kiểu khuếch đại
mà shedding tồn tại để ngăn.

### Chặn theo thời gian, không theo số lượng
Một độ sâu hàng đợi cố định là giới hạn sai, vì độ sâu đúng phụ thuộc bạn
đang drain nó nhanh thế nào — 100 request xếp hàng thì ổn ở 1ms mỗi cái và
thảm họa ở 500ms mỗi cái.

Quy tắc tốt hơn, từ CoDel, là chặn theo *sojourn time*: drop các request đã
chờ lâu hơn một target. Cái này tự thích ứng khi service time thay đổi, và
nó biểu đạt trực tiếp điều bạn thực sự quan tâm — một request đã chờ 5
giây là vô giá trị bất kể nó đứng đầu hay đứng thứ một trăm trong hàng.

Gotcha: đo thời gian chờ từ lúc request *đến*, không phải từ lúc bạn bắt
đầu xử lý nó. Toàn bộ điểm mấu chốt là nhận ra thời gian đã chờ, và một
timer bắt đầu lúc dequeue không thấy được gì trong đó.

### Shed đúng request
Khi đã chấp nhận là phải drop cái gì đó, chọn cái nào là một quyết định
thiết kế:
- **Theo priority.** Health check (`06-proxy/03-healthcheck.md`) và các
  đường quan trọng sống sót; traffic bulk hoặc batch bị drop trước. Điều
  này cần một priority tồn tại trên request, nghĩa là phải classify nó ở
  edge — theo route, theo tier client, theo một header rõ ràng từ caller
  đáng tin.
- **Theo chi phí.** Các route đắt đỏ shed sớm hơn, để một endpoint tốn
  kém không thể tiêu hết capacity của mọi thứ khác (phần endpoint đắt đỏ
  của `07-security/09-ddos.md`). Giới hạn concurrency theo route là dạng
  đơn giản nhất của điều này.
- **Ngẫu nhiên.** Mặc định, và ổn khi bạn không có tín hiệu nào tốt hơn —
  nhưng nghĩa là traffic quan trọng nhất của bạn bị drop cùng tỷ lệ với
  mọi thứ khác.

Gotcha: không bao giờ shed health check. Một proxy shed chính các probe
xác định nó có healthy hay không sẽ bị load balancer của chính nó đánh
dấu down và loại khỏi rotation — điều này dồn traffic của nó lên các
instance còn lại, đẩy chúng vào shedding luôn. Đó là cách shedding, nếu
scope tệ, kéo sập cả một fleet.

### Giới hạn thích ứng thắng giới hạn tĩnh
Một concurrency limit hard-code là một phép đoán già đi tệ: nó sai sau
một thay đổi hardware, một dependency chậm lại, hoặc một thay đổi code
làm đổi chi phí mỗi request.

Adaptive concurrency limiting (`concurrency-limits` của Netflix, adaptive
concurrency filter của Envoy) coi nó như một vấn đề congestion-control —
quan sát latency, tăng limit khi latency vẫn phẳng, giảm nó khi latency
tăng, về cơ bản là AIMD của TCP áp vào việc admission request. Limit theo
sát capacity thật mà không ai phải tune nó.

Gotcha: adaptive limiting cần một tín hiệu latency ổn định để hoạt động,
nên nó xử sự tệ khi latency vốn bimodal một cách tự nhiên (cache hit ở 1ms,
miss ở 200ms — `05-http-stack/07-cache.md`). Áp nó theo route, hoặc theo
nhóm công việc có chi phí tương tự, thay vì toàn cục.

### Nói sự thật với client
`503` với `Retry-After` là response shed đúng: nó nói "không phải lúc này,
thử lại sau N giây" thay vì "cái này đã fail." Một client biết điều thì
back off thay vì retry ngay.

Gotcha: phân biệt 503 bị shed với 503 do upstream fail trong metrics của
bạn (`08-observability/02-metrics.md`). Chúng có nguyên nhân và cách sửa
hoàn toàn khác nhau, và một counter `status="503"` duy nhất che mất cái
nào đang xảy ra. Sự phân biệt này cũng quan trọng cho SLI của bạn — một
503 bạn phát ra vì bạn bị overload là lỗi của bạn, và thuộc về error budget
(`08-observability/06-alerting.md`).

## Practice
Làm theo thứ tự này.

1. Thêm một counter concurrency và một shed threshold tĩnh ở đầu pipeline
   của `proxy`, trả 503 với `Retry-After`. **Xong khi** load vượt
   threshold bị reject ngay thay vì xếp hàng.
2. Chứng minh thất bại của việc xếp hàng trước, để có baseline cho cách
   sửa. **Xong khi** bạn có thể chỉ ra, với một hàng đợi không giới hạn,
   p99 latency tăng vô hạn và throughput hữu ích giảm trong khi process
   vẫn bận 100%.
3. Đo chi phí của đường shed. **Xong khi** một request bị reject đo được
   là rẻ hơn một request được serve — nếu không, tìm ra thứ gì đang
   allocate.
4. Thay giới hạn theo độ sâu bằng giới hạn theo sojourn-time đo từ lúc
   đến. **Xong khi** threshold thích ứng đúng qua hai workload có service
   time rất khác nhau, mà không cần tune lại.
5. Thêm giới hạn concurrency theo route. **Xong khi** làm bão hòa một
   route đắt đỏ để các route khác vẫn serve bình thường.
6. Miễn trừ health check và thêm các lớp priority. **Xong khi** proxy dưới
   shedding nặng vẫn trả lời được probe health của chính nó và vẫn cho qua
   traffic priority cao.
7. Ngừng retry các response bị shed. **Xong khi** một load test ở 3x
   capacity cho thấy số request upstream giữ nguyên thay vì nhân lên.
8. Tách 503 do shed khỏi 503 do upstream trong metrics, và gán đúng chúng
   vào SLI của bạn. **Xong khi** một dashboard phân biệt được "chúng ta
   đang overload" với "upstream đang hỏng."
9. (Stretch) Thay giới hạn tĩnh bằng giới hạn thích ứng và so sánh. **Xong
   khi** giới hạn thích ứng tìm ra một threshold gần capacity đo được của
   bạn mà không cần ai nói cho nó biết, và tìm lại được nó sau khi bạn
   inject một cú chậm 3x ở upstream.
