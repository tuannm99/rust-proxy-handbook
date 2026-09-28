# tokio

Async runtime bên dưới mọi thứ trong workspace này. Theo
[`reading-guide.md`](reading-guide.md) để có lộ trình đi qua source, kèm việc mỗi điểm dừng
nên đọc sau lab nào; các file notes liệt kê bên dưới là để bạn tự viết
trong lúc theo guide.

Một trong hai ngoại lệ đọc sớm so với mặc định "đọc sau [`proxy/`](../../../proxy)" của thư
mục này: đọc reactor của `tokio::runtime::io` ngay sau bài tập raw-epoll
([`02-linux/07-epoll.md`](../../02-linux/07-epoll.md), Practice bước 6), trong khi vòng lặp
`epoll_create1`/`ctl`/`wait` của riêng bạn vẫn còn mới — đó là sự so sánh
khiến nó dễ hiểu. Phần còn lại (scheduler, task system, cơ chế waker) đi
kèm với [`04-runtime/01-tokio.md`](../../04-runtime/01-tokio.md) và [`04-runtime/02-waker.md`](../../04-runtime/02-waker.md).

Các file dự kiến (xem [`19-reading-source/00-README.md`](../00-README.md) để biết template):

- `architecture.md`
- `request-flow.md`
- `memory.md`
- `interesting-code.md`
- `what-to-learn.md`
