# Rate Limiting
Token Bucket, Leaky Bucket.

Bản thân các thuật toán (lazy refill, GCRA, bộ đếm lock-free, sliding
window) nằm trong [`13-algorithms/token-bucket.md`](../13-algorithms/token-bucket.md),
[`13-algorithms/sliding-window.md`](../13-algorithms/sliding-window.md), và [`13-algorithms/leaky-bucket.md`](../13-algorithms/leaky-bucket.md). File
này là tầng chính sách: đếm theo cái gì, làm gì khi chạm giới hạn, và
chuyện gì xảy ra khi chính bộ limiter thất bại.

## What to learn
### Token Bucket
Một bucket giữ tối đa `capacity` token, refill với tốc độ `rate`
token/giây; mỗi request tiêu thụ một token, và bị từ chối (hoặc trì hoãn)
nếu không còn token nào. Tự nhiên cho phép burst tới `capacity` trong khi
vẫn thực thi một tốc độ trung bình theo thời gian — đây là lý do nó là lựa
chọn phổ biến hơn cho rate limiting API (traffic client dồn cục là bình
thường và không nên bị phạt miễn là trung bình vẫn giữ vững).

```rust
struct TokenBucket {
    capacity: f64,
    tokens: f64,
    rate_per_sec: f64,
    last_refill: std::time::Instant,
}

impl TokenBucket {
    fn try_consume(&mut self, now: std::time::Instant) -> bool {
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        self.last_refill = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
```
Gotcha: refill theo kiểu lazy (như trên, tại mỗi lần kiểm tra) thay vì bằng
một timer nền cho mỗi client — một timer cho mỗi key bị rate-limit không
scale được tới hàng triệu client.

### Leaky Bucket
Mô hình hóa một hàng đợi "rò rỉ" ở một tốc độ không đổi; request được xếp
hàng và xử lý ở tốc độ cố định đó, hoặc bị drop nếu hàng đợi đầy. Khác với
token bucket, nó làm phẳng output thành một tốc độ nghiêm ngặt không đổi
thay vì cho phép burst — phù hợp khi hệ thống *downstream* thực sự không
chịu được burst (ví dụ bảo vệ một backend legacy có capacity cố định), kém
phù hợp hơn cho việc giới hạn "sử dụng API công bằng" nơi burst là bình
thường.

### Chọn key: quyết định thực sự quan trọng
Thuật toán là phần dễ. *Bạn đếm theo cái gì* mới quyết định limiter bảo vệ
bạn hay chỉ làm phiền người dùng của bạn.

**Source IP** là mặc định và có hai kiểu thất bại cụ thể. Các địa chỉ dùng
chung (CGNAT, NAT công ty, một trường đại học) đặt hàng ngàn người dùng
sau một key, nên một giới hạn thiết kế cho một người sẽ throttle tất cả họ
— cùng vấn đề shared-address như trong [`08-ip-filtering.md`](08-ip-filtering.md).

Và trường hợp IPv6 còn tệ hơn theo chiều ngược lại: một kẻ tấn công thường
được cấp một **allocation /64, tức 2^64 địa chỉ**. Key theo địa chỉ đầy đủ
128-bit nghĩa là mỗi request đơn lẻ có thể mang một key hoàn toàn mới, và
limiter theo IP của bạn không bao giờ kích hoạt dù chỉ một lần — trong khi
bucket map của bạn tăng trưởng vô hạn. Key IPv6 theo **prefix /64** (đôi
khi /56, tùy thuộc vào những gì khách hàng của bạn thực sự được cấp), không
phải toàn bộ địa chỉ. Một limiter đã chạy production nhiều năm có thể mang
lỗ hổng này mà không ai để ý cho tới khi một kẻ tấn công dùng nó.

**Authenticated identity** (API key, user ID từ claim đã xác thực —
[`01-auth.md`](01-auth.md)) tốt hơn hẳn khi có sẵn: nó ổn định, không bị chia sẻ, và là
thứ mà business rule của bạn thực sự được diễn đạt theo. Điểm bất tiện là
auth chạy sau limiter trong một số thiết kế, nên bạn cần cả hai — một
limiter rẻ dựa trên IP đứng trước để bảo vệ chính đường auth, và một
limiter thật theo từng identity đứng sau nó.

**Route** gần như luôn nên là một phần của key. Một giới hạn toàn cục cho
mỗi client nghĩa là một burst các request rẻ tiêu tốn hết ngân sách mà một
endpoint đắt đỏ duy nhất cần ([`09-ddos.md`](09-ddos.md)). Giới hạn `/search` riêng biệt
với `/health`.

