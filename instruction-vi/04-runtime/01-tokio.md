# Tokio Internals

Reactor, executor, scheduler.

## What to learn

### Reactor
Reactor sở hữu event source của OS (epoll trên Linux, xem
[`02-linux/07-epoll.md`](../02-linux/07-epoll.md)) và biến readiness event thành wakeup. Mỗi
`TcpStream`/`TcpListener` đăng ký fd của nó với reactor một lần; khi
epoll báo fd readable, reactor tìm `Waker` gắn với task đang block trên
fd đó và gọi `.wake()`. Reactor không chạy code của bạn — nó chỉ quyết
định *khi nào* một task xứng đáng được `poll()` lần nữa.

### Executor & work-stealing scheduler
Executor multi-threaded của tokio chạy N worker thread, mỗi thread có
một run queue cục bộ, cộng một global injection queue. Worker rảnh rỗi ăn
cắp task từ queue của worker bận thay vì block, giữ CPU bận rộn mà không
cần một khóa trung tâm cho mỗi lần lập lịch. `tokio::spawn` đặt một task
vào local queue của worker hiện tại; rẻ, nhưng nghĩa là một đợt spawn dồn
dập từ một kết nối có thể làm đói các worker khác cho tới lần ăn cắp tiếp
theo.

```rust
#[tokio::main] // runtime multi-threaded, mặc định một worker mỗi core
async fn main() {
    tokio::spawn(handle_connection(socket)); // được lập lịch trên một worker nào đó, không nhất thiết là worker này
}
```

### Blocking work vs `spawn_blocking`
Một điểm `.await` là một lời hứa rằng lần poll hiện tại sẽ không block OS
thread. Bất cứ thứ gì có thể thực sự block (sync file I/O, nén tốn CPU
nặng, một mutex blocking, một lời gọi vào thư viện C đồng bộ) phải đi qua
`tokio::task::spawn_blocking`, chạy nó trên một thread pool blocking
riêng. Quên điều này là cách phổ biến nhất vô tình làm nghẽn toàn bộ một
worker thread — và mọi task được lập lịch trên worker đó — dưới tải.

### Cooperative scheduling & starvation
Tokio giới hạn số lần poll cho mỗi task trước khi buộc nó nhường lại cho
scheduler (`tokio::task::coop`), nên một task luôn sẵn sàng ngay lập tức
(ví dụ một vòng lặp chặt trên một channel trong-memory) không thể độc
chiếm một worker thread mãi mãi. Điều này quan trọng với một proxy: một
kết nối upstream nóng đang bơm dữ liệu không nên làm đói các task health
check trên cùng worker.

### Vì sao điều này quan trọng với một proxy
Một reverse proxy về bản chất là "đọc từ một socket, ghi vào một socket
khác, lặp lại, nhân với hàng chục ngàn kết nối." Việc của tokio là làm
cho điều đó rẻ: một task cho mỗi kết nối (không phải một thread), I/O
non-blocking multiplex qua một nhúm OS thread, và một scheduler giữ mọi
core bận rộn. Làm sai sự phân chia reactor/executor (ví dụ block một
worker thread) làm suy giảm mọi kết nối trên worker đó, không chỉ kết nối
chậm.

## Practice
1. Trong executor tự viết tay bạn sẽ xây ở bài tập [`03-rust/05-async.md`](../03-rust/05-async.md)
   (một scratch project, không thuộc workspace này), poll một `Vec`
   future trong một vòng lặp với một waker no-op, và quan sát nó
   busy-spin thay vì ngủ — đây chính là *lý do* một reactor + waker thật
   sự tồn tại.
2. Trong [`labs/00-tcp-server`](../../labs/00-tcp-server), log OS thread ID nào xử lý mỗi kết nối
   (`std::thread::current().id()`) và xác nhận các kết nối được rải đều
   trên các worker.
3. Cố tình gọi một `std::thread::sleep` blocking bên trong một handler
   async trong `tcp-server` và quan sát các kết nối khác bị nghẽn; sửa
   nó bằng `tokio::time::sleep` rồi lại bằng `spawn_blocking`, và so
   sánh.
4. Đọc metrics worker của tokio (`tokio::runtime::Handle::metrics()`;
   riêng steal count cần `RUSTFLAGS="--cfg tokio_unstable"`, tập con
   stable chỉ có số worker, số task còn sống, và độ sâu global queue) và in
   steal count dưới tải đồng thời.
