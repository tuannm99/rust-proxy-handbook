# Các bài toán Synchronization kinh điển

## What to learn

### Producer-consumer (bounded buffer)
Một hoặc nhiều producer thêm item vào một buffer kích thước cố định, một
hoặc nhiều consumer lấy chúng ra; buffer không bao giờ được viết khi đầy
hoặc đọc khi rỗng. Giải pháp textbook kinh điển dùng hai semaphore đếm
(theo dõi slot rỗng và slot đầy) cộng một mutex cho buffer — ba primitive
hợp tác, chính xác là thứ một `mpsc` channel có bound
([`03-rust/11-concurrency-patterns.md`](../03-rust/11-concurrency-patterns.md)) cho bạn miễn phí, đã được chứng
minh đúng, thay vì tự viết tay.

```rust
// tokio::sync::mpsc::channel(capacity) *chính là* một producer-consumer đã giải:
// bounded capacity = kích thước buffer, .send().await block khi đầy,
// .recv().await block khi rỗng — không cần tự xoay semaphore bằng tay
let (tx, rx) = tokio::sync::mpsc::channel::<Job>(16);
```

### Readers-writers
Bất kỳ số reader nào có thể giữ một resource đồng thời, nhưng một writer
cần quyền truy cập độc quyền — đọc không xung đột với nhau, chỉ xung đột
với viết. Bài toán kinh điển là về *fairness*: một implementation ngây
thơ có thể starve writer vô hạn nếu reader liên tục đến, hoặc starve
reader nếu writer được ưu tiên. `RwLock` ([`03-rust/04-sync.md`](../03-rust/04-sync.md)) giải sẵn
bài toán này cho bạn, nhưng chính sách fairness của nó là một lựa chọn
thật, có hệ quả, không phải một chi tiết.

Gotcha: `std::sync::RwLock` của Rust không đảm bảo fairness chút nào —
một `RwLock` bị starve writer dưới áp lực đọc liên tục là một failure mode
thật, âm thầm. `parking_lot::RwLock` và `tokio::sync::RwLock` ghi rõ chính
sách fairness cụ thể của chúng; đọc nó trước khi giả định "readers-writers"
tự động nghĩa là "không có starvation."

### Semaphore vs mutex, một cách hình thức
Một mutex là một semaphore nhị phân kèm một quy tắc ownership (chỉ thread
đã lock mới được unlock); một semaphore đếm chung không có quy tắc đó —
bất kỳ thread nào cũng có thể increment nó, và nó bắt đầu ở N thay vì 1,
mô hình hóa N permit resource có thể thay thế nhau thay vì một resource
bị giữ độc quyền. Đây chính xác là lý do `tokio::sync::Semaphore` (dùng
cho accept-rate limiter trong [`07-security/09-ddos.md`](../07-security/09-ddos.md)) là primitive
đúng cho "nhiều nhất N thứ đồng thời," và một `Mutex` là sai — một mutex
không có khái niệm "N permit," chỉ có "đã lock hay chưa."

```rust
let permits = tokio::sync::Semaphore::new(100); // 100 permit có thể thay thế nhau, không phải 1 resource bị giữ độc quyền
```

### Bài toán barrier
Một barrier làm mọi thread tham gia chờ tới khi *tất cả* thread đã đến
điểm barrier trước khi bất kỳ thread nào được tiếp tục — hữu ích cho một
vòng công việc song song cố định phải hoàn thành hoàn toàn trước khi vòng
kế tiếp bắt đầu. `std::sync::Barrier` implement trực tiếp cái này; nó
hiếm gặp trong một proxy phục vụ request (không có "vòng" tự nhiên) nhưng
phổ biến trong code batch/warm-up — warm cache song song qua N shard, chờ
tất cả shard trước khi phục vụ traffic.

### Vì sao những cái này đáng biết dù đã có channel
Mỗi bài toán kinh điển này đều "đã giải" theo nghĩa một primitive thư
viện đã tồn tại cho nó, nhưng nhận ra một vấn đề thật có hình dạng kinh
điển nào mới là thứ cho bạn biết primitive nào thực sự đúng. Dùng một
`Mutex` khi hình dạng thật là readers-writers, hoặc một channel thường
khi hình dạng thật cần đếm permit của semaphore, tạo ra code về kỹ thuật
hoạt động được nhưng có đặc tính performance hoặc fairness sai dưới load.

## Practice
1. Implement producer-consumer có bound bằng tay với hai `std::sync::Condvar`
   (hoặc `tokio::sync::Notify`) thay vì channel, để cảm nhận channel làm gì
   cho bạn miễn phí; rồi đổi sang `tokio::sync::mpsc` và xóa phiên bản
   viết tay.
2. Viết một test readers-writers cố tình starve writer (nhiều reader
   chồng nhau, một writer) dùng `std::sync::RwLock`; rồi đổi sang
   `parking_lot::RwLock` hoặc `tokio::sync::RwLock` và so sánh hành vi
   fairness.
3. Thay một `Mutex<usize>` dùng như một counter concurrency thô ở đâu đó
   trong workspace của bạn bằng một `tokio::sync::Semaphore`, và giải
   thích trong một câu vì sao phiên bản semaphore là mô hình đúng hơn.
4. Implement một bước warm cache song song qua N shard giả lập dùng
   `std::sync::Barrier` (hoặc `tokio::sync::Barrier`), và xác nhận không
   shard nào bắt đầu phục vụ tới khi cả N đã warm xong.
5. Với một điểm synchronization thật trong [`proxy`](../../proxy) (hoặc một crate
   [`labs/`](../../labs)), gọi tên nó thực sự là bài toán kinh điển nào — bounded
   buffer, readers-writers, hay N-permit — và xác nhận primitive bạn dùng
   khớp với nó.
