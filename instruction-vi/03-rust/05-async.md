# Async/Future

## What to learn

### Trait `Future`
Một `async fn` là cú pháp đường (sugar) cho một hàm trả về
`impl Future<Output = T>`. `Future` có một method bắt buộc,
`poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<T>`, trả về
`Poll::Ready(value)` hoặc `Poll::Pending`. Không có gì chạy cho tới khi
thứ gì đó gọi `poll` — một `Future` tự thân là một state machine trơ,
không phải một đơn vị công việc đã được lên lịch. Đây là lý do vì sao thân
của một hàm async không chạy chút nào cho tới khi nó được `.await` hoặc
spawn.

```rust
async fn fetch(id: u64) -> Response { /* ... */ }

let fut = fetch(1); // nothing has run yet — fut is just a value
let resp = fut.await; // NOW it runs, possibly suspending at internal .await points
```

### Async/await desugaring và state machine
Compiler biến thân của một `async fn` thành một struct vô danh implement
`Future`, với một enum variant cho mỗi điểm suspension (`.await`) — mỗi
variant giữ đúng những local variable còn cần thiết sau điểm đó. Đây là lý
do vì sao kích thước của future được cố định tại compile time và vì sao
các local được giữ qua `.await` phải thỏa mọi bound mà future cần (thường
là `Send`, nếu bạn định `tokio::spawn` nó) — xem [`03-rust/02-lifetimes.md`](02-lifetimes.md)
để biết điều gì bị vỡ khi một borrow là một trong các local-được-giữ-qua-
await đó.

### Cooperative scheduling và blocking
Executor của tokio là cooperative: một task chạy cho tới khi nó trả về
`Poll::Pending` (thường vì đang chờ I/O) hoặc hoàn tất — nó không bao giờ
bị preempt giữa chừng một lần poll. Gọi một hàm *blocking* thật sự
(`std::thread::sleep`, sync file I/O, một vòng lặp nặng CPU) bên trong một
async fn làm nghẽn toàn bộ worker thread, bỏ đói mọi task khác được lên
lịch trên nó. Đây là một trong những bug tokio thực tế phổ biến nhất trong
một proxy: một connection làm DNS hoặc disk I/O đồng bộ âm thầm làm nghẽn
các connection không liên quan đang chia sẻ thread đó.

```rust
// WRONG in async code: blocks the whole worker thread
std::thread::sleep(std::time::Duration::from_secs(1));

// RIGHT: yields control back to the executor while waiting
tokio::time::sleep(std::time::Duration::from_secs(1)).await;

// For unavoidable blocking work (sync I/O, heavy CPU):
tokio::task::spawn_blocking(|| { /* do_blocking_work() */ });
```

### Cancellation: drop chính là cancel
Drop một `Future` trước khi nó resolve sẽ hủy nó ngay lập tức, tại bất kỳ
điểm `.await` nào nó đang bị suspend — không có callback dọn dẹp nào ngoài
`Drop` thông thường. `tokio::select!` và timeout (`tokio::time::timeout`)
dựa hoàn toàn vào điều này: future của nhánh "thua" đơn giản là bị drop.
Điều này nghĩa là code async phải được viết sao cho việc bị drop giữa
chừng một thao tác không bao giờ để lại shared state không nhất quán — ví
dụ một request gửi dở tới upstream cần hoặc là commit hoàn toàn hoặc được
bỏ dở an toàn, không để một connection ở trạng thái lấp lửng.

```rust
tokio::select! {
    resp = upstream_call() => handle(resp),
    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => handle_timeout(),
}
// if the timeout branch wins, upstream_call()'s future is dropped mid-flight
```

Gotcha: một future bị drop không có nghĩa là thao tác ở mức OS bên dưới
(ví dụ một syscall write đang bay) bị hoàn tác — cancellation là hợp tác
(cooperative) ở mức Rust, không phải ở mức kernel. Xem
[`06-proxy/05-retry.md`](../06-proxy/05-retry.md) để biết điều này nghĩa là gì với tính an toàn của
retry (idempotency).

## Practice
1. Đọc hình dạng state-machine được desugar bằng tay: viết một async fn
   có 2 điểm `.await`, rồi phác thảo enum mà compiler sẽ sinh ra cho nó
   (local nào sống trong variant nào).
2. Chủ động tái tạo một bug stalled-executor: gọi `std::thread::sleep`
   bên trong một async task trên một runtime chỉ có một worker thread
   trong khi một task khác cố gắng tiến triển; quan sát sự nghẽn, rồi sửa
   nó bằng `tokio::time::sleep` hoặc `spawn_blocking`.
3. Xây một `tokio::select!` đua một thao tác thật với một timeout; xác
   nhận qua một `Drop` impl trên một guard type rằng nhánh thua thực sự bị
   drop/hủy.
4. Trong một scratch project (không thuộc workspace này — không có lab
   riêng cho một executor tự viết ở đây), implement trait `Future` bằng
   tay cho một timer type đơn giản (`poll` trả về `Pending` cho tới một
   deadline, `Ready(())` sau đó), rồi drive nó bằng executor của chính bạn
   thay vì của tokio.
5. Trong cùng scratch executor đó, implement một `Waker` (qua
   `std::task::Wake` hoặc `RawWakerVTable`) và một executor với một
   run-queue duy nhất chỉ poll một task khi waker của nó được gọi — đây là
   cơ chế mà cả [`04-runtime/02-waker.md`](../04-runtime/02-waker.md) lẫn reactor của tokio đều xây
   dựng trên đó.
