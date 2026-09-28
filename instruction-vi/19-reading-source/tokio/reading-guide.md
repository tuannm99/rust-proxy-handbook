# Reading guide: tokio

Một lộ trình đi qua source của tokio, kèm các câu hỏi cần trả lời trên
đường đi. Đây là guide, không phải notes: các file theo từng project liệt
kê trong [`00-README.md`](00-README.md) (`architecture.md`, `request-flow.md`, ...) là để bạn
tự viết từ những gì tìm được. Câu trả lời cố ý không có ở đây — tự tìm ra
chúng trong code chính là bài tập.

Repository: `github.com/tokio-rs/tokio`, thư mục crate `tokio/src/`. Đường
dẫn file bên dưới khớp với tokio 1.x tại thời điểm viết; code di chuyển
giữa các release, nên nếu một đường dẫn biến mất, hãy search trong
repository tên type hoặc function được nêu bên cạnh.

## Khi nào đọc phần nào

| Đọc | Sau khi | Vì sao lúc đó |
| --- | --- | --- |
| Điểm dừng 1-2: I/O driver | Bài tập raw-epoll trong [`02-linux/07-epoll.md`](../../02-linux/07-epoll.md) | Vòng `epoll_wait` của chính bạn là thứ để so sánh |
| Điểm dừng 3-4: scheduler và task | [`04-runtime/01-tokio.md`](../../04-runtime/01-tokio.md), [`04-runtime/02-waker.md`](../../04-runtime/02-waker.md), [`labs/00-tcp-server`](../../../labs/00-tcp-server) | Bạn đã spawn task và thấy chúng rải qua các worker |
| Điểm dừng 5: blocking pool và coop | [`04-runtime/03-runtime-config.md`](../../04-runtime/03-runtime-config.md) | Bạn đã tự làm kẹt một worker và sửa nó |
| Điểm dừng 6: channel | [`03-rust/11-concurrency-patterns.md`](../../03-rust/11-concurrency-patterns.md) | Bạn đã dùng `mpsc`/`oneshot` trong một lab |

## Lộ trình

### Điểm dừng 1: từ `TcpStream::read` xuống tới readiness
Bắt đầu ở `net/tcp/stream.rs` và theo một lần đọc: `TcpStream` bọc một
`PollEvented` (`io/poll_evented.rs`), thứ nói chuyện với I/O driver
(`runtime/io/driver.rs`, `runtime/io/registration.rs`,
`runtime/io/scheduled_io.rs`).
- Một lần đọc lẽ ra sẽ block biến thành `Poll::Pending` ở đâu?
- Waker của task được lưu ở đâu trong lúc chờ, và cấu trúc dữ liệu nào gắn nó với socket?
- tokio làm gì khi OS báo readiness nhưng lần đọc vẫn trả về `WouldBlock`?

### Điểm dừng 2: chính vòng lặp driver
Trong `runtime/io/driver.rs`, tìm hàm block chờ event (nó đi qua `mio`,
xem [`19-reading-source/mio/`](../mio)).
- So nó với vòng `epoll_wait` bạn tự viết. tokio làm gì mà bạn không làm?
- Nó edge-triggered hay level-triggered? Chỗ nào trong code cho bạn câu trả lời, và tokio tránh bug missed-wakeup trong [`02-linux/07-epoll.md`](../../02-linux/07-epoll.md) thế nào?

### Điểm dừng 3: scheduler work-stealing
`runtime/scheduler/multi_thread/worker.rs` là vòng lặp worker;
`runtime/scheduler/multi_thread/queue.rs` là run queue của từng worker;
global inject queue nằm dưới `runtime/scheduler/inject`.
- Một worker tìm task kế tiếp theo thứ tự nào: local queue, global queue, stealing, I/O driver?
- Một lần steal lấy bao nhiêu task, và vì sao không chỉ một?
- LIFO slot là gì, và nó tối ưu cho workload nào?

### Điểm dừng 4: task là gì
`runtime/task/` — bắt đầu với `raw.rs`, `harness.rs`, `state.rs`, và
`join.rs`.
- State word của task biểu diễn cùng lúc "running", "notified", "complete", và "cancelled" thế nào, và vì sao nó là một atomic duy nhất?
- Chuyện gì xảy ra, từng bước, khi bạn gọi `JoinHandle::abort()` lên một task đang chạy trên thread khác?
- Một panic bên trong task bị bắt và biến thành `JoinError` ở đâu ([`03-rust/08-error-handling.md`](../../03-rust/08-error-handling.md))?

### Điểm dừng 5: `spawn_blocking` và cooperative budgeting
Blocking pool nằm dưới `runtime/blocking/`. Cooperative budget là module
`coop` (search `coop` nếu nó đã chuyển chỗ).
- Blocking pool quyết định mở thread mới hay dùng lại một thread rảnh thế nào, và khi nào thread rảnh thoát?
- Budget của task bị trừ ở đâu, và chuyện gì xảy ra khi nó về 0 giữa một vòng lặp bận rộn trên channel?

### Điểm dừng 6: một channel từ đầu tới cuối
`sync/mpsc/` — `bounded.rs`, `chan.rs`, và danh sách liên kết theo block mà nó dùng.
- Vì sao hàng đợi là một danh sách liên kết các block kích thước cố định thay vì ring buffer hay `VecDeque`?
- Trên một bounded channel, một `send().await` khi channel đầy park ở đâu, và cái gì đánh thức nó?

## Viết gì vào notes
Sau lộ trình, viết các file theo project trong [`00-README.md`](00-README.md). Tối thiểu,
`request-flow.md` nên lần theo một `TcpStream::read` từ code của bạn xuống
tới syscall rồi quay lên tới lúc task của bạn được poll lại — bằng lời của
bạn, kèm tên file — và `what-to-learn.md` nên liệt kê các file handbook mà
code đã xác nhận, mâu thuẫn, hoặc đi xa hơn.
