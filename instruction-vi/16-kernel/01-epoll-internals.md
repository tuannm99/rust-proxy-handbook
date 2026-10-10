# epoll Internals

[`02-linux/14-epoll.md`](../02-linux/14-epoll.md) nói về việc dùng epoll từ phía ứng dụng, bao gồm cả
bug `EAGAIN` ở chế độ edge-triggered. File này nói về chuyện gì đang xảy ra
bên trong kernel khiến API của epoll có hình dạng như vậy — và khiến bug đó
có lý do để tồn tại.

## What to learn

### Hai cấu trúc: một cây cho việc đăng ký, một list cho việc sẵn sàng
Một instance `epoll` (`eventpoll` trong source kernel) giữ hai cấu trúc dữ
liệu tách biệt: một red-black tree chứa mọi file descriptor đang được theo
dõi, khóa theo fd, để `epoll_ctl(ADD/MOD/DEL)` chạy ở O(log n); và một
**ready list** — một linked list thuần chỉ chứa các fd hiện đang có sự kiện
chờ xử lý. `epoll_wait` chỉ làm mỗi việc rút cạn ready list: chi phí của nó
tỷ lệ với số fd *sẵn sàng*, không phải số fd *đang theo dõi*. Đây chính là
toàn bộ lý do epoll scale được ở nơi `select`/`poll` không làm được — hai
cái đó quét lại toàn bộ fd đang theo dõi ở mỗi lần gọi, O(n) bất kể có bao
nhiêu fd thực sự sẵn sàng.

### Đường đi của wakeup
Khi NIC nhận dữ liệu cho một socket, interrupt handler của driver chuyển
giao cho một softirq ([`16-kernel/04-interrupt.md`](04-interrupt.md)) đi ngược lên network
stack tới tầng socket. Socket có một wait queue chứa các callback được
đăng ký bởi bất cứ thứ gì đang chờ nó; epoll đăng ký một callback như vậy
khi bạn add fd. Việc duy nhất callback đó làm: thêm entry của fd này vào
ready list, và nếu có task đang block trong `epoll_wait`, đánh thức nó.
Không có việc match, filter, hay scan nào xảy ra trên đường đi này — đó là
một push O(1) trực tiếp vào ready list.

### Level-triggered vs edge-triggered, nhìn từ góc độ ready list
**Level-triggered** (mặc định): một fd vẫn nằm (hoặc được add lại) vào
ready list mỗi lần `epoll_wait` được gọi, miễn là nó vẫn thực sự sẵn sàng
(ví dụ receive buffer của socket vẫn còn byte chưa đọc).
**Edge-triggered** (`EPOLLET`): fd chỉ được add vào ready list tại thời
điểm *chuyển trạng thái* sang sẵn sàng — từ "không có gì để đọc" sang "có
gì đó để đọc". Nếu bạn không rút cạn socket tới `EAGAIN` ở lần thông báo
đó, sẽ không có chuyển trạng thái mới nào xảy ra (socket đã sẵn sàng từ
trước, nó vẫn sẵn sàng, nhưng không có gì kích hoạt lại), và epoll sẽ
không bao giờ báo cho bạn nữa dù dữ liệu chưa đọc vẫn đang nằm đó. Đây
chính xác là cơ chế đứng sau bug missed-wakeup mà bài tập ở
[`02-linux/14-epoll.md`](../02-linux/14-epoll.md) bắt bạn tự tái hiện — giờ nhìn dưới góc độ *vì sao*
kernel hành xử như vậy thay vì chỉ quan sát triệu chứng.

### `EPOLLEXCLUSIVE` và thundering herd
Nhiều thread hoặc process có thể chia sẻ một listening socket và mỗi cái
đăng ký nó với epoll instance riêng của mình. Không có `EPOLLEXCLUSIVE`,
tất cả bọn chúng đều wake trên cùng một kết nối đến và đua nhau `accept()`
— tất cả trừ một cái sẽ nhận `EAGAIN`, và mọi wakeup sau cái đầu tiên đều
là CPU lãng phí. `EPOLLEXCLUSIVE` (Linux 4.5+) bảo kernel chỉ đánh thức một
waiter mỗi sự kiện, điều này quan trọng khi một proxy chạy nhiều accept
loop (một cho mỗi worker thread, hoặc `SO_REUSEPORT` với nhiều listening
socket) trên cùng một địa chỉ.

## Practice
1. Đọc `/proc/<pid>/fdinfo/<epfd>` của một [`labs/00-tcp-server`](../../labs/00-tcp-server) đang chạy
   để xem các fd đã đăng ký và event mask của chúng — xác nhận nó khớp với
   những gì code bạn thực sự đã đăng ký.
2. Tái hiện lại bug missed-wakeup ở chế độ edge-triggered từ
   [`02-linux/14-epoll.md`](../02-linux/14-epoll.md), nhưng lần này giải thích cách fix theo cơ chế
   ready-list ở trên: vì sao rút cạn tới `EAGAIN` mới là thứ tạo ra
   transition kế tiếp, chứ không chỉ là "làm đúng theo docs".
3. Chạy nhiều accept-loop thread trên cùng một `SO_REUSEPORT` listener mà
   không có `EPOLLEXCLUSIVE`, đếm số wakeup `EAGAIN` lãng phí dưới một đợt
   kết nối đến dồn dập; thêm `EPOLLEXCLUSIVE` và so sánh.
4. Trace (bằng `strace -e epoll_wait,epoll_ctl` hoặc tương tự) một chương
   trình `tokio` đang chạy trong [`labs/00-tcp-server`](../../labs/00-tcp-server) và đối chiếu những gì
   bạn thấy với mô hình tree/ready-list ở trên.
