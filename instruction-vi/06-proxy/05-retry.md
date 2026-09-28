# Retry

Khôi phục một request thất bại đơn lẻ. Lớp phía trên — quyết định một
upstream nên ngừng nhận request hoàn toàn — là
[`06-proxy/06-circuit-breaker.md`](06-circuit-breaker.md).

## What to learn
### Idempotency: quy tắc đến trước mọi logic retry
Không bao giờ retry một request mà method/ngữ nghĩa của nó không an toàn
để lặp lại trừ khi bạn biết upstream idempotent với nó. GET/HEAD/PUT/DELETE
nói chung an toàn để retry; một POST trần thường thì không (nó có thể tạo
một resource hai lần). Một proxy production hoặc chỉ auto-retry các
method idempotent, hoặc yêu cầu một idempotency key tường minh từ client
cho các retry POST.

Gotcha: "idempotent theo RFC" và "idempotent ở upstream này" là hai khẳng
định khác nhau. `DELETE /orders/42` idempotent theo spec, nhưng nếu
upstream phát ra một webhook hoặc giảm inventory ở mỗi lần gọi, retry nó
vẫn có một side effect thấy được. Chính sách retry dựa trên method là một
mặc định hợp lý, không phải một bằng chứng — hãy làm nó override được
theo từng route ([`05-http-stack/04-router.md`](../05-http-stack/04-router.md)) để một team biết endpoint
của họ không an toàn có thể tắt nó đi.

### Ràng buộc đặc thù của proxy: bạn có thể không retry được chút nào
Một retry nghĩa là gửi lại request, nghĩa là vẫn còn giữ nó. Một proxy
streaming request body lên upstream đã tiêu thụ nó từ socket của client —
các byte đã mất, và không còn gì để replay lại. Vậy nên retry một request
có body đòi hỏi buffer body đó trước, và buffer thì có giới hạn: bạn không
thể giữ một upload 5 GB trong bộ nhớ chỉ để bảo toàn *tùy chọn* retry.

Điều này tạo ra một quy tắc cứng với một ngưỡng bên trong: buffer request
body tới N byte và giữ nó retry-được; quá N, stream và đánh dấu request
không-retry-được từ điểm đó trở đi. Envoy phơi bày chính xác điều này qua
`per_try_buffer_limit`/retry buffer limit.

Gotcha: điều tương tự áp dụng cho *response*. Một khi bạn đã gửi byte
response đầu tiên tới client, bạn không thể retry — client đã thấy một
status line và header. Mọi quyết định retry phải xảy ra trước khi response
head được forward, nghĩa là một upstream trả 200 rồi *mới* fail giữa
chừng body là không-retry-được bất kể method idempotent thế nào.

