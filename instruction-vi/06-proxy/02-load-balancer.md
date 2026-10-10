# Algorithms
- Round Robin
- Least Connection
- Consistent Hash

Các biến thể sâu hơn (chi tiết smooth WRR, rendezvous hashing, Maglev) nằm
trong [`13-algorithms/`](../13-algorithms).

## What to learn

### Round Robin (+ weighted)
Thuật toán đơn giản nhất: xoay vòng qua các upstream theo thứ tự bằng một
atomic counter modulo kích thước pool. Weighted round robin (WRR) làm lệch
vòng xoay sao cho một upstream weight-3 được chọn nhiều gấp 3 lần một
upstream weight-1 — cài đặt qua "smooth WRR" (thuật toán của nginx) để các
lượt chọn được xen kẽ thay vì dồn cục (3,3,3,1 thay vì 1,1,1,3,3,3).

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

struct RoundRobin {
    next: AtomicUsize,
}

impl RoundRobin {
    fn pick<'a>(&self, upstreams: &'a [Upstream]) -> &'a Upstream {
        let i = self.next.fetch_add(1, Ordering::Relaxed) % upstreams.len();
        &upstreams[i]
    }
}
```
Gotcha: round robin thuần bỏ qua load — nếu một upstream chậm, nó vẫn nhận
một phần bằng nhau trong các request mới và queue của nó chất đống.

Gotcha: `fetch_add` trên một counter dùng chung là một cache line bị tranh
chấp trên mọi worker thread ở mọi request ([`17-performance/02-false-sharing.md`](../17-performance/02-false-sharing.md))
— ở request rate cao, một atomic duy nhất này trở nên đo được. Một counter
riêng cho mỗi worker, mỗi cái bắt đầu ở một offset khác nhau, cho cùng
phân phối mà không có traffic xuyên core nào cả, và là cách sửa tiêu chuẩn
khi round robin xuất hiện trong một profile.

Gotcha: `% upstreams.len()` trên một pool *đang thay đổi* âm thầm xáo trộn
mọi thứ khi độ dài thay đổi dù chỉ một ([`07-service-discovery.md`](07-service-discovery.md)) — ổn
với round robin không trạng thái, chết người nếu bất cứ thứ gì phía sau
giả định sự ổn định. Sự khác biệt đó chính xác là lý do consistent hashing
tồn tại.

### Least Connection
Chọn upstream có ít `active_conns` nhất ngay lúc này. Tốt hơn round robin
khi chi phí request không đồng đều (một số request rẻ, một số đắt) vì nó
phản ứng với load thực tế, không chỉ số lượng. Cần một counter
`active_conns` chính xác, chi phí thấp cho mỗi upstream (xem
[`01-upstream.md`](01-upstream.md)) — quét qua N upstream mỗi lượt chọn thì ổn với hàng chục
upstream, không ổn với hàng nghìn (dùng một heap,
[`13-algorithms/heap.md`](../13-algorithms/heap.md), nếu bạn cần scale xa hơn).

Gotcha: least-connection có thể gây thundering herd lên một upstream vừa
hồi phục (0 connection trông hấp dẫn nhất) — kết hợp với slow start / ramp-up
connection ([`03-healthcheck.md`](03-healthcheck.md)).

Gotcha, và cái này mang tính cấu trúc: **số đếm connection của bạn là cục
bộ.** Với M instance proxy, mỗi instance chỉ biết những connection *chính nó*
đã mở. Mỗi instance độc lập tính toán "upstream 7 đang ít tải nhất" từ góc
nhìn một phần của riêng nó, và tất cả chúng đồng thời gửi request tiếp
theo tới đó. Thuật toán được kỳ vọng dàn đều tải đã synchronization M proxy
vào một host. Chất lượng của least-connection giảm khi số lượng proxy
tăng, chính xác là điều ngược lại với những gì bạn muốn từ một câu chuyện
scaling — và đó là lý do cho phần tiếp theo.

### Power of two choices (P2C)
Thay vì quét tìm minimum toàn cục, chọn **hai upstream ngẫu nhiên** và lấy
cái ít tải hơn trong hai:

```rust
fn pick_p2c<'a>(upstreams: &'a [Upstream]) -> &'a Upstream {
    let (a, b) = two_distinct_random_indices(upstreams.len());
    if upstreams[a].load() <= upstreams[b].load() { &upstreams[a] } else { &upstreams[b] }
}
```

Đây là thuật toán đáng biết nhất ở đây, vì hai lý do. Nó là O(1) cho mỗi
lượt chọn bất kể kích thước pool — không quét, không heap, không cấu trúc
sắp xếp. Và tính ngẫu nhiên chính là thứ sửa hiện tượng herd ở trên: hai
proxy chọn độc lập hiếm khi lấy mẫu cùng một cặp, nên chúng không hội tụ
vào cùng một host "tốt nhất" theo cách least-connection chính xác làm.
Kết quả kinh điển ("The Power of Two Random Choices") là lấy mẫu hai thay
vì một giảm max load từ `O(log n / log log n)` xuống `O(log log n)` — một
cải thiện theo hàm mũ — trong khi lấy mẫu nhiều hơn hai gần như không thêm
gì. Đây là mặc định trong linkerd và có sẵn trong Envoy, và nói chung nó
cũng nên là mặc định của bạn.

Gotcha: P2C chỉ tốt bằng metric tải mà bạn đem so sánh. Với `active_conns`
nó thừa hưởng bug leak-khi-cancellation từ [`01-upstream.md`](01-upstream.md) (một counter bị
leak làm một host khỏe mạnh trở nên vĩnh viễn kém hấp dẫn); với latency nó
thừa hưởng vấn đề host-nguội bên dưới.

### Latency-aware: peak EWMA
Số lượng connection là một proxy cho load, không phải bản thân load — một
upstream với 4 request nhanh ít tải hơn một upstream với 3 request chậm.
Peak EWMA chấm điểm mỗi upstream bằng một exponentially-weighted moving
average của latency phản hồi quan sát được, nhân với số request đang chờ,
và chọn điểm thấp nhất (thường kết hợp với P2C thay vì quét toàn cục). Nó
phản ứng với một host đã trở nên chậm mà không hề fail — trường hợp
degradation một phần mà health check hoàn toàn bỏ lỡ ([`03-healthcheck.md`](03-healthcheck.md)).

Gotcha: một host không nhận traffic thì không có mẫu latency gần đây, nên
EWMA của nó cũ — và cũ-mà-nhanh trông giống host tốt nhất trong pool, gửi
cho nó một đợt bùng nổ. Hãy decay average về một mặc định bi quan theo
thời gian, hoặc coi "không có mẫu gần đây" là một trạng thái riêng biệt
thay vì một điểm số tuyệt vời. Cùng lý luận đó áp dụng cho một host vừa
trở lại từ trạng thái circuit-breaker mở.

### Consistent Hash
Dùng khi bạn cần cùng một client (hoặc cache key) luôn rơi vào cùng một
upstream — session affinity, hoặc caching phía upstream. Hash upstream lên
một ring (thường với 100+ virtual node cho mỗi upstream để làm mượt phân
phối), hash request key, đi theo chiều kim đồng hồ tới node đầu tiên.

```rust
fn ring_lookup(ring: &std::collections::BTreeMap<u64, usize>, key_hash: u64) -> usize {
    ring.range(key_hash..).next()
        .or_else(|| ring.iter().next())
        .map(|(_, &upstream_idx)| upstream_idx)
        .expect("ring is non-empty")
}
```
Gotcha: bỏ một upstream ra khỏi ring N-upstream chỉ remap ~1/N số key (đó
chính là mấu chốt so với `hash % N`), nhưng *thêm* virtual node mà không
cẩn thận vẫn có thể làm lệch phân phối — luôn benchmark độ cân bằng của
ring, đừng giả định.

Gotcha: consistent hashing phân phối đều *key*, đó không phải cùng một
chuyện với phân phối đều *load*. Một key nóng — một tài khoản celebrity
duy nhất, một cache entry mà ai cũng muốn — rơi hoàn toàn vào một upstream
và không có số lượng virtual node nào giúp được, vì ring đang làm đúng
những gì bạn yêu cầu. Cách giảm nhẹ là "consistent hashing with bounded
loads": nếu upstream được chọn vượt quá một ngưỡng load, đi tới node kế
tiếp trên ring thay vào đó. Bạn từ bỏ affinity nghiêm ngặt cho lát traffic
bị quá tải, đó là đánh đổi đúng khi phương án còn lại là một host tan
chảy.

Gotcha: hash phải ổn định qua các process và các lần restart. `DefaultHasher`
của `std` được nói rõ là không ổn định qua các bản Rust, và `SipHash` của
`HashMap` được seed ngẫu nhiên theo từng process (xem
[`13-algorithms/hashmap.md`](../13-algorithms/hashmap.md)) — hai instance proxy dùng nó sẽ xây *các ring
khác nhau* từ cùng một config và bất đồng về mọi key. Dùng một hash
fixed-seed, chỉ định rõ ràng (xxHash, hoặc SipHash với một key hằng số)
cho bất cứ thứ gì mà kết quả phải khớp qua các process.

### Chọn giữa các thuật toán
Một hướng dẫn quyết định ngắn gọn, vì đây là câu hỏi thật sự khi bạn ngồi
xuống viết [`proxy/`](../../proxy):
- Không cần affinity, chi phí request đồng đều, pool nhỏ → round robin.
  Nó rẻ và điểm yếu của nó không phát tác.
- Không cần affinity, chi phí request thay đổi → **P2C trên một load
  metric**. Đây là mặc định hợp lý cho một proxy thật.
- Degradation một phần là rủi ro chính của bạn (host chậm, không phải host
  chết) → P2C trên peak EWMA.
- Cần affinity (cache phía upstream, sticky session) → consistent hash,
  với bounded loads nếu bất kỳ key nào có thể trở nên nóng.

## Practice
Xây theo thứ tự — mỗi bước cần các phép đo của bước trước để đánh giá.

1. Trong [`labs/06-load-balancer`](../../labs/06-load-balancer), cài đặt round robin. **Xong khi** một
   load test cho thấy request được phân phối trong ±1% giữa 3 upstream giả
   bằng nhau.
2. Thêm một upstream cố ý chậm (inject 200ms vào một backend) và chạy lại
   cùng test. **Xong khi** bạn có thể chỉ ra round robin vẫn gửi một phần
   ba traffic cho nó và p99 của bạn tan nát — đây là baseline mà mọi thuật
   toán sau phải vượt qua.
3. Thêm least connection. **Xong khi** test upstream-chậm cho thấy traffic
   tới host chậm giảm rõ rệt và p99 tốt hơn bước 2, với số liệu ghi lại cho
   cả hai.
4. Tái tạo hiện tượng herd phân tán: chạy 3 instance của balancer nhắm vào
   cùng một pool, tất cả dùng least-connection chính xác, và log upstream
   nào mỗi instance chọn cho mỗi request. **Xong khi** bạn có thể chỉ ra
   các instance hội tụ vào cùng một upstream đồng thời.
5. Cài đặt P2C trên cùng counter đó. **Xong khi** chạy lại bước 4 cho thấy
   sự hội tụ biến mất, và chạy lại bước 3 cho thấy p99 ít nhất tốt bằng
   least-connection chính xác — với chi phí O(1) thay vì O(n) mỗi lượt
   chọn.
6. Cài đặt chấm điểm peak EWMA đứng sau cùng một trait và kết hợp với P2C.
   **Xong khi** một test với một upstream *chậm nhưng khỏe mạnh* route
   traffic tránh xa nó nhanh hơn cách chấm điểm dựa trên số connection, và
   một test với một upstream đang idle cho thấy nó không nhận một đợt
   bùng nổ khi lần đầu có traffic.
7. Cài đặt consistent hash theo key từ một header. **Xong khi** cùng một
   key rơi vào cùng một upstream qua một lần restart toàn bộ process
   (chứng minh hash của bạn ổn định), và bỏ một upstream khỏi ring 5-host
   remap gần đúng 20% số key — hãy đo, đừng giả định.
8. Thêm bounded loads vào ring. **Xong khi** một test nơi 90% request chia
   sẻ một key phân tán traffic đó ra nhiều upstream thay vì làm tan chảy
   một cái, trong khi các key ít lưu lượng vẫn giữ affinity hoàn hảo.
