# Ring Buffer

Một circular buffer kích thước cố định. [`08-observability/01-logging.md`](../08-observability/01-logging.md)
cần một cái để logging không bao giờ allocate trên hot path của request và
không bao giờ block nó khi chờ một writer chậm.

## What to learn

### Cấu trúc
Một `Vec<T>` dung lượng cố định cộng với một head và tail index, cả hai
đều lấy modulo theo capacity:

```rust
struct RingBuffer<T> {
    buf: Vec<Option<T>>,
    head: usize, // vị trí ghi tiếp theo
    tail: usize, // vị trí đọc tiếp theo
    cap: usize,
}
// push: buf[head] = Some(item); head = (head + 1) % cap
// pop:  let item = buf[tail].take(); tail = (tail + 1) % cap
```

Không allocate sau khi khởi tạo, không dịch chuyển phần tử — mỗi push và
pop đều là O(1), đó chính là ý nghĩa của cấu trúc này khi nằm trên hot
path.

### SPSC: lock-free giữa đúng một producer và một consumer
Khi chỉ có đúng một thread ghi và một thread đọc — trường hợp phổ biến
"request handler tạo ra dòng log, một task nền ghi chúng xuống đĩa" — ring
buffer không cần lock nào cả: producer chỉ ghi `head` và đọc `tail`,
consumer chỉ ghi `tail` và đọc `head`, và mỗi bên chỉ cần
`Ordering::Release` khi ghi / `Acquire` khi đọc chỉ số của mình để phía
kia thấy đúng. Đây chính là điều `tracing_appender::non_blocking` (được
nhắc tới trong [`08-observability/01-logging.md`](../08-observability/01-logging.md)) và đa số crate SPSC
channel (`crossbeam`, `ringbuf`) implement.

### Chính sách khi đầy: không bao giờ block hot path
Một ring buffer dùng cho logging phải quyết định chuyện gì xảy ra khi đầy,
và "block producer" là lựa chọn mặc định sai cho một thread xử lý request
— nó biến việc đĩa chậm thành latency của request. Hai lựa chọn tiêu chuẩn
là **drop cái mới nhất** (từ chối dòng log vừa đến, rẻ, mất event gần
đây nhất) hoặc **ghi đè cái cũ nhất** (dịch `tail` theo cùng với `head`,
mất lịch sử nhưng không bao giờ từ chối). Chọn dựa trên việc "chúng ta
biết mình đã mất gì đó" (kèm một metric đếm số bị drop,
[`08-observability/02-metrics.md`](../08-observability/02-metrics.md)) có quan trọng hơn việc giữ event mới
nhất hay không.

### Gotcha: false sharing giữa head và tail
`head` và `tail` được ghi bởi hai thread khác nhau (trong trường hợp SPSC)
nhưng nếu chúng nằm trên cùng một cache line, mỗi lần ghi vào một cái sẽ
làm invalidate bản cache của core kia cho cache line đó — hai thread cuối
cùng bị serialize trên lưu lượng cache dù đang chạm vào dữ liệu độc lập về
mặt logic. Đây chính xác là kiểu thất bại của
[`17-performance/02-false-sharing.md`](../17-performance/02-false-sharing.md); pad `head` và `tail` sang các cache
line riêng biệt (`#[repr(align(64))]` trên một wrapper, hoặc xen kẽ với
các trường padding) để sửa nó.

## Practice
1. Implement ring buffer SPSC ở trên với `Acquire`/`Release` ordering
   đúng; viết một test với một thread producer và một thread consumer xác
   nhận không bao giờ có item bị mất hoặc trùng lặp dưới tải.
2. Implement cả hai chính sách khi đầy (drop-newest, overwrite-oldest)
   đằng sau một flag, và thêm một counter đếm số dòng bị drop được export
   thành metric.
3. Nối ring buffer làm buffer đứng sau một đường logging bất đồng bộ trong
   [`proxy`](../../proxy) (hoặc một test harness độc lập mô phỏng nó), và load-test với
   một "disk writer" consumer cố tình chậm để xác nhận việc xử lý request
   không bao giờ bị block bởi nó.
4. Tái hiện chi phí false sharing: benchmark throughput với `head` và
   `tail` nằm liền kề trong bộ nhớ, rồi với chúng được pad sang các cache
   line riêng, và đo sự khác biệt dưới truy cập đồng thời.
