# epoll

Edge-triggered và level-triggered, event loop.

## What to learn

### Vì sao epoll tồn tại
`select`/`poll` quét lại toàn bộ fd đang theo dõi ở mỗi lần gọi. Chi phí là O(n) mỗi lần wake up, bất kể thực sự có bao nhiêu fd
sẵn sàng. Một proxy giữ hàng chục nghìn keep-alive connection đang idle sẽ phải trả giá này liên tục.

`epoll` giữ danh sách cần theo dõi ngay trong kernel (`epoll_create1`) và chỉ trả về các fd đã sẵn sàng. Chi phí tỷ lệ với số fd
*sẵn sàng*, không phải số fd *đang theo dõi*.

### Ba syscall
```rust
// epoll_create1(0)  -> epoll fd
// epoll_ctl(epfd, EPOLL_CTL_ADD/MOD/DEL, fd, &event) -> đăng ký/sửa/xóa
// epoll_wait(epfd, &mut events, max, timeout_ms)     -> block cho đến khi có fd sẵn sàng
use libc::{epoll_event, EPOLLIN, EPOLLOUT};

let mut ev = epoll_event { events: (EPOLLIN | EPOLLOUT) as u32, u64: fd as u64 };
```
Trường `u64` (hay `ptr` trong union) là sợi dây duy nhất nối event với câu hỏi "đây là connection nào". Trong thực tế, bạn đặt một
token/index vào đó, rồi tra state của connection trong một slab hoặc `Vec`.

### Level-triggered và edge-triggered
**Level-triggered** (mặc định): `epoll_wait` báo mỗi lần bạn gọi, miễn là fd *đang* readable/writable. Chế độ này an toàn, nhưng có
thể quay vòng (spin) nếu bạn không drain fd.

**Edge-triggered** (`EPOLLET`): chỉ báo ở *thời điểm chuyển trạng thái* sang readable/writable. Vì vậy bạn **phải** đọc/ghi trong
một vòng lặp cho đến khi nhận `EWOULDBLOCK`/`EAGAIN`. Nếu không, dữ liệu đến sau lần đọc dở dang cuối cùng sẽ bị bỏ lỡ, và fd sẽ
không bao giờ báo lại cho bạn nữa.

Gotcha: trộn giả định của level-triggered vào `EPOLLET` là nguồn gốc phổ biến nhất của bug "connection ngẫu nhiên treo mãi" trong
các event loop tự viết.

### Mẫu event loop
```
đăng ký listener với EPOLLIN
loop:
    n = epoll_wait(epfd, events, MAX, -1)
    với mỗi event sẵn sàng:
        nếu là listener -> accept() trong vòng lặp cho đến EAGAIN, đăng ký mỗi fd mới
        nếu không -> đọc/ghi trong vòng lặp cho đến EAGAIN, cập nhật state machine của connection đó
```
Mỗi fd sẵn sàng ứng với một state machine nhỏ (đang đọc header, đang đọc body, đang ghi response...). Bản thân event loop không có
logic protocol nào. Nó chỉ chạy state machine gắn với token của fd đó.

### Vì sao reactor của tokio tồn tại
Reactor của tokio chính là event loop này, được tổng quát hóa. Một thread (hoặc một pool nhỏ) sở hữu epoll fd. Mỗi `.await` trên
một socket đăng ký một waker gắn với một token, thay vì block. Xem [`03-rust/05-async.md`](../03-rust/05-async.md) và
[`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md). Hiểu epoll thô trước sẽ giúp bạn nắm rõ cách `Poll::Pending`/waker của
tokio hoạt động, vì đó cùng là mô hình đăng ký đó, chỉ có vòng lặp polling và phần sổ sách được che đi.

### Xử lý EAGAIN
Socket non-blocking trả `EWOULDBLOCK`/`EAGAIN` thay vì block, khi không có dữ liệu hoặc không còn chỗ trong buffer. Trong chế độ
edge-triggered, đây không phải lỗi. Nó là tín hiệu để thoát vòng lặp của bạn.

Quên đặt `O_NONBLOCK` trên socket đã accept (chỉ listener non-blocking là chưa đủ) là một bug kinh điển, âm thầm đưa bạn quay lại
blocking I/O cho từng connection.

## Practice
1. Trong một scratch project (không thuộc workspace này, vì không có lab riêng cho raw epoll), implement một listener non-blocking
   và đăng ký nó với `epoll_create1`/`epoll_ctl`.
2. Chạy echo server ở chế độ level-triggered trước. Xác nhận nó hoạt động dưới `nc` và một công cụ tạo tải đồng thời đơn giản.
3. Chuyển sang `EPOLLET` và tái hiện một connection bị kẹt bằng cách *không* lặp cho đến EAGAIN khi đọc. Quan sát hiện tượng treo,
   rồi sửa nó.
4. Thêm một fd đăng ký thứ hai (ví dụ một pipe dùng làm tín hiệu shutdown) và xử lý nó trong cùng vòng lặp.
5. So sánh số syscall bằng `strace -c` giữa phiên bản epoll của bạn và một phiên bản dùng `select` ngây thơ, dưới 1000 connection
   idle.
6. Xong rồi, đọc source reactor của tokio (`tokio::runtime::io`) và xác định nơi nó gọi cùng ba syscall mà bạn vừa dùng bằng tay.

---

<!-- nav -->
← Trước: [`02-linux/13-containers.md`](13-containers.md) · [Mục lục](00-README.md) · Tiếp theo: [`02-linux/15-io_uring.md`](15-io_uring.md) →
