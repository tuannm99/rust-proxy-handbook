# Circuit Breaker

Ngừng gọi một upstream đang fail liên tục, thay vì retry vào nó mãi mãi.
[`06-proxy/05-retry.md`](05-retry.md) xử lý các lỗi request đơn lẻ; đây là lớp phía trên,
quyết định một upstream không nên được gọi chút nào trong một khoảng thời
gian.

## What to learn
### Ba trạng thái
```rust
enum CircuitState {
    Closed,                                 // normal, calls pass through
    Open { until: std::time::Instant },     // failing fast, no calls made
    HalfOpen,                               // one trial call allowed
}
```
Chuyển trạng thái: `Closed -> Open` sau khi điều kiện failure trip;
`Open -> HalfOpen` khi `until` trôi qua; `HalfOpen -> Closed` khi một lệnh
gọi thử nghiệm thành công, `HalfOpen -> Open` (với `until` được reset,
thường backoff xa hơn nữa) khi nó fail.

Giá trị nằm ở thứ `Open` mang lại: request fail *ngay lập tức* thay vì sau
một connect timeout. Với client đó là khác biệt giữa một lỗi nhanh nó có
thể hành động theo, và một cú treo nhiều giây; với upstream đó là khác
biệt giữa được để yên hồi phục và bị giữ down bởi traffic tiếp diễn.

### Trip theo failure rate, không phải theo số đếm liên tiếp
Một trigger fail-liên-tiếp là đơn giản nhất và nó ồn ào ở traffic thấp:
một upstream nhận 3 request mỗi phút trip ở 3 lần fail xui xẻo dàn trải
trong một phút, còn một upstream nhận 10,000 mỗi giây có thể không bao giờ
thấy 5 lần fail liên tiếp ngay cả ở tỷ lệ lỗi 30%, vì các lần thành công
cứ xen kẽ vào.

Dùng một *tỷ lệ* failure trên một cửa sổ trượt với một sàn **số request
tối thiểu**: "chỉ mở nếu ≥20 request trong cửa sổ và >50% fail." Cái sàn
này là guard giống hệt xuất hiện trong rate-based alerting
([`08-observability/06-alerting.md`](../08-observability/06-alerting.md)) và canary analysis
([`09-architecture/06-canary-deploy.md`](../09-architecture/06-canary-deploy.md)) — dưới nó, phán quyết đúng là
"chưa đủ dữ liệu", không phải "healthy" cũng không phải "failing".

Gotcha: quyết định cái gì tính là failure, và không bao gồm lỗi của
client. Một 404 hay 400 nghĩa là upstream hoạt động đúng và request mới
là cái sai ([`06-proxy/03-healthcheck.md`](03-healthcheck.md) nêu cùng điểm cho passive health
check). Tính 4xx cho phép một client với một URL scheme hỏng mở circuit
cho tất cả mọi người.

Gotcha: tính *timeout* là failure, và đảm bảo timeout ngắn hơn cửa sổ bạn
đang đo — nếu không failure tới quá chậm để bao giờ hình thành một tỷ lệ.

### Chỉ một lệnh gọi thử nghiệm trong HalfOpen
Khi `until` trôi qua, mọi request in-flight đều muốn là lệnh gọi thử
nghiệm. Để tất cả chúng đi qua sẽ gửi một thundering herd vào một upstream
mà, theo định nghĩa, vừa mới fail.

Gate nó bằng một `compare_exchange` trên state hoặc một semaphore kích
thước một, để đúng một request probe và phần còn lại tiếp tục fail fast
cho tới khi nó báo cáo lại.

```rust
// only the thread that wins the CAS gets to be the trial call
if state.compare_exchange(OPEN, HALF_OPEN, AcqRel, Acquire).is_ok() {
    // this request probes
} else {
    // everyone else still fails fast
}
```

Gotcha: một lệnh gọi thử nghiệm duy nhất là một thí nghiệm một mẫu. Một
upstream hỏng 50% có cơ hội ngang nhau để đóng circuit, tại điểm đó full
traffic quay lại và nó mở lại — một flap. Yêu cầu vài lần thành công liên
tiếp trước khi đóng, hoặc ramp traffic trở lại dần dần (slow start trong
[`06-proxy/04-outlier-detection.md`](04-outlier-detection.md)), là thứ biến một cú tung đồng xu thành
một phép đo.

