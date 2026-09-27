# Cache Stampede

Sự cố nơi một cache, đúng vào lúc nó ngừng giúp ích, chủ động làm mọi thứ
tệ hơn. [`05-http-stack/07-cache.md`](07-cache.md) bao quát semantics caching HTTP; file
này bao quát bài toán concurrency bên dưới bất kỳ cache nào, và vì sao
single-flight là một yêu cầu chứ không phải một tối ưu.

## What to learn
### Hình dạng của sự cố
Một entry phổ biến hết hạn. Trong mili giây tiếp theo, 1.000 request đồng
thời đều thấy một miss, và cả 1.000 cùng đi tới origin — chính điều mà
cache tồn tại để ngăn chặn, xảy ra vào thời điểm tệ nhất có thể.

Nó tự khuếch đại: origin chậm lại dưới tải đó, nên lần fetch mất lâu hơn,
nên cửa sổ miss mở lâu hơn, nên nhiều request chồng vào hơn. Key càng phổ
biến, spike càng tệ — nên các entry mà cache của bạn bảo vệ tốt nhất lại
là những cái gây đau nhất khi hết hạn.

Gotcha: đây không chỉ là vấn đề hết hạn. Một cold start
([`09-architecture/05-rolling-restart.md`](../09-architecture/05-rolling-restart.md) — mỗi lần restart làm rỗng một
cache trong memory), một lần purge, hay một lần eviction dưới áp lực
memory đều tạo ra cùng điều kiện miss-đồng-thời, cho mọi key cùng lúc
thay vì một.

### Single-flight: một lần fetch cho mỗi key
Cách sửa là request coalescing: request đầu tiên cho một key trở thành
cái đi fetch; mọi request khác chờ kết quả của nó.

```rust
// one in-flight fetch per key; the rest await the same shared future
enum Entry {
    Ready(CachedResponse),
    InFlight(tokio::sync::broadcast::Sender<CachedResponse>),
}
```

Việc check-rồi-insert phải atomic với các request khác cho cùng key —
hai request cùng thấy "không có entry" và cùng bắt đầu một lần fetch đã
đánh bại cơ chế này. Với một map đã shard (`dashmap`), điều đó nghĩa là
giữ guard của shard xuyên suốt quá trình chuyển từ absent sang
`InFlight`, không phải check rồi mới insert.

nginx gọi cái này là `proxy_cache_lock`; Go có `singleflight`; pattern
này đã cũ và việc thiếu nó là một sự cố lặp đi lặp lại.

Gotcha: bên chờ cần một **timeout**. Một lần fetch origin bị treo không
được để hàng nghìn request chờ vô thời hạn — mỗi bên chờ nên tự bỏ cuộc
theo deadline riêng (request timeout của [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)) và tự
quyết định fail hay thử tự fetch.

Gotcha: khi fetch **thất bại**, mọi bên chờ phải được đánh thức kèm lỗi.
Một leader trả về sớm — một panic, một nhánh chưa xử lý, một future bị
drop do client hủy — để lại các bên chờ bị chặn trên một kết quả sẽ không
bao giờ đến. Cấu trúc leader sao cho việc thông báo xảy ra trong một
`Drop` guard, cùng kỷ luật với connection counter của
[`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md).

Gotcha: map in-flight được key theo dữ liệu attacker có thể ảnh hưởng,
nên nó cần cùng giới hạn như bất kỳ map nào tương tự
([`13-algorithms/count-min-sketch.md`](../13-algorithms/count-min-sketch.md)).

### Serve stale trong khi refresh
Single-flight giảm N lần fetch xuống một, nhưng các bên chờ vẫn phải chờ.
Với `stale-while-revalidate` ([`05-http-stack/07-cache.md`](07-cache.md)) việc chờ biến
mất hoàn toàn: serve ngay bản stale cho tất cả mọi người, refresh một lần
ở nền, swap nó vào khi xong.

Đây là tổ hợp tốt hơn hẳn, và nó thay đổi kiểu sự cố từ "1.000 request
chờ origin" thành "1.000 request nhận một câu trả lời hơi cũ." Chỉ các
request cho một key *hoàn toàn không có* giá trị cache nào — thực sự
nguội, không chỉ stale — mới phải chờ.

Gotcha: một lần refresh nền mà không ai chờ vẫn cần một timeout, một
đường xử lý lỗi, và một giới hạn số lượng chạy đồng thời. Nếu không, một
lần origin sập để lại một đống task refresh ngày càng lớn, mỗi cái giữ
một connection ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)), tất cả đều vô ích.

### Dàn trải hết hạn để không đồng bộ hóa
Các key được điền cùng lúc sẽ hết hạn cùng lúc. Điền một cache từ cold
start và mọi thứ bạn nạp trong giây đầu tiên hết hạn trong cùng giây đó,
một giờ sau — một stampede tự gây ra, lặp lại với chu kỳ bằng TTL của
bạn.

Thêm jitter vào TTL lúc lưu (±10% là đủ) để việc hết hạn dàn trải ra trên
một cửa sổ thay vì rơi đồng loạt. Đây là cùng vấn đề đồng bộ hóa như chu
kỳ probe ([`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md)) và retry storm
([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)), với cùng cách sửa.

