# Service Discovery

## What to learn
### Config tĩnh vs discovery động
[`01-upstream.md`](01-upstream.md) giả định một danh sách upstream cố định. Các deployment
thật thay đổi tập upstream liên tục (autoscaling, rolling deploy, node
fail). Config tĩnh (một danh sách trong file config, reload theo
[`09-architecture/03-config.md`](../09-architecture/03-config.md)) đơn giản nhất và ổn cho các hạm đội
nhỏ/ổn định. Discovery động — bản ghi DNS SRV, Consul, hay Kubernetes
Endpoints/EndpointSlices — cần thiết khi membership của upstream thay đổi
nhanh hơn bạn muốn tự tay sửa config.

Khung nhìn thẳng thắn: discovery động đổi một vấn đề cấu hình lấy một vấn
đề *hệ thống phân tán*. Bạn có được cập nhật membership tự động và có
thêm một dependency mới mà các failure mode của nó (dữ liệu cũ, góc nhìn
một phần, control plane không tới được) giờ quyết định liệu proxy của bạn
có thể route được hay không. Phần lớn file này nói về việc sống sót qua
sự đánh đổi đó.

### Discovery dựa trên DNS
Resolve một tên DNS (thường là một bản ghi SRV, cái cũng mang theo
port + weight, khác với A/AAAA thuần) theo một interval và diff kết quả
với pool hiện tại. Rẻ, không dependency, nhưng bị giới hạn bởi DNS TTL —
bạn không thể phản ứng nhanh hơn TTL, và resolver cache cũ (xem
[`01-network/14-dns.md`](../01-network/14-dns.md)) có thể để bạn trỏ vào một upstream đã ngừng hoạt
động trong một thời gian ngắn sau một thay đổi.

Gotcha, và đây là cái kinh điển: **resolve một lần lúc khởi động không
phải là discovery.** `tokio::net::TcpStream::connect("upstream:8080")`
resolve mỗi lần nó được gọi, nhưng bất kỳ code nào resolve thành một
`SocketAddr` và lưu nó lại đã đóng băng DNS tại thời điểm khởi động
process. Proxy sau đó tiếp tục gửi traffic tới một IP đã được tái sử dụng
cho một workload khác từ nhiều giờ trước — và vì các connection tới nó *thành
công* (có gì đó đang listen), health check không bắt được nó. Các
process sống lâu phải re-resolve theo lịch; bug này vô hình khi test và
hiển nhiên trong production một tuần sau.

Gotcha: `getaddrinfo` (thứ `std`/`tokio` dùng mặc định) trả về địa chỉ
nhưng không trả TTL, nên poll interval của bạn là một phỏng đoán tách rời
khỏi những gì zone thực sự công bố. Nếu hành vi đúng-TTL quan trọng, dùng
một resolver crate phơi bày bản ghi đúng cách (`hickory-resolver`) thay vì
cố suy ra nó.

Gotcha: một tra cứu A record thuần có thể trả về một *tập con* địa chỉ —
nhiều resolver giới hạn response ở những gì vừa một gói UDP, và một số
round-robin tập con nào chúng trả về. Diff "những gì tôi nhận được ở lần
poll này" với "những gì tôi có" sau đó tạo ra các phép xóa và thêm lại ma
của các upstream chưa bao giờ đi đâu cả. Nghi ngờ một tập kết quả bị thu
hẹp (xem guard kết-quả-rỗng bên dưới) thay vì coi nó là sự thật.

### Consul / Kubernetes (push-based)
Thay vì poll, subscribe vào một watch/stream API đẩy các thay đổi tập
upstream khi chúng xảy ra (Consul blocking query, Kubernetes watch trên
Endpoints/EndpointSlices, hoặc xDS của Envoy nếu bạn dùng protocol đó).
Latency phát hiện thay đổi thấp hơn poll DNS, với chi phí là một
dependency vào việc control plane đó có thể truy cập được.

