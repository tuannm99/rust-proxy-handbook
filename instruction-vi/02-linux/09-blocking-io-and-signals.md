# Blocking I/O, Event Loops, and Signals

Một phần của chuỗi fundamentals từ-con-số-0 — xem
[`02-linux/01-fundamentals.md`](01-fundamentals.md) để có index đầy đủ. Hai ý tưởng được gộp
lại vì cả hai đều xoay quanh cùng một câu hỏi nền: chương trình của bạn
làm sao biết được thứ nó đang chờ đã xảy ra?

## What to learn

### Syscall blocking vs non-blocking
Một syscall chưa có gì để làm — `read()` trên một socket chưa có dữ liệu
— có hai cách hành xử khả dĩ: **block** (thread gọi bị kernel tạm dừng
và không chạy lại cho tới khi dữ liệu tới — dễ viết, nhưng thread đó
không làm gì khác trong lúc đó) hoặc **non-block** (syscall trả về ngay
lập tức với một lỗi, `EWOULDBLOCK`/`EAGAIN`, nghĩa là "chưa có gì sẵn
sàng, hỏi lại sau").

```rust
// blocking: dòng này đơn giản là không return cho tới khi dữ liệu tới
let n = blocking_socket.read(&mut buf)?;

// non-blocking: return ngay lập tức dù thế nào
match nonblocking_socket.read(&mut buf) {
    Ok(n) => { /* nhận được n byte */ }
    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => { /* chưa có gì */ }
    Err(e) => return Err(e),
}
```

### Vì sao one-thread-per-connection không scale
Một thread blocking cho mỗi kết nối là thiết kế đơn giản nhất và không
scale — hàng ngàn kết nối keep-alive idle sẽ có nghĩa là hàng ngàn OS
thread phần lớn không làm gì, mỗi thread có memory stack riêng và overhead
lập lịch của kernel riêng ([`03-processes-and-threads.md`](03-processes-and-threads.md)). Giải pháp thay
thế là non-blocking socket cộng với một cơ chế để hỏi kernel "cho tôi
biết trong số một ngàn fd này cái nào thực sự có gì sẵn sàng" trong một
lời gọi, thay vì tự poll từng cái — cơ chế đó là `epoll`
([`02-linux/14-epoll.md`](14-epoll.md)), và nó là toàn bộ nền tảng mà reactor của tokio
được xây trên đó ([`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md)).

Đây là lý do cụ thể khiến tiêu chí hoàn thành của [`labs/00-tcp-server`](../../labs/00-tcp-server)
khăng khăng đòi xử lý 500+ kết nối mà không dùng 500+ thread: nó ép bạn
thực sự cảm nhận sự khác biệt mà phần này mô tả, chứ không chỉ đọc về nó.

### Mẫu event loop, một tầng cao hơn epoll
Dù bạn tự viết tay (bài tập trong [`02-linux/14-epoll.md`](14-epoll.md)) hay để tokio làm
hộ, hình dạng luôn là: đăng ký quan tâm tới một tập fd, block *một lần*
trên "cho tôi biết khi bất kỳ cái nào trong số này sẵn sàng" thay vì block
theo từng fd, và dispatch tới bất kỳ logic nào sở hữu fd đó khi việc chờ
trả về. Một thread (hoặc một pool nhỏ) có thể phục vụ hàng ngàn kết nối
theo cách này vì nó không bao giờ bị block chờ bất kỳ một kết nối cụ thể
nào — nó chỉ block, nhiều nhất, chờ *bất cứ thứ gì, thứ gì cũng được* trở
nên sẵn sàng.

### Signal: kernel ngắt process của bạn
Một **signal** là một thông báo kernel gửi tới một process một cách bất
đồng bộ — nó có thể tới giữa hai instruction bất kỳ, ngắt bất cứ thứ gì
process đang làm, không giống một syscall mà code của bạn chủ động khởi
xướng. `kill -TERM <pid>` và Ctrl-C đều hoạt động bằng cách gửi một
signal. Đây là một cơ chế gửi hoàn toàn khác so với một hàm return hay
kết quả syscall: chương trình của bạn không hỏi "có signal nào cho tôi
không?" — kernel đơn giản là preempt nó.

### Vì sao xử lý signal ngây thơ nguy hiểm
Tính bất đồng bộ này chính xác là lý do xử lý signal mong manh nếu làm
ngây thơ: một handler chạy "bất cứ lúc nào, bất kể process đang làm gì"
không thể an toàn làm hầu hết các việc bình thường (cấp phát memory, khóa
một mutex) vì nó có thể vừa ngắt đúng code đang làm chính việc đó — khóa
một mutex mà handler bây giờ cũng cố khóa sẽ làm process tự deadlock với
chính nó. Tập các thao tác an toàn để thực hiện bên trong một raw signal
handler được gọi là **async-signal-safe**, và đó là một danh sách ngắn
loại trừ hầu hết những gì cảm giác như "code bình thường."

Cách sửa idiomatic, và điều mọi async runtime nghiêm túc đều làm: raw
handler không làm gì ngoài ghi một byte vào một pipe/eventfd (hoặc tăng
một atomic — cả hai đều nằm trong danh sách async-signal-safe), và logic
reload/shutdown thật sự của bạn chạy sau đó, trên một thread bình thường,
được đánh thức bởi lần ghi đó qua cùng cơ chế event-loop mô tả ở trên.
`tokio::signal` implement chính xác mẫu này cho bạn; [`02-linux/17-signals.md`](17-signals.md)
bao quát các signal cụ thể (`SIGHUP`, `SIGTERM`) mà một proxy quan tâm và
API tương ứng.

### Sợi chỉ chung
Blocking I/O readiness và signal delivery đều là các biến thể của "chương
trình của tôi cần phản ứng với thứ gì đó bên ngoài, vào một thời điểm nó
không kiểm soát" — khác biệt chỉ nằm ở cơ chế (một syscall mà bạn ở vị
trí sẵn sàng thử lại, so với một sự gián đoạn tìm đến bạn bất kể bạn đang
ở đâu). Cả hai cuối cùng đều được dẫn, trong một chương trình async xây
tốt, qua cùng một event loop: epoll readiness event và signal-derived
pipe/eventfd write được dispatch bởi cùng một cơ chế, đó là lý do
`tokio::select!` có thể chờ một socket read và một signal trong cùng một
biểu thức mà không có gì đặc biệt xảy ra bên dưới.

## Practice
1. Viết một chương trình block trên `std::net::TcpStream::read` không có
   dữ liệu tới, và trong một terminal khác xác nhận (qua `ps` hoặc `top`)
   thread đang ngồi idle thay vì spin — rồi giải thích vì sao 500 thread
   như vậy sẽ là một chi phí tài nguyên thật dù không thread nào dùng CPU.
2. Trong một scratch project, đặt một socket non-blocking
   (`set_nonblocking(true)`) và gọi `read()` trên nó khi không có dữ liệu
   sẵn sàng — xác nhận bạn nhận `WouldBlock` ngay lập tức thay vì lời gọi
   bị treo.
3. Viết một binary nhỏ đăng ký `tokio::signal` handler cho `SIGHUP` và
   `SIGTERM` và chỉ in ra cái nào đã bắn; gửi cả hai bằng `kill -HUP <pid>`
   / `kill -TERM <pid>` và xác nhận cả hai đều bắt được mà không giết
   process — rồi so sánh với gửi `SIGKILL`, thứ bạn hoàn toàn không thể
   bắt được.
4. Đọc danh sách các hàm async-signal-safe (`man 7 signal-safety`) và xác
   định ít nhất hai thao tác trông rất bình thường (ví dụ `malloc`,
   `printf`) mà *không* nằm trong danh sách đó — giải thích, dựa trên ví
   dụ mutex ở trên, vì sao gọi một trong hai từ một raw handler là nguy
   hiểm.
