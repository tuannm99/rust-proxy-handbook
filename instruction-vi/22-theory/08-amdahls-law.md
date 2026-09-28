# Amdahl's Law và Parallel Speedup

## What to learn

### Amdahl's Law: phần tuần tự là trần
Nếu một phần `p` công việc của một chương trình có thể song song hóa và
`(1-p)` phải chạy tuần tự, speedup tối đa từ N processor là
`1 / ((1-p) + p/N)`. Khi N tăng không giới hạn, speedup tiến tới `1/(1-p)`
— một trần cứng do phần tuần tự quyết định hoàn toàn, bất kể ném bao
nhiêu core vào.

```text
speedup(N) = 1 / ((1-p) + p/N)
p = 0.95 (95% song song hóa được): speedup(N vô hạn) = 1/0.05 = 20x, không bao giờ hơn
p = 0.50: speedup(N vô hạn) = 1/0.50 = 2x — một nửa công việc tuần tự chặn bạn ở 2x mãi mãi
```
Đây là căn cứ hình thức cho một câu hỏi proxy rất cụ thể: "chúng ta tăng
gấp đôi `worker_threads`, sao throughput không tăng gấp đôi?" Nếu bất kỳ
phần nào đáng kể của request path bị tuần tự hóa — một `Mutex` toàn cục
trên routing table, một metrics aggregator đơn luồng, một lock connection-pool
chia sẻ bị contend nặng — phần đó đặt trần từ rất lâu trước khi hết core.

### Tìm phần tuần tự trong thực tế
Amdahl's Law chỉ hữu ích khi biết `p`, và trong một hệ thống thật điều đó
nghĩa là profiling ([`08-observability/04-profiling.md`](../08-observability/04-profiling.md)), không phải đoán
— đo thời gian wall-clock tốn bên trong bất kỳ phần đơn luồng hoặc bị
serialize bởi lock nào dưới load, chia cho tổng thời gian, đó là `(1-p)`.
Một proxy có một `Mutex<Vec<Upstream>>` duy nhất cho load-balancer state
bị chạm mỗi request có phần tuần tự tỷ lệ trực tiếp với lock đó bị contend
đến đâu — lời khuyên của [`03-rust/04-sync.md`](../03-rust/04-sync.md) dùng `RwLock` hoặc `watch`
thay vì `Mutex` ngây thơ, theo ngôn ngữ Amdahl, là một đòn trực tiếp vào
`(1-p)`.

### Gustafson's Law: cách đóng khung lạc quan hơn
Amdahl's Law giả định một kích thước bài toán *cố định* và hỏi nhanh hơn
được bao nhiêu; Gustafson's Law thay vào đó giả định một ngân sách thời
gian *cố định* và hỏi làm được *nhiều việc hơn* bao nhiêu khi N tăng —
theo khung này, speedup scale gần tuyến tính với N, vì phần tuần tự không
lớn theo kích thước bài toán như phần song song. Không luật nào "đúng
hơn" — chúng trả lời câu hỏi khác nhau. Amdahl's là lăng kính đúng cho
"làm request này nhanh hơn"; Gustafson's là lăng kính đúng cho "phục vụ
nhiều request đồng thời hơn với nhiều core hơn," chính là hình dạng thật
của bài toán scale của một proxy.

```text
Amdahl:    công việc cố định, thêm core -> ít thời gian hơn (speedup bị chặn)
Gustafson: thời gian cố định, thêm core -> làm được nhiều việc hơn (scale gần tuyến tính)
```

### Vì sao điều này ủng hộ mô hình work-stealing của tokio cho một proxy cụ thể
Workload của một proxy — nhiều request độc lập, mỗi cái chủ yếu I/O-bound
— có phần tuần tự vốn nhỏ *nếu* tránh được các điểm contention toàn cục,
chính xác là chế độ của Gustafson: thêm core cho phép phục vụ nhiều kết
nối đồng thời hơn, không làm bất kỳ request đơn lẻ nào nhanh hơn (latency
của một request bị chi phối bởi network RTT và thời gian upstream, không
phải parallelism CPU). Đây là lý do định lượng vì sao scheduler
work-stealing của [`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md) nhắm vào throughput dưới
concurrency thay vì song song hóa công việc của một request qua nhiều
core.

### Phần tuần tự thực sự ẩn ở đâu trong một proxy
Các thủ phạm kinh điển: một logger hoặc metrics aggregator đơn luồng mọi
request đều đi qua ([`08-observability/01-logging.md`](../08-observability/01-logging.md)), một counter
rate-limiter toàn cục ([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md) — giảm nhẹ bằng cách
shard counter, không phải bằng một lock nhanh hơn), một config snapshot
sau một `Mutex` thay vì `ArcSwap`/`watch` ([`09-architecture/03-config.md`](../09-architecture/03-config.md)),
và contention trên TLS session-cache dùng chung. Mỗi cái là một phần tuần
tự nhỏ riêng lẻ, nhưng các phần tuần tự cộng dồn: một phần tuần tự 2%
riêng lẻ đã chặn speedup ở 50x, còn ba phần 2% độc lập cho `(1-p) = 6%`
và chặn nó quanh 17x — tệ hơn nhiều so với bất kỳ cái nào khi đứng riêng.
Đo hiệu ứng kết hợp thay vì bỏ qua từng cái vì "không đáng kể".

## Practice
1. Profile [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) với `worker_threads` tăng dần (1, 2,
   4, 8) và vẽ throughput; fit đường cong theo công thức Amdahl để ước
   lượng phần tuần tự thật `(1-p)`.
2. Cố tình đưa một `Mutex` toàn cục vào hot path (bọc lookup upstream
   pool trong một lock chia sẻ dù `RwLock` là đủ), đo lại, và xác nhận
   trần throughput giảm theo cách khớp với Amdahl's Law.
3. Thay `Mutex` đó bằng `RwLock` hoặc một cấu trúc sharded/lock-free, đo
   lại, và định lượng đã phục hồi bao nhiêu phần trần.
4. Dùng `tokio::runtime::Handle::metrics()` ([`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md)),
   tìm một component của [`proxy`](../../proxy) (hoặc một crate [`labs/`](../../labs)) đang serialize
   mọi request qua một điểm, và ước lượng đóng góp của nó vào phần tuần
   tự tổng thể.
5. Viết một đoạn giải thích, theo ngôn ngữ Gustafson thay vì Amdahl, vì
   sao thêm core vào một host proxy được kỳ vọng nâng số kết nối đồng
   thời bền vững đáng tin cậy hơn là hạ latency của bất kỳ request đơn lẻ
   nào.
