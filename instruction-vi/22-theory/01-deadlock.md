# Deadlock

## What to learn

### Bốn điều kiện Coffman
Deadlock cần cả bốn điều kiện đồng thời: mutual exclusion (một resource
bị giữ độc quyền), hold-and-wait (một process giữ một resource trong khi
chờ resource khác), no preemption (một resource không thể bị lấy lại
cưỡng ép), và circular wait (một chu trình các process, mỗi process chờ
resource mà process kế tiếp đang giữ). Phá vỡ bất kỳ một điều kiện nào là
deadlock trở nên không thể — đây là toàn bộ menu chiến lược để giải quyết
nó.

### Resource allocation graph
Mô hình process và resource như một đồ thị có hướng: một cạnh từ process
tới resource nghĩa là "đang chờ," resource tới process nghĩa là "đang
giữ." Với resource chỉ có một instance, một chu trình trong đồ thị này là
điều kiện cần và đủ cho deadlock — phát hiện deadlock chính là phát hiện
chu trình.

```text
A -> R1   (A chờ R1)
R1 -> B   (B đang giữ R1)
B -> R2   (B chờ R2)
R2 -> A   (A đang giữ R2)   => chu trình => deadlock
```

### Dining philosophers
Ví dụ kinh điển: N triết gia, N cái nĩa, mỗi triết gia cần cả hai nĩa cạnh
mình để ăn. "Cầm nĩa trái rồi nĩa phải" ngây thơ khiến mọi triết gia rơi
vào hold-and-wait đồng thời — circular wait kinh điển. Cách sửa: thứ tự
bất đối xứng (một triết gia cầm phải-trước-trái, phá chu trình), một thứ
bậc resource (luôn lấy nĩa số nhỏ hơn trước), hoặc một người phục vụ/giám
sát cấp phép.

```rust
// cách sửa bằng lock ordering: luôn lấy mutex có index nhỏ hơn trước
fn eat(left: &Mutex<Fork>, right: &Mutex<Fork>, left_idx: usize, right_idx: usize) {
    let (first, second) = if left_idx < right_idx { (left, right) } else { (right, left) };
    let _a = first.lock().unwrap();
    let _b = second.lock().unwrap();
}
```
Đây không phải một bài toán đồ chơi — nó chính xác là hình dạng của một
bug thật: hai lock trong một connection pool (lock free-list và lock
stats-counter của [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)) bị lấy theo thứ tự khác nhau
bởi hai code path deadlock một proxy trong production, và nó trông chính
xác như thế này.

### Prevention, avoidance, detection+recovery
Ba chiến lược, theo thứ tự tốn kém. **Prevention** loại bỏ một điều kiện
Coffman về mặt cấu trúc — một lock ordering toàn cục, hoặc request tất cả
resource ngay từ đầu. **Avoidance** (giải thuật Banker's) chỉ cấp một
resource request nếu state kết quả vẫn "an toàn" (tồn tại một thứ tự nào
đó để mọi process vẫn hoàn thành được), tính động — về lý thuyết đẹp,
hiếm khi implement vì cần biết trước nhu cầu resource tối đa. **Detection
+ recovery** để deadlock xảy ra, định kỳ check chu trình, và kill hoặc
rollback một process để phá vỡ nó. Hầu hết hệ thống thật, kể cả standard
library của Rust, không làm điều nào trong số này một cách hình thức —
chúng dựa vào prevention bằng quy ước (kỷ luật lock ordering) vì
avoidance và detection quá tốn kém để chạy trên hot path.

### Vì sao Rust không cứu bạn ở đây
Borrow checker của Rust ngăn data race (hai thread mutate cùng memory
không đồng bộ) lúc compile, nhưng deadlock là một bug liveness, không phải
bug memory-safety — code hoàn toàn an toàn, nó chỉ đơn giản là không bao
giờ tiến triển. Không có gì trong type system ngăn bạn lấy hai `Mutex`
theo thứ tự không nhất quán qua hai code path. Feature deadlock-detection
của `parking_lot` (một bộ phát hiện chu trình chỉ dùng cho debug, trên các
lock đang giữ) là thứ gần nhất với hỗ trợ tự động, và nó là opt-in, không
phải mặc định.

## Practice
1. Implement dining philosophers với `std::sync::Mutex` một cách ngây thơ
   (luôn trái-rồi-phải) và tái tạo một deadlock thật dưới load — xác nhận
   mọi thread đều bị kẹt, không chỉ chậm.
2. Sửa nó bằng lock ordering (luôn lấy nĩa index nhỏ hơn trước) và xác
   nhận cùng load đó không còn deadlock.
3. Vẽ tay resource allocation graph cho một tình huống hai-lock trong
   connection pool của [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) nơi hai code path lấy
   lock free-list và lock stats theo thứ tự ngược nhau; xác định chu
   trình.
4. Bật feature `deadlock_detection` của `parking_lot` trong một ví dụ
   nhỏ, cố tình deadlock hai thread, và quan sát nó báo cáo chu trình.
5. Viết ra quy tắc lock-ordering của riêng bạn cho các lock thật của
   [`proxy`](../../proxy) (nếu nó có nhiều hơn một) và thêm nó như một comment code ở
   nơi định nghĩa mỗi lock — đây là cách "prevention bằng quy ước" trong
   thực tế.
