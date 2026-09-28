# Byte Streams, Packets, and Connections

Một phần của chuỗi fundamentals từ-con-số-0 — xem [`01-network/01-fundamentals.md`](01-fundamentals.md)
để có index đầy đủ. File này nói về mental model quan trọng nhất cho
[`labs/00-tcp-server`](../../labs/00-tcp-server) và [`labs/01-http-parser`](../../labs/01-http-parser).

## What to learn

### Ảo giác byte-stream
TCP đưa cho ứng dụng của bạn thứ *trông giống như* một stream byte liên
tục — bạn gọi `read()`, bạn nhận byte; bạn gọi `write()`, byte được gửi
đi. Cảm giác như đang ghi vào một file. Bên dưới, ở tầng mạng không có gì
như vậy cả: dữ liệu thực ra di chuyển dưới dạng các **packet** rời rạc (ở
tầng IP) mang các **segment** TCP, mỗi cái có header riêng, mỗi cái được
route độc lập, mỗi cái có thể đến theo thứ tự khác với thứ tự nó được gửi
(TCP sắp xếp lại chúng theo đúng thứ tự trước khi đưa byte cho chương
trình của bạn).

Ảo giác mà TCP duy trì là *thứ tự* và *tính đầy đủ* — byte bạn đọc được
theo đúng thứ tự chúng được ghi, và không thiếu byte nào. Nó **không hứa
hẹn gì về việc gom nhóm**. Nếu bên gửi gọi `write()` một lần với 10.000
byte, bên nhận có thể thấy nó đến như một `read()` duy nhất 10.000 byte,
hoặc năm lần `read()` mỗi lần 2.000, hoặc một `read()` 3 byte rồi một
`read()` 9.997 byte còn lại — mạng, buffer của kernel, và thời gian đều
ảnh hưởng đến việc này, và ứng dụng của bạn không kiểm soát được và không
được giả định bất cứ điều gì về nó.

Đây chính xác là lý do guide của `01-http-parser` khăng khăng đòi parsing
theo kiểu incremental (`ParseStatus::Partial`) thay vì "đọc cả request
trong một lần gọi" — không hề tồn tại thứ gọi là "cả request trong một
lần gọi". Một `read()` duy nhất trả về 3 byte của một request 2.000 byte
không phải là bug hay edge case; đó hoàn toàn là hành vi TCP bình thường
mà code của bạn phải xử lý mỗi lần, không chỉ vào những ngày xui xẻo.

### Packet, segment, datagram — cùng một ý tưởng, khác tầng
Các thuật ngữ khác nhau tùy vào tầng bạn đang nói tới, và đáng để biết cái
nào là cái nào vì bạn sẽ gặp cả ba:
- **Packet** — thuật ngữ chung, thường có nghĩa là một IP packet: một IP
  header cộng với bất kỳ thứ gì tầng trên đặt vào bên trong nó.
- **Segment** — cụ thể là một TCP packet: header của TCP (sequence
  number, ack number, flags, window size) cộng một đoạn byte của stream
  của bạn.
- **Datagram** — một UDP packet: header nhỏ hơn nhiều của UDP cộng một
  message hoàn chỉnh, tự chứa. Khác với một TCP segment, một UDP datagram
  không phải một phần của một stream — ứng dụng nhận nó nguyên vẹn hoặc
  không nhận gì cả.

