# epoll

Edge-trigger vs level-trigger, event loop.

## What to learn

### Vì sao epoll tồn tại
`select`/`poll` quét lại toàn bộ fd đang theo dõi ở mỗi lần gọi — O(n) mỗi
lần wake-up bất kể thực sự có bao nhiêu fd sẵn sàng. Một proxy giữ hàng
chục ngàn kết nối keep-alive idle phải trả cái giá đó liên tục. `epoll`
giữ danh sách theo dõi bên trong kernel (`epoll_create1`) và chỉ trả về
các fd đã trở nên sẵn sàng, nên chi phí tỉ lệ với số fd *sẵn sàng*, không
phải số fd *đang theo dõi*.

### Ba syscall
```rust
// epoll_create1(0)  -> epoll fd
// epoll_ctl(epfd, EPOLL_CTL_ADD/MOD/DEL, fd, &event) -> đăng ký/sửa/xóa
// epoll_wait(epfd, &mut events, max, timeout_ms)     -> block cho tới khi sẵn sàng
use libc::{epoll_event, EPOLLIN, EPOLLOUT};

let mut ev = epoll_event { events: (EPOLLIN | EPOLLOUT) as u32, u64: fd as u64 };
```
Trường `u64` (hay `ptr` trong union) là mối liên hệ duy nhất giữa event và
"đây là kết nối nào" — trong thực tế bạn nhét một token/index vào đó và
tra cứu trạng thái kết nối trong một slab/`Vec`.

### Level-triggered vs edge-triggered
Level-triggered (mặc định) bắn mỗi lần bạn gọi `epoll_wait` miễn là fd
*đang* readable/writable — an toàn nhưng có thể spin nếu bạn không drain
fd. Edge-triggered (`EPOLLET`) chỉ bắn ở *thời điểm chuyển đổi* sang
readable/writable, nghĩa là bạn **phải** đọc/ghi trong một vòng lặp cho
tới khi nhận `EWOULDBLOCK`/`EAGAIN`, nếu không bạn sẽ bỏ lỡ dữ liệu tới
sau lần đọc dở dang cuối cùng và fd sẽ không bao giờ báo cho bạn nữa nữa.
Gotcha: trộn giả định level-triggered với `EPOLLET` là nguồn gốc phổ biến
nhất của bug "kết nối ngẫu nhiên treo mãi mãi" trong các event loop tự
viết tay.

### Mẫu event loop
```
đăng ký listener với EPOLLIN
loop:
    n = epoll_wait(epfd, events, MAX, -1)
    với mỗi event sẵn sàng:
        nếu là listener -> accept() trong một vòng lặp cho tới EAGAIN, đăng ký mỗi fd mới
        nếu không -> đọc/ghi trong một vòng lặp cho tới EAGAIN, cập nhật state machine theo kết nối
```
Mỗi fd sẵn sàng ánh xạ tới một state machine nhỏ (đang đọc header, đang
đọc body, đang ghi response...) — bản thân event loop không có logic
protocol nào, nó chỉ drive bất kỳ state machine nào gắn với token của fd
đó.

### Vì sao reactor của tokio tồn tại
Reactor của tokio chính xác là event loop này, tổng quát hóa: một thread
(hay một pool nhỏ) sở hữu epoll fd, và mỗi `.await` trên một socket đăng
ký một waker gắn với một token thay vì block. Xem `03-rust/05-async.md`
và `04-runtime/01-tokio.md` — hiểu epoll thô trước sẽ giúp điệu nhảy
`Poll::Pending`/waker của tokio trở nên rõ ràng, vì nó là cùng một mô
hình đăng ký với vòng lặp polling và sổ sách kế toán được ẩn đi.

### Xử lý EAGAIN
Socket non-blocking trả về `EWOULDBLOCK`/`EAGAIN` thay vì block khi không
có dữ liệu/chỗ trống buffer. Ở chế độ edge-triggered đây không phải một
lỗi — đó là tín hiệu thoát vòng lặp của bạn. Quên đặt `O_NONBLOCK` trên
socket đã accept (chỉ listener non-blocking là chưa đủ) là một bug kinh
điển âm thầm đưa bạn quay lại blocking I/O theo từng kết nối.

## Practice
1. Trong một scratch project (không thuộc workspace này — không có lab
   riêng cho raw epoll ở đây), implement một listener non-blocking và
   đăng ký nó với `epoll_create1`/`epoll_ctl`.
2. Chạy echo server ở chế độ level-triggered trước; xác nhận nó hoạt
   động dưới `nc` và một bộ sinh tải đồng thời đơn giản.
3. Chuyển sang `EPOLLET` và tái tạo một kết nối bị kẹt bằng cách *không*
   loop tới EAGAIN khi đọc — quan sát sự treo, rồi sửa nó.
4. Thêm một fd đăng ký thứ hai (ví dụ một pipe dùng làm shutdown signal)
   và dispatch trên nó bên trong cùng vòng lặp.
5. So sánh số syscall qua `strace -c` giữa phiên bản epoll của bạn và một
   phiên bản dùng `select` ngây thơ dưới 1000 kết nối idle.
6. Xong rồi, đọc source reactor của tokio (`tokio::runtime::io`) và xác
   định nơi nó gọi cùng ba syscall bạn vừa dùng tay.