### Retry budget
Retry mù quáng trên mọi failure có thể biến một sự cố thoáng qua nhỏ của
upstream thành một retry storm khiến upstream sập hoàn toàn (mỗi request
fail giờ tốn gấp 2-3 lần). Một retry budget giới hạn tổng retry như một
phần trăm của tổng request trong một cửa sổ trượt (ví dụ "retry không được
vượt quá 10% request trong 10s vừa qua") — nếu budget cạn, ngừng retry và
fail fast thay vào đó.

Vì sao dùng budget theo phần trăm thay vì một cap số lần thử theo từng
request: một cap 3 theo từng request thì ổn khi 1% request fail, và thảm
họa khi 100% fail, vì cap đó là theo-từng-request và chính *load* mới là
thứ quan trọng với một upstream đang chật vật. Một budget được định nghĩa
theo tổng traffic, nên nó suy giảm về "không retry" đúng lúc retry gây
hại nhất. Đây là thứ cả linkerd và Envoy đều cài đặt, và nó là control
retry quan trọng nhất cần làm đúng.

Gotcha: kế toán budget phải theo-từng-upstream-pool, không phải toàn cục.
Một backend cư xử tồi làm cạn một budget toàn cục sẽ tắt retry cho mọi
route khác trong proxy, biến một failure cục bộ thành một suy giảm toàn
hạm đội.

### Retry khuếch đại qua các lớp
Retry nhân lên. Một client retry 3x đứng trước một proxy retry 3x đứng
trước một service mesh sidecar retry 3x nghĩa là một hành động của người
dùng có thể trở thành 27 request ở tầng đáy stack — và mỗi lớp đều nghĩ
mình đang khiêm tốn. Đây là một nguyên nhân lặp lại của các outage toàn
phần trong lúc suy giảm một phần, vì hệ số nhân traffic đạt đỉnh đúng lúc
hệ thống ít khả năng hấp thụ nó nhất.

Hai kỷ luật giữ nó bị chặn: **chỉ retry ở một lớp** (thường là lớp gần
failure nhất, với nhiều context nhất), và lan truyền trạng thái
"đã-retry-rồi" để các lớp downstream không thêm retry riêng của chúng —
cụ thể, một header mà proxy đặt và hop tiếp theo tôn trọng. Nếu bạn chỉ
kiểm soát lớp của riêng mình, ít nhất hãy biết các lớp trên và dưới bạn
được cấu hình làm gì, và ghi nó xuống cạnh config retry của bạn.

### Exponential backoff với jitter
Retry với delay cố định từ nhiều client đồng bộ hóa thành retry storm.
Exponential backoff với jitter ngẫu nhiên dàn trải các retry ra theo thời
gian.

```rust
fn backoff(attempt: u32, base: std::time::Duration) -> std::time::Duration {
    let exp = base * 2u32.pow(attempt.min(6));
    let jitter_ms = rand_range(0..exp.as_millis() as u64 / 2);
    exp + std::time::Duration::from_millis(jitter_ms)
}
```
Gotcha: giới hạn số mũ (như trên) — `2u32.pow(attempt)` tràn số rất nhanh
nếu `attempt` không bị giới hạn.

Hình dạng của jitter quan trọng hơn người ta nghĩ. Phiên bản ở trên
("equal jitter") giữ một nửa delay là tất định; **full jitter** —
`random(0, min(cap, base * 2^attempt))` — bỏ hoàn toàn nửa tất định và đo
được là tốt hơn hẳn trong việc giảm tranh chấp, theo phân tích công bố
của AWS. Trực giác: bất kỳ thành phần tất định nào cũng là một lịch trình
mà mọi client vẫn dùng chung, nên chỉ phần ngẫu nhiên mới thực sự dàn trải
tải.

Gotcha: backoff giữa các lần retry của *một* request cộng dồn trực tiếp
vào latency của request đó, và client có timeout riêng của nó. Ba lần
retry với exponential backoff dễ dàng vượt quá một client timeout 1s, tại
điểm đó mọi retry sau lần đầu tiên là load thuần túy không có cơ hội hữu
ích. Giới hạn tổng thời gian retry theo deadline còn lại của request, chứ
không chỉ theo số lần thử.

### Retry nên gửi tới upstream nào
Retry request lại vào chính upstream vừa fail thường là lãng phí — bất cứ
thứ gì làm nó hỏng khó mà được sửa trong vài micro giây sau đó. Retry vào
một upstream *khác* trong pool, và loại upstream đã fail khỏi tập ứng
viên cho request đó.

Gotcha: điều này xung đột với consistent hashing ([`02-load-balancer.md`](02-load-balancer.md))
khi affinity mang tính chất bắt buộc — một cache phía upstream hoặc một
session có trạng thái nghĩa là upstream "khác" đó là một cold miss hoặc
thẳng thừng là một lỗi. Khi affinity quan trọng, ưu tiên failover tới node
*kế tiếp* trên ring một cách tất định (để mọi client của key đó đồng
thuận về phương án dự phòng) hơn là chọn một upstream khác ngẫu nhiên.

### Khi nào retry không còn là câu trả lời
Một retry xử lý một sự cố thoáng qua đơn lẻ. Khi sự cố thoáng qua trở
thành chuyện thường ngày, retry vào một upstream đang fail chỉ là load nó
không thể hấp thụ — lớp phía trên là một circuit breaker, thứ ngừng hoàn
toàn việc gọi upstream đó trong một khoảng thời gian nguội và fail fast
thay vào đó.

Điều duy nhất cần làm đúng từ phía retry: **retry phải tôn trọng
circuit**, bỏ qua các host có circuit mở thay vì coi "circuit open" là
một failure khác để retry vượt qua. Xem [`06-proxy/06-circuit-breaker.md`](06-circuit-breaker.md).

### Hedged request: biến thể cho tail latency
Retry bắn khi failure. **Hedging** bắn khi *chậm*: nếu một request chưa
phản hồi trong, ví dụ, p95 latency của pool, gửi một bản sao thứ hai tới
một upstream khác và lấy cái nào trả lời trước, hủy cái thua. Vì trường
hợp xấu trong một hạm đội lớn thường là một host chậm chứ không phải một
host hỏng, hedging cắt giảm p99 latency đáng kể với một mức tăng nhỏ tổng
load — đây là kỹ thuật lõi từ "The Tail at Scale" của Google.

Gotcha: hedging thừa hưởng mọi ràng buộc ở trên *và* thêm một cái —
bản sao đang bay đồng thời, nên một hedge không idempotent còn tệ hơn hẳn
một retry không idempotent (cả hai bản sao đều có thể thành công). Chỉ
hedge các request idempotent, hedge ở một ngưỡng suy ra từ latency đo được
([`08-observability/02-metrics.md`](../08-observability/02-metrics.md)), và tính hedge vào retry budget — nếu
không một regression latency trên toàn hạm đội sẽ biến thành mọi request
được gửi hai lần, đúng lúc capacity đang thiếu.

## Practice
Xây theo thứ tự.

1. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), thêm một retry wrapper chỉ retry
   GET/HEAD, dùng hàm backoff ở trên giới hạn 3 lần thử, và gửi mỗi retry
   tới một upstream *khác*. **Xong khi** kill một upstream giữa lúc load
   test tạo ra zero lỗi thấy được ở client, và log cho thấy retry rơi vào
   các upstream còn lại.