Gotcha: bất kể bạn key theo cái gì, key đó đến từ dữ liệu do attacker kiểm
soát và index vào một map. Giới hạn nó, theo phần về key growth trong
[`13-algorithms/token-bucket.md`](../13-algorithms/token-bucket.md) — đây là cùng một vector cạn kiệt memory,
và lỗi /64 ở trên biến nó từ lý thuyết thành khai thác được một cách tầm
thường.

### Không phải request nào cũng có chi phí như nhau
Một model "1 request = 1 token" định giá một health check 2 KB giống hệt
một report ghim CPU 400ms. Cost-based limiting tính phí token tỷ lệ với chi
phí thực tế: một giá trị tĩnh theo route (rẻ và thường đủ dùng), hoặc token
bị trừ *sau khi* xong việc dựa trên thời gian upstream đo được hay số byte
response — cho phép request tiếp theo của một client bị throttle theo
những gì request trước đó thực sự tốn.

Gotcha: tính phí sau khi xong việc nghĩa là một client luôn có thể vượt
giới hạn đúng bằng một request đắt đỏ, vì chi phí không được biết cho tới
khi nó xong. Điều đó chấp nhận được cho việc *làm phẳng* chi phí và vô
dụng như một trần cứng — kết hợp nó với một giới hạn concurrency theo route
nếu một request đơn lẻ có thể gây hại cho bạn.

### Giới hạn theo client so với toàn cục
Giới hạn theo client (theo API key / theo IP) cần một map các bucket theo
key — để ý mức tăng trưởng memory (evict các entry rảnh) và dùng một map
sharded (`dashmap`) để tránh một lock toàn cục trở thành nút thắt cổ chai
dưới concurrency cao. Giới hạn toàn cục (bảo vệ toàn bộ hạ tầng bất kể
client nào) là một bucket/counter chia sẻ duy nhất và tương đối dễ.