Gotcha: watch có thể gãy. Connection rớt, server restart, một resource
version expire và API bảo bạn bắt đầu lại. Một cài đặt dựa trên watch
không phải "subscribe một lần" — nó là một vòng lặp được giám sát, tự
reconnect với backoff ([`05-retry.md`](05-retry.md)), re-list toàn bộ state khi
reconnect, và đối chiếu state đầy đủ đó với những gì nó đang giữ. Làm
đúng đường đi re-list quan trọng hơn happy path, vì happy path là thứ bạn
test còn re-list là thứ chạy trong lúc có sự cố.

### Không bao giờ chấp nhận một kết quả rỗng
Đây là failure mode biến một sự cố thoáng qua của discovery thành một
outage toàn phần, và đáng để xây guard này trước khi xây tính năng:

```rust
// WRONG: one failed lookup, and the pool is now empty
pool.store(Arc::new(discovered));

// Right: an empty discovery result is far more likely to be a bug,
// a DNS hiccup, or an unreachable control plane than a real fleet of zero.
if discovered.is_empty() {
    tracing::warn!("discovery returned no upstreams; keeping previous set");
    metrics::increment("discovery_empty_result_total");
    return;
}
pool.store(Arc::new(discovered));
```

Một hạm đội thật scale xuống zero là hiếm; một control plane trả về không
gì cả trong lúc chính nó gặp outage thì phổ biến. Giữ lại tập biết-là-tốt
gần nhất nghĩa là một outage của discovery hạ bạn xuống "routing cũ" thay
vì "không routing" — và routing cũ vẫn phục vụ người dùng, trong khi một
pool rỗng trả 503 cho tất cả. Alert trên cảnh báo đó (nó là một vấn đề
thật) nhưng đừng để nó làm sập traffic.