### Backoff cho thời lượng open
Một thời gian open cố định 30 giây nghĩa là một upstream down một giờ bị
probe 120 lần, mỗi probe tốn latency của một request thật. Backoff thời
lượng open ở mỗi lần thử thất bại (30s, 60s, 120s, có trần), và reset nó
sau một lần đóng thành công — cùng hình dạng exponential-có-trần như
retry backoff ([`06-proxy/05-retry.md`](05-retry.md)).

### Scope: theo từng upstream, không theo từng pool
Một circuit breaker trên *pool* fail fast cho mọi thứ ngay khi một host cư
xử tồi, điều này vứt bỏ các host khỏe mạnh bạn đang có. Giữ circuit theo
từng instance upstream, và để load balancer
([`06-proxy/02-load-balancer.md`](02-load-balancer.md)) route tránh các circuit mở bằng cách coi
chúng như các host unhealthy.

Gotcha: nghĩa là lý luận panic-threshold cũng áp dụng ở đây — nếu circuit
của mọi upstream đều mở, fail fast cho 100% traffic có thể tệ hơn cứ thử
dù sao. Quyết định "tất cả circuit mở" thì làm gì, đúng như
[`06-proxy/03-healthcheck.md`](03-healthcheck.md) quyết định "tất cả host unhealthy" thì làm
gì.

Gotcha: circuit state là theo-từng-process. Với N instance proxy, một
upstream phải fail đủ để *mỗi* instance trip độc lập, và một instance vừa
restart bắt đầu với mọi circuit đóng và học lại bằng cách gửi traffic thật
vào một upstream đã biết là tồi
([`09-architecture/05-rolling-restart.md`](../09-architecture/05-rolling-restart.md)).

### Vị trí của nó so với retry
Retry trước, circuit sau: một retry xử lý sự cố thoáng qua đơn lẻ, và
circuit nhận ra rằng sự cố thoáng qua đã trở thành chuyện thường ngày.
Thứ tự quan trọng là **retry phải tôn trọng circuit** — retry vào một
circuit mở chính xác là loại traffic mà circuit tồn tại để chặn, nên việc
chọn upstream của retry phải bỏ qua các host có circuit mở thay vì coi
"circuit open" chỉ là một failure khác để retry vượt qua.

## Practice
Xây theo thứ tự.

1. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), cài đặt `CircuitState` theo từng
   upstream với một trigger fail-liên-tiếp. **Xong khi** log cho thấy
   `Closed -> Open -> HalfOpen -> Closed` với một upstream bạn kill rồi
   restart.
2. Chỉ ra cú trip sai ở traffic thấp. **Xong khi** một upstream nhận một
   request mỗi phút mở circuit ở ba lần fail rải rác — rồi chuyển sang
   một trigger dựa trên tỷ lệ với một số request tối thiểu và xác nhận nó
   không còn xảy ra.
3. Loại 4xx khỏi số đếm failure và bao gồm timeout. **Xong khi** 1000
   request tới một path không tồn tại để circuit đóng, và một upstream
   chấp nhận kết nối nhưng không bao giờ phản hồi mở nó.
4. Gate `HalfOpen` chỉ một lệnh gọi thử nghiệm. **Xong khi** một load test
   đồng thời tại thời điểm circuit half-open cho thấy đúng một request
   chạm tới upstream.
5. Yêu cầu vài lần thành công liên tiếp để đóng, và backoff thời lượng
   open trên các lần fail lặp lại. **Xong khi** một upstream hỏng 50% ổn
   định ở trạng thái open thay vì flap.
6. Làm cho retry nhận biết circuit. **Xong khi** một retry bỏ qua các
   host có circuit mở thay vì đếm chúng là một failure khác.
7. Quyết định và cài đặt chính sách tất-cả-circuit-mở. **Xong khi** kill
   mọi upstream tạo ra hành vi bạn đã chọn có chủ đích — fail fast, hoặc
   cứ thử dù sao — thay vì bất cứ điều gì rơi ra tình cờ.
