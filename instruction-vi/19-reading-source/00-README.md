# Reading Source

Nghiên cứu có cấu trúc các dự án Rust networking production và các proxy
viết bằng C đã đặt nền móng cho các khái niệm mà handbook này dạy. Mỗi
subfolder là một dự án; đọc source của nó với một lăng kính cụ thể thay vì
lướt qua ngẫu nhiên.

## Trạng thái: đã có reading guide; notes cố tình để bạn tự viết

[`tokio/`](tokio), [`hyper/`](hyper), và [`pingora/`](pingora) mỗi thư mục có một `reading-guide.md`:
một lộ trình đi qua source kèm các câu hỏi cần trả lời, nhưng không có câu
trả lời. Các file notes theo từng dự án trong template bên dưới vẫn chưa
được viết, có chủ đích — đó là thứ *bạn* viết khi theo một guide, và một
bộ notes viết sẵn cho bạn sẽ không dạy được gì. Năm subfolder còn lại vẫn
là stub; chúng ít ưu tiên hơn với lộ trình của handbook này.

[`contributing-upstream.md`](contributing-upstream.md) nói về bước sau khi đọc: biến những gì đã
học thành đóng góp cho các dự án này.

Việc sắp xếp thứ tự bên dưới vẫn là điểm mấu chốt.

Đọc connection pool của [`pingora`](pingora) trước khi bạn tự viết một cái gần như
chẳng dạy bạn được gì: bạn không có thiết kế nào của riêng mình để so
sánh, nên mọi quyết định đọc lên đều có vẻ tùy tiện. Đọc nó *sau khi*
[`proxy/`](../../proxy) chạy được, và cùng đoạn code đó trở thành một bài bình luận sống
động về những lựa chọn bạn đã phải tự đưa ra — kể cả những lựa chọn bạn đã
làm sai. Sự đối chiếu đó chính là toàn bộ giá trị của thư mục này.

Có hai ngoại lệ đáng đọc sớm, và cả hai đã được trỏ tới ngay từ nơi chúng
quan trọng: reactor của tokio ngay sau bài tập raw-epoll
([`02-linux/07-epoll.md`](../02-linux/07-epoll.md), Practice bước 6) và codec `h1` của [`hyper`](hyper) ngay
sau khi bạn tự viết parser của mình
([`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md), Practice bước 8). Hai cái đó hiệu quả khi
đọc sớm chính vì bạn vừa mới tự xây thứ đang được đem ra so sánh.

Khi nào viết các file này: như ghi chú cho chính bạn trong lúc đọc, sau
phase 10 trong [`00-introduction/01-learning-roadmap.md`](../00-introduction/01-learning-roadmap.md).

## Template (cho mỗi dự án)

Mỗi subfolder dự án được dự kiến sẽ chứa:

- `architecture.md` — các thành phần chính và cách chúng khớp với nhau
- `request-flow.md` — trace một request từ đầu tới cuối qua source
- `memory.md` — dự án quản lý bộ nhớ/buffer trên hot path thế nào
- `interesting-code.md` — các hàm/file cụ thể đáng đọc kỹ, kèm lý do
- `what-to-learn.md` — các topic trong handbook (theo đường dẫn) mà dự án
  này là ví dụ thực tế tốt nhất

## Các dự án

- [`nginx/`](nginx) — kiến trúc proxy L7 tham chiếu (master/worker, event loop)
- [`envoy/`](envoy) — proxy C++ hiện đại, config động xDS, thiết kế
  observability-first
- [`haproxy/`](haproxy) — load balancer L4/L7 đã qua thử lửa, event loop tối thiểu
  allocation
- [`pingora/`](pingora) — framework proxy bằng Rust của Cloudflare, tương đồng thực
  tế gần nhất với [`proxy/`](../../proxy)
- [`hyper/`](hyper) — thư viện HTTP mà [`labs/02-http-server`](../../labs/02-http-server) trở đi được xây trên
  đó
- [`tokio/`](tokio) — async runtime bên dưới mọi thứ trong workspace này
- [`mio/`](mio) — lớp trừu tượng epoll/kqueue bên dưới tokio
- [`quinn/`](quinn) — implementation QUIC/HTTP-3, liên quan khi
  [`01-network/12-http3.md`](../01-network/12-http3.md) nằm trong phạm vi
