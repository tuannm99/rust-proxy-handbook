# Consistent Hashing

[`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md) nói về việc lookup trên ring và vì sao việc
gỡ bỏ chỉ remap ~1/N số key. File này nói về phần mà bản tóm tắt đó bỏ
qua: cách sizing virtual node, bài toán cân bằng, và bounded loads.

## What to learn

### Vì sao `hash(key) % N` thất bại
Với N upstream, `hash % N` gán key một cách tất định — cho tới khi N thay
đổi. Đi từ 4 upstream lên 5 thay đổi modulus cho *mọi* key: khoảng 80% số
key bị remap. Với session affinity điều đó có nghĩa là mất session hàng
loạt; với một cache phía upstream điều đó có nghĩa là một cơn bão cache
miss gần như toàn bộ đánh vào origin của bạn đúng lúc bạn đang cố thêm
capacity. Consistent hashing tồn tại để đưa tỉ lệ đó về 1/N thay vì vậy.

### Virtual node và bài toán cân bằng
Đặt mỗi upstream tại một điểm duy nhất trên ring cho cân bằng rất tệ: với
4 upstream, bốn cung giữa chúng có kích thước ngẫu nhiên, và mất cân bằng
tải 2-3 lần là chuyện thường. Virtual node sửa điều này bằng cách đặt mỗi
upstream tại V vị trí riêng biệt trên ring (`hash("upstream-1#0")`,
`hash("upstream-1#1")`, ...), để mỗi upstream sở hữu V cung nhỏ mà tổng độ
dài của chúng hội tụ về giá trị trung bình khi V tăng.

Độ lệch chuẩn của tải giảm xấp xỉ theo `1/sqrt(V)`. Điều đó xác định phạm
vi thực tế: V=1 không dùng được, V=10 vẫn cho thấy độ lệch ~30%, V=100-200
rơi vào trong vài phần trăm, và V=1000 mang lại rất ít lợi ích cho gấp 10
lần bộ nhớ.

```rust
use std::collections::BTreeMap;

struct Ring {
    // vị trí trên ring -> chỉ số upstream; V entry cho mỗi upstream
    nodes: BTreeMap<u64, usize>,
    vnodes_per_upstream: usize, // thường là 100-200
}
```

Gotcha: bộ nhớ và chi phí rebuild là `O(upstream × V)`. Ở 1000 upstream ×
200 vnode, đó là 200 nghìn entry `BTreeMap` bị rebuild ở mỗi lần thay đổi
membership — đây là lúc rendezvous hashing
([`13-algorithms/rendezvous-hash.md`](rendezvous-hash.md), không có cấu trúc nào phải rebuild)
hoặc Maglev ([`13-algorithms/maglev.md`](maglev.md), lookup O(1)) trở thành câu trả lời
tốt hơn.

### Weight
Consistent hashing có trọng số được biểu diễn bằng số vnode: một upstream
weight-3 có gấp 3 lần số virtual node của một upstream weight-1, nên nó sở
hữu ~3 lần diện tích ring. Cách này đơn giản nhưng thô — tỉ lệ chỉ chính
xác đến mức mà số vnode cho phép, nên một upstream weight-1 cần đủ vnode
về giá trị tuyệt đối (không chỉ tương đối) để phần của nó ổn định.

### Consistent hashing với bounded load
Consistent hashing thuần không biết gì về tải thật: nếu một key cực kỳ
hot, upstream sở hữu nó bị quá tải trong khi phần còn lại rảnh rỗi.
Consistent hashing *với bounded loads* sửa điều này bằng cách giới hạn mỗi
upstream ở `c × tải_trung_bình` (c hơi lớn hơn 1, ví dụ 1.25); khi việc đi
tới rơi vào một upstream đã đạt giới hạn, nó tiếp tục theo chiều kim đồng
hồ tới upstream tiếp theo còn dư capacity.

Cách này giữ được affinity cho trường hợp phổ biến trong khi suy giảm một
cách nhẹ nhàng dưới các hot key, và đó là điều khiến consistent hashing an
toàn để dùng như một balancer đa dụng thay vì chỉ cho việc routing cache.
Cả Envoy lẫn HAProxy đều có một biến thể của nó.

Gotcha: cái giới hạn (cap) phải được tính dựa trên tải trung bình *hiện
tại* và tính lại khi tải thay đổi, và bước đi tràn (overflow walk) phải bounded — một implementation ngây thơ với mọi upstream đều ở giới hạn sẽ đi
hết cả ring ở mỗi request.

### Nơi nó thực sự có chỗ đứng trong một proxy
Dùng consistent hashing khi upstream giữ state theo từng key mà việc rebuild
nó tốn kém: một cache phía upstream, một sticky session, chủ sở hữu một
shard. *Đừng* dùng nó làm balancer mặc định cho các upstream không trạng
thái — least-connection hay smooth WRR phản ứng với tải thật, còn
consistent hashing thì cố tình không làm vậy.

## Practice
1. Trong [`labs/06-load-balancer`](../../labs/06-load-balancer), xây ring với V=1 và hash 100 nghìn key
   trên 5 upstream; ghi lại phần trăm mỗi upstream và tỉ lệ giữa upstream
   bận nhất và rảnh nhất.
2. Lặp lại với V = 10, 100, 500. Vẽ tỉ lệ max/min theo V và xác nhận nó
   thu hẹp xấp xỉ theo `1/sqrt(V)`; chọn V mà bạn sẽ thực sự ship và giải
   thích vì sao.
3. Đo thời gian xây ring và bộ nhớ ở 1000 upstream × 200 vnode, rồi so
   sánh với implementation HRW từ [`13-algorithms/rendezvous-hash.md`](rendezvous-hash.md) cho
   cùng pool đó.
4. Implement bounded loads: giới hạn mỗi upstream ở 1.25× trung bình và đi
   tràn khi vượt. Gửi 50% traffic vào một hot key và xác nhận tải được
   trải ra thay vì pin vào một upstream.
5. Xác nhận bước đi tràn kết thúc khi mọi upstream đều ở giới hạn — viết
   test mà đáng lẽ đã bắt được một vòng lặp tràn vô hạn.