Chạy cả hai. Giới hạn theo client thực thi sự công bằng; giới hạn toàn cục
mới là thứ thực sự bảo vệ upstream, vì "10.000 client mỗi người trong giới
hạn của họ" vẫn có thể vượt quá capacity. Giới hạn toàn cục nên được định
cỡ từ capacity đo được ([`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)), không phải chọn
như một con số tròn.

### Trả về gì khi từ chối
`429 Too Many Requests`, kèm `Retry-After` cho số giây tới khi có capacity.
GCRA ([`13-algorithms/token-bucket.md`](../13-algorithms/token-bucket.md)) cho ra con số đó chính xác; một
token bucket tính nó là `(1 - tokens) / rate`.

Ngoài ra, họ header `RateLimit-Limit` / `RateLimit-Remaining` /
`RateLimit-Reset` (bản draft của IETF mà GitHub, Stripe và những bên khác
đã triển khai dưới một dạng nào đó) cho phép các client biết điều chỉnh
*trước khi* bị từ chối — điều này hiệu quả hơn nhiều trong việc giảm tải so
với việc từ chối sau khi đã xảy ra, vì một client biết mình còn 3 request
sẽ tự chậm lại còn một client không biết sẽ liên tục dội bạn cho tới khi
nhận 429.

Gotcha: một 429 phải rẻ. Nếu việc từ chối tốn một database lookup, một
dòng log có cấu trúc đầy đủ context, và một trang lỗi được render, kẻ tấn
công có một tỷ lệ chi phí tốt hơn khi bị rate-limit so với khi được phục
vụ ([`09-ddos.md`](09-ddos.md)). Từ chối sớm trong pipeline, log với tỷ lệ mẫu thay vì
mỗi lần xảy ra.

Gotcha: đảm bảo client của chính bạn không retry 429 ngay lập tức. Một cơn
bão retry các request bị từ chối chính là loại tải mà giới hạn tồn tại để
ngăn chặn — xem [`06-proxy/05-retry.md`](../06-proxy/05-retry.md); `Retry-After` tồn tại là để được
tôn trọng.

### Rate limiting phân tán
Bucket trong bộ nhớ của một instance proxy đơn lẻ chỉ giới hạn traffic đi
qua *instance đó*. Với N replica proxy đứng sau một LB khác, một giới hạn
"100 req/s" cho mỗi client trở thành "100*N req/s" trừ khi các replica phối
hợp với nhau — hoặc thông qua một shared store (Redis với `INCR`+`EXPIRE`
nguyên tử, hoặc một script Lua/token-bucket), hoặc bằng cách chấp nhận sai
số và chia giới hạn cho N. Phối hợp trên mỗi request thêm một round trip
mạng cho mỗi request — thường được giảm nhẹ bằng batching/local caching với
đồng bộ định kỳ, đánh đổi độ chính xác nghiêm ngặt lấy latency.

Ba cách tiếp cận, nói thẳng về sự đánh đổi:
- **Chia cho N.** Không tốn chi phí phối hợp, và sai bất cứ khi nào traffic
  không phân bố đều — chính xác là trường hợp dưới consistent hashing hay
  khi kết nối của một client rơi vào cùng một replica. Một client đáng lẽ
  được 100 req/s chỉ nhận 20 vì họ tình cờ chạm vào một replica.
- **Shared store trên mỗi request.** Chính xác, và thêm một RTT mạng cộng
  một dependency cứng vào hot path của mọi request.
- **Bucket cục bộ với đồng bộ bất đồng bộ định kỳ.** Mỗi replica thực thi
  cục bộ và đối soát bộ đếm của nó với shared store mỗi vài trăm mili giây.
  Hơi thoáng trong cửa sổ đồng bộ, không tốn latency trên đường request, và
  đây là những gì hầu hết hệ thống production thực sự làm.

Gotcha: shared store giờ là một dependency có thể fail, và bạn phải quyết
định trước theo hướng nào. **Fail open** (cho phép khi Redis sập) giữ site
hoạt động và loại bỏ sự bảo vệ của bạn đúng vào lúc tệ nhất. **Fail closed**
(từ chối khi Redis sập) biến một outage cache thành một outage site. Câu
trả lời thường gặp là fail open *về limiter cục bộ* — tiếp tục thực thi
ngân sách cục bộ của mỗi replica, đây gần giống hành vi "chia cho N" — nên
một outage phối hợp làm giảm độ chính xác thay vì loại bỏ giới hạn hoặc
dịch vụ. Quyết định điều này một cách có chủ đích và test nó; phát hiện ra
nó trong một sự cố là cách "rate limiter bị sập" biến thành "site bị sập".

## Practice
Làm lần lượt theo thứ tự sau.

1. Trong [`labs/11-rate-limit`](../../labs/11-rate-limit), implement `TokenBucket` ở trên, key theo
   source IP với một `dashmap`. **Xong khi** một client vượt tốc độ bị từ
   chối và một client trong giới hạn thì không bao giờ.
2. Sửa key. **Xong khi** client IPv6 được key theo prefix /64 — chứng minh
   bằng một test gửi từ 10.000 địa chỉ riêng biệt trong một /64 và xác
   nhận chúng chia sẻ một bucket, sau khi trước tiên đã thấy phiên bản
   /128 để cho cả 10.000 lọt qua.
3. Thêm route vào key và giới hạn theo từng route từ config. **Xong khi**
   một burst trên `/health` không tiêu tốn ngân sách của `/search`.
4. Trả về đúng 429 kèm `Retry-After` và các header `RateLimit-*`. **Xong
   khi** `Retry-After` chính xác trong khoảng một giây so với thời điểm
   capacity thực sự quay lại — xác minh bằng cách sleep đúng khoảng đó và
   xác nhận request tiếp theo thành công.
5. Thêm eviction cho bucket rảnh và một map có giới hạn. **Xong khi** lấp
   đầy map với 1 triệu key tổng hợp làm memory phẳng ra thay vì tăng
   trưởng, và bucket của các client đang hoạt động không bao giờ bị evict
   giữa chừng.
6. Thêm một giới hạn toàn cục bên cạnh các giới hạn theo client, định cỡ từ
   một load test thật. **Xong khi** 10.000 client tuân thủ riêng lẻ bị giới
   hạn tập thể ở capacity upstream đo được của bạn.
7. Thêm tính phí theo cost cho một route đắt đỏ. **Xong khi** một client
   gọi endpoint đắt đỏ tiêu hết ngân sách của họ nhanh hơn tỷ lệ so với một
   client gọi endpoint rẻ.
8. Chạy hai instance và chứng minh sự nhân lên. **Xong khi** bạn có thể
   cho thấy một client nhận gấp 2 lần giới hạn của họ; rồi implement bucket
   cục bộ với đồng bộ định kỳ tới một shared store và cho thấy nó hội tụ
   gần 1x.
9. Kill shared store giữa lúc test. **Xong khi** proxy vẫn phục vụ traffic
   với thực thi cục bộ vẫn hoạt động — không phải vô hạn cũng không phải
   từ chối tất cả — và một metric ghi lại rằng nó đang chạy ở chế độ suy
   giảm.
