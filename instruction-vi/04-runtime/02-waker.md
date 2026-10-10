# Waker & Poll

Cách một `Future` báo cho executor "poll tôi lần nữa."

## What to learn

### Hợp đồng của `Future::poll`
`Future::poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>`
hoặc trả về `Poll::Ready(value)` hoặc `Poll::Pending`. Trả về `Pending`
là một lời hứa: *thứ gì đó* sẽ gọi `cx.waker().wake()` một khi future này
có thể tiến triển tiếp. Nếu không có gì gọi nó, task sleep mãi mãi — một
lớp bug thật và phổ biến ("lost wakeup").

```rust
impl Future for MyTimer {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.deadline_reached() {
            Poll::Ready(())
        } else {
            self.register_waker(cx.waker().clone()); // ai đó phải gọi .wake() sau này
            Poll::Pending
        }
    }
}
```

### `Waker`, `RawWaker`, và `std::task::Wake`
Một `Waker` là một handle bị xóa kiểu (type-erased) trỏ ngược tới "schedule lại chính task này." Trước đây bạn xây một cái từ một `RawWaker` +
một vtable tự viết tay (clone/wake/wake_by_ref/drop dưới dạng con trỏ
hàm thô trên một con trỏ kiểu `Arc`) — unsafe, dễ làm sai. `std::task::Wake`
(stable từ 1.68) bọc lại sự unsafe đó: implement `wake(self: Arc<Self>)`
trên type task của bạn và standard library tự xây vtable cho bạn.

### Wakeup thực sự đến từ đâu
Với các future I/O, waker cuối cùng được lưu trong reactor, keyed theo fd
đã đăng ký (xem [`04-runtime/01-tokio.md`](01-tokio.md)); epoll báo readiness chính là
thứ kích hoạt `.wake()`. Với một future tự viết tay (một timer, một
channel), *bạn* chịu trách nhiệm gọi `.wake()` đúng thời điểm — ví dụ một
thread nền bắn khi tới deadline, hoặc phía gửi của một channel wake up
phía nhận.

### Mô hình poll-driven vs push-driven
Nghĩ về `poll` như "hỏi, đừng báo" sẽ hữu ích: executor hỏi một future
"xong chưa?", và cách duy nhất future có thể nói "chưa, nhưng tôi sẽ báo
khi xong" là cất giữ waker. Nhiều lời gọi `.poll()` với các waker khác
nhau (ví dụ giữa các nhánh `select!`) luôn phải wake up waker *mới nhất*
đã đăng ký, không phải một cái cũ — đây là một bug kinh điển trong các
future tự viết tay.

## Practice
1. Trong executor tự viết tay từ bài tập [`03-rust/05-async.md`](../03-rust/05-async.md), implement
   một `Waker` qua `std::task::Wake`: `wake()` nên đẩy id của task trở
   lại một run queue (ví dụ một `VecDeque` sau một `Mutex`, hoặc một
   channel `mpsc`).
2. Viết tay một future `Delay` (lưu một deadline `Instant`) spawn một
   `std::thread` nền để sleep rồi gọi `.wake()`, và drive nó tới hoàn thành
   trên mini executor của bạn.
3. Cố tình implement một future có bug là drop waker thay vì lưu nó, chạy
   nó, và quan sát task không bao giờ được wake up — xác nhận bạn hiểu
   vì sao.
4. So sánh waker tự viết tay của bạn với waker thật của tokio bằng cách
   chạy cùng một future dưới `#[tokio::main]` và dưới executor của riêng
   bạn.