Gotcha: cùng lý luận đó áp dụng cho các thay đổi *lớn*, không chỉ các
thay đổi rỗng. Một lần poll xóa 90% upstream cùng lúc nhiều khả năng là
một góc nhìn một phần hơn là một event thật. Một guard churn-tối-đa
("không bao giờ xóa nhiều hơn X% pool trong một lần cập nhật mà không có
một lần poll xác nhận thứ hai") là bảo hiểm rẻ; cái tương đương của Envoy
là panic threshold của nó ([`03-healthcheck.md`](03-healthcheck.md)), áp dụng ở lớp membership
thay vì lớp health.

### Fail static
Tổng quát hóa điều trên: khi control plane không tới được, tiếp tục phục
vụ config biết-là-tốt gần nhất *vô thời hạn*, và không expire nó theo một
timer. Bản năng thêm "nếu dữ liệu discovery cũ hơn 5 phút, ngừng dùng nó"
cảm giác an toàn và chính xác là ngược lại — nó biến một outage của
control plane (thứ người dùng lẽ ra sẽ không nhận thấy) thành một outage
của data plane (thứ họ chắc chắn sẽ nhận thấy). Data plane nên có thể chạy
hàng ngày trên membership cũ; tính chất đó là thứ biến control plane thành
một dependency không quan trọng thay vì một điểm lỗi duy nhất.

### Phản ứng với thay đổi membership mà không rớt traffic
Bất biến quan trọng: cập nhật tập upstream không bao giờ được drop các
request in-flight tới một upstream đang bị xóa, và không bao giờ được
route request mới tới một tham chiếu cũ.

```rust
struct UpstreamPool {
    upstreams: arc_swap::ArcSwap<Vec<std::sync::Arc<Upstream>>>,
}
```
Swap toàn bộ `Vec` nguyên tử với `arc-swap` (hoặc một `RwLock` nếu bạn
không muốn thêm dependency) thay vì mutate tại chỗ — các reader chọn một
upstream luôn thấy hoặc danh sách cũ hoặc danh sách mới hoàn chỉnh, không
bao giờ một danh sách nửa-cập-nhật.

Chú ý `Arc<Upstream>` bên trong `Vec` mang lại gì ngoài tính nguyên tử:
một request đã nắm lấy một `Arc<Upstream>` trước lần swap giữ nó sống qua
cả quá trình hoàn thành của chính nó, dù `Vec` mới không còn tham chiếu nó
nữa. Việc xóa trở thành dẫn dắt bởi refcount — `Upstream` bị drop khi
request in-flight cuối cùng dùng nó hoàn thành — đó chính xác là ngữ nghĩa
drain bạn muốn, miễn phí, thay vì một kiểm tra thủ công "còn ai đang dùng
cái này không".

### Drain một upstream bị xóa
"Xóa" là ba bước, không phải một, và làm chúng theo thứ tự sai sẽ rớt
traffic:
1. **Ngừng chọn nó** cho request mới (xóa khỏi tập ứng viên của balancer).
2. **Để các request in-flight hoàn thành** — hành vi refcount ở trên xử
   lý cái này nếu bạn giữ `Arc<Upstream>` theo từng request, với một
   deadline cho những cái không bao giờ xong.
3. **Đóng các connection idle trong pool của nó** ([`01-upstream.md`](01-upstream.md)) sau
   cùng. Bỏ qua bước này là bug phổ biến: connection pool giữ các socket
   ấm tới một host mà discovery đã xóa, và nếu pool của bạn được key theo
   địa chỉ thay vì theo identity của `Arc<Upstream>`, một host sau này ở
   cùng địa chỉ đó thừa hưởng chúng.

Gotcha: một đường graceful-shutdown
([`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md)) và một đường xóa-do-discovery
là cùng một logic drain ở các phạm vi khác nhau. Viết nó một lần.

## Practice
Xây theo thứ tự.

1. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), chuyển danh sách upstream hardcode đứng
   sau một trait `Discovery` với một cài đặt tĩnh. **Xong khi** proxy hành
   xử giống hệt như trước và không gì bên ngoài trait biết danh sách đến
   từ đâu.
2. Thêm một cài đặt DNS-poll dùng một resolver re-resolve mỗi tick, diff
   với tập hiện tại và log các phần thêm/xóa. **Xong khi** thay đổi một
   entry `/etc/hosts` cục bộ (hoặc zone của một DNS server cục bộ) được
   nhận ra trong vòng một poll interval — và viết phiên bản
   resolve-một-lần-lúc-khởi-động trước để bạn thấy nó *không* nhận ra.
3. Chuyển pool storage sang `arc_swap::ArcSwap<Vec<Arc<Upstream>>>`.
   **Xong khi** một load test đồng thời chạy trong lúc pool swap lặp lại
   tạo ra zero lỗi và zero torn read.
4. Thêm guard kết-quả-rỗng và guard churn tối đa. **Xong khi** trỏ
   discovery vào một tên resolve ra không gì cả để pool trước đó tiếp tục
   phục vụ traffic, phát ra một cảnh báo, và tăng một metric — với zero
   503 thấy được ở client.
5. Cài đặt drain ba bước. **Xong khi** xóa một upstream giữa lúc load test
   hoàn thành mọi request in-flight tới nó (không reset), và `ss -tan` cho
   thấy các connection idle trong pool của nó đã đóng sau đó thay vì còn
   lại.
6. Xác minh fail-static. **Xong khi** nguồn discovery bị làm cho không tới
   được vĩnh viễn và proxy tiếp tục route đúng trên tập biết-là-tốt gần
   nhất trong bao lâu bạn muốn để nó chạy.
7. (Stretch) Thêm một cài đặt dựa trên watch của Consul hoặc Kubernetes
   đứng sau cùng trait. **Xong khi** latency phát hiện thay đổi đo được
   thấp hơn của DNS poller, *và* kill connection watch giữa lúc test kích
   hoạt một reconnect-và-relist đối chiếu đúng thay vì nhân đôi hoặc xóa
   mất upstream.