2. Chuyển jitter sang full jitter và đo. **Xong khi** bạn có thể chỉ ra,
   từ timestamp của 1000 client retry đồng bộ, rằng các lần retry đến dàn
   trải khắp cửa sổ backoff thay vì dồn cục — so sánh với phiên bản
   equal-jitter trên cùng một test.
3. Giới hạn tổng thời gian retry theo một deadline mỗi request. **Xong
   khi** một request với 200ms budget còn lại ngừng retry thay vì chạy ba
   lần exponential backoff mà client đã bỏ cuộc từ lâu.
4. Thêm giới hạn buffer cho request body. **Xong khi** một POST nhỏ
   retry-được và một upload 100 MB stream qua mà không bị buffer — và cái
   lớn được đánh dấu đúng là không-retry-được thay vì fail hoặc OOM.
5. Thêm một retry budget theo phần trăm trượt, theo từng upstream pool.
   **Xong khi** ép 100% upstream fail khiến retry ngừng trong vòng một
   cửa sổ (thay vì nhân ba load), và traffic tới một pool khỏe mạnh khác
   vẫn retry bình thường.
6. Đi qua các bài tập của [`06-proxy/06-circuit-breaker.md`](06-circuit-breaker.md), rồi làm cho
   retry nhận biết circuit. **Xong khi** một retry bỏ qua một host có
   circuit mở thay vì tiêu tốn một lần thử vào nó.
7. (Stretch) Thêm hedging trên GET ở p95 đo được. **Xong khi** p99 latency
   dưới một load test với một upstream cố ý chậm giảm đo được, và tổng số
   request tới upstream chỉ tăng vài phần trăm — nếu nó tăng 50%, ngưỡng
   của bạn sai.