### Negative caching
Một origin trả về 404 hoặc 500 cho một key nóng, không được cache, nhận
mọi request cho nó mãi mãi — một stampede mà không cần cả hết hạn để kích
hoạt. Cache cả response âm tính, trong thời gian ngắn (vài giây), để một
miss nóng chỉ tốn một request origin mỗi khoảng thay vì tất cả.

Gotcha: giữ TTL âm ngắn và tách biệt với TTL dương. Một 404 được cache 5
phút cho một resource vừa được tạo là một bug người dùng thấy được, và sự
bất đối xứng này là cố ý: sai về việc-vắng-mặt rẻ để sửa hơn sai về nội
dung.

## Practice
Làm theo thứ tự này.

1. Trong [`labs/10-cache`](../../labs/10-cache), tái tạo stampede: cache một response origin cố
   tình chậm, làm nó hết hạn, và bắn 500 request đồng thời. **Xong khi**
   bạn có thể chỉ ra khoảng 500 lần chạm origin bằng một counter phía
   origin.
2. Thêm single-flight coalescing với một phép check-and-insert atomic.
   **Xong khi** cùng test đó tạo ra đúng một lần chạm origin và cả 500
   client nhận một response đúng.
3. Thêm timeout cho bên chờ và lan truyền lỗi. **Xong khi** một origin bị
   treo mãi mãi để các bên chờ fail theo deadline riêng thay vì bị chặn
   vô thời hạn, và một origin trả lỗi đánh thức mọi bên chờ kèm lỗi đó.
4. Giết leader giữa lúc fetch (hủy future của nó, mô phỏng client ngắt
   kết nối). **Xong khi** các bên chờ vẫn nhận được một kết quả hoặc một
   lỗi sạch, thay vì bị treo — viết nó trước khi có `Drop` guard và xem
   chúng bị treo.
5. Thêm `stale-while-revalidate` lên trên. **Xong khi** cùng test 500
   request đó trả về ngay từ cache stale với một lần refresh nền, và chỉ
   một key thực sự nguội mới khiến ai đó phải chờ.
6. Giới hạn số lần refresh nền đồng thời. **Xong khi** một lần origin sập
   tạo ra một số lượng task refresh in-flight có giới hạn thay vì một
   đống ngày càng lớn.
7. Thêm jitter cho TTL. **Xong khi** một cache được điền trong một đợt
   cho thấy việc hết hạn dàn trải trên một cửa sổ thay vì một đợt spike —
   vẽ biểu đồ tốc độ request origin qua một chu kỳ TTL đầy đủ để thấy nó.
8. Thêm negative caching ngắn. **Xong khi** một 404 nóng chỉ tốn một
   request origin mỗi khoảng negative-TTL, và một resource được tạo ngay
   sau một 404 đã cache trở nên thấy được trong vài giây.