### Connection-oriented vs connectionless
TCP là **connection-oriented**: trước khi bất kỳ dữ liệu nào chảy, cả hai
bên trao đổi một handshake (xem mục tiếp theo) để đồng ý rằng cả hai đều
sẵn sàng và đồng bộ trạng thái. Cả hai đầu sau đó theo dõi trạng thái của
kết nối đó trong suốt vòng đời của nó — sequence number, dữ liệu chưa được
ack, kích thước buffer. Chính trạng thái này làm cho reliability và thứ tự
trở nên khả thi, và cũng chính nó làm cho một kết nối TCP trở thành một
*tài nguyên* thật, có trạng thái, trên cả hai máy (xem
[`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md) để biết trạng thái đó tốn kém gì ở quy mô lớn,
và [`02-addressing.md`](02-addressing.md) để biết về 4-tuple định danh nó).

UDP là **connectionless**: một datagram cứ thế được gửi đi, không
handshake, không acknowledgment, không đảm bảo thứ tự, không tự động
retransmit. Nếu bạn cần những tính chất đó trên UDP, tầng ứng dụng của bạn
phải tự xây chúng. Nghe có vẻ tệ hơn hẳn, và với một request/response
thông thường thì đúng là vậy — nhưng đó cũng là lý do vì sao **QUIC**
(tầng transport bên dưới HTTP/3, xem [`01-network/13-http3.md`](13-http3.md)) được xây
trên UDP thay vì TCP: reliability trong kernel, một-kích-cỡ-cho-tất-cả của
TCP tạo ra head-of-line blocking mà HTTP/2 phải chịu ở tầng multiplexed
stream ([`01-network/12-http2.md`](12-http2.md)), và QUIC tái hiện thực reliability
*theo từng stream*, ở userspace, chính là để tránh điều đó — một thứ bạn
không thể làm trên nền TCP vì đảm bảo về thứ tự của TCP áp dụng cho cả
kết nối, không phải cho từng logical stream.

### Handshake: đồng ý về trạng thái trước khi trao đổi dữ liệu
Một "handshake" là bất kỳ cuộc trao đổi nào mà cả hai bên đồng ý về trạng
thái chung trước khi dữ liệu thật chảy — bạn sẽ gặp từ này ba lần riêng
biệt trong thư mục này, mỗi lần là một instance khác nhau của cùng một ý
tưởng:
- **3-way handshake của TCP** ([`08-tcp.md`](08-tcp.md)) đồng ý về trạng thái kết nối
  và sequence number ban đầu.
- **Handshake của TLS** ([`14-tls.md`](14-tls.md)) đồng ý về encryption key và
  protocol version/cipher nào sẽ dùng.
- **Handshake `Upgrade` của HTTP/1.1** ([`05-http-stack/10-websocket.md`](../05-http-stack/10-websocket.md))
  đồng ý dừng nói HTTP và bắt đầu nói một protocol khác trên cùng kết nối.

Nhận ra "đây là một handshake" cho bạn biết nên kỳ vọng gì: một cuộc trao
đổi qua lại cố định, trạng thái mà cả hai bên giờ phải đồng ý, và một
failure mode nơi một bên nghĩ handshake đã thành công còn bên kia thì
không (đây là nơi rất nhiều bug thật sự trú ngụ).

### Stateful vs stateless, ở tầng application
Sự khác biệt ở tầng kết nối phía trên (TCP theo dõi trạng thái, UDP thì
không) có một tiếng vọng ở tầng application đáng nêu riêng: các protocol
**stateless** (HTTP/1.1 request/response thuần túy, ở mức ngữ nghĩa — xem
[`01-network/10-http.md`](10-http.md)) xử lý mỗi request độc lập, không nhớ gì về những
request trước; các tương tác **stateful** (một phiên WebSocket, một
session đã xác thực theo dõi qua cookie) đòi hỏi server phải nhớ điều gì
đó giữa các lần trao đổi. Một proxy load-balance các request stateless có
thể gửi mỗi request đi bất cứ đâu (round robin của
[`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)); một proxy đứng trước các tương tác
stateful cần affinity (consistent hashing) hoặc trạng thái phải nằm ở một
nơi dùng chung, không phải trên một instance riêng lẻ.

## Practice
1. Trong [`labs/00-tcp-server`](../../labs/00-tcp-server), viết một test client gửi một payload
   10.000 byte trong một lệnh gọi `write_all` duy nhất, nhưng để *server*
   đọc bằng buffer 256 byte và log xem cần bao nhiêu lệnh gọi `read()` để
   nhận hết. Xác nhận con số đó không đúng bằng `10000 / 256`, và giải
   thích vì sao dựa vào mục byte-stream ở trên.
2. Capture một cuộc trao đổi UDP (ví dụ một DNS query: `tcpdump -i lo udp
   port 53` trong khi chạy `dig @127.0.0.1 example.com` nhắm vào một
   resolver local, hoặc bất kỳ traffic UDP nào bạn tạo được) và một cuộc
   trao đổi TCP song song; xác định handshake trong bản capture TCP và
   xác nhận không có handshake nào trong bản UDP.
3. Viết ra, mỗi ý một câu, trạng thái nào một kết nối TCP đang theo dõi
   mà một "kết nối" UDP (thực ra chỉ là một 4-tuple cố định bạn chọn tái
   sử dụng) thì không.
4. Đọc mục 3-way handshake trong [`01-network/08-tcp.md`](08-tcp.md) và mục handshake
   trong [`01-network/14-tls.md`](14-tls.md) liền nhau; liệt kê mỗi cái đang đồng ý về
   điều gì, dùng cách diễn giải từ file này.
