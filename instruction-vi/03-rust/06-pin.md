# Pin

## What to learn

### Vì sao Pin tồn tại: self-referential future
Desugar một `async fn` thành một struct state machine (xem
[`03-rust/05-async.md`](05-async.md)) có thể sinh ra một struct borrow từ chính các field
của nó — ví dụ một local variable ở một phần của hàm được borrow bởi một
biểu thức `.await` sau đó trong cùng hàm, cả hai đều được lưu trong cùng
struct được sinh ra. Một struct như vậy không bao giờ được phép di chuyển
trong bộ nhớ sau khi các tham chiếu nội bộ đó đã được thiết lập, vì di
chuyển nó sẽ để tham chiếu nội bộ trỏ vào vị trí cũ. `Pin<P>` là một
wrapper quanh một pointer type giữ đúng một đảm bảo: một khi đã pin, đối
tượng được trỏ tới sẽ không di chuyển nữa (với các type không phải
`Unpin`) — đảm bảo đó chính là thứ khiến việc poll một self-referential
future trở nên sound.

```rust
// Conceptually, an async fn awaiting across a borrow generates something like:
struct GeneratedFuture<'a> {
    local: String,
    borrow: &'a str, // borrows `local`, a sibling field — self-referential
}
// This shape is only sound to poll if it's guaranteed never to move: hence Pin.
```

### `Unpin`: trường hợp phổ biến
Phần lớn type là `Unpin` (được implement tự động cho bất cứ thứ gì không
chứa cấu trúc self-referential) — di chuyển chúng luôn ổn, và
`Pin<&mut T>` cho một `T` là `Unpin` hoạt động y hệt `&mut T` (bạn thậm
chí có thể lấy lại nó qua `Pin::get_mut`). Bạn chỉ cần suy nghĩ kỹ về các
type `!Unpin` khi tự tay implement `Future` cho thứ gì đó giữ tham chiếu
vào chính nó — code ứng dụng thông thường gần như không bao giờ viết
`Pin` bằng tay; nó xuất hiện trong signature của `Future::poll` và trong
code của executor.

```rust
fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
    // `self` is guaranteed not to move again while pending
    std::task::Poll::Pending
}
```

### `Pin<Box<dyn Future<Output = T>>>`
Boxing một future rồi pin cái box đó là cách chuẩn để lưu một future được
allocate trên heap, dynamic-dispatch — ví dụ task queue của một executor
tự viết, hoặc một hàm trả về "một future nào đó, không quan tâm type cụ
thể" mà không dùng `async fn` trong trait. `Box::pin` allocate trên heap
và pin nó ngay lập tức, tránh luôn câu hỏi về việc giá trị di chuyển trên
stack sau đó.

```rust
struct Task {
    future: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
}

let task = Task { future: Box::pin(async { /* ... */ }) };
```

Gotcha: `Pin` chỉ ngăn việc di chuyển *đối tượng được trỏ tới*; nó không
nói gì về tính thread-safety. Một boxed future vẫn cần `+ Send` trong
kiểu trait object của nó nếu executor của bạn poll task từ một thread
pool (executor của tokio làm vậy) — lẫn lộn hai điều này sinh ra một lỗi
compiler khó hiểu liên quan tới `Send` mà thực ra là do thiếu bound trên
trait object, không phải do bản thân `Pin`.

## Practice
1. Viết một struct self-referential `!Unpin` tối giản (không dùng async)
   bằng `PhantomPinned`, pin nó bằng `Box::pin`, và quan sát compiler từ
   chối một nỗ lực di chuyển nó sau đó.
2. Trong bài tập executor tự viết từ [`03-rust/05-async.md`](05-async.md), định nghĩa
   type `Task` của bạn là `Pin<Box<dyn Future<Output = ()> + Send>>` và
   implement run-queue của executor xoay quanh nó.
3. Giải thích bằng lời của bạn (một comment cũng được) vì sao
   `Future::poll` nhận `self: Pin<&mut Self>` thay vì `&mut self` thông
   thường — liên hệ lại với hình dạng struct self-referential từ
   [`03-rust/05-async.md`](05-async.md).
4. Đọc docs của thư viện chuẩn cho `Pin::get_mut` và `Pin::new_unchecked`
   và viết ra, chính xác, invariant nào mà `new_unchecked` yêu cầu bạn tự
   tay giữ đúng (đây là unsafe — xem [`03-rust/03-unsafe.md`](03-unsafe.md)).
