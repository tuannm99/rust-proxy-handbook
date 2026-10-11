# Byte Stream, Packet và Connection

File này thuộc nhóm fundamentals. Xem [`01-network/01-fundamentals.md`](01-fundamentals.md) để có mục lục đầy đủ. Nó
nói về mental model quan trọng nhất cho [`labs/00-tcp-server`](../../labs/00-tcp-server) và
[`labs/01-http-parser`](../../labs/01-http-parser).

## What to learn

### Ảo giác byte stream
TCP đưa cho ứng dụng của bạn thứ *trông giống như* một dòng byte liên tục. Bạn gọi `read()` thì nhận byte, gọi
`write()` thì byte được gửi đi. Cảm giác như đang ghi vào một file.

Ở bên dưới, mạng không hoạt động như vậy. Dữ liệu thực ra di chuyển dưới dạng các **packet** rời rạc (ở tầng IP) mang
các **segment** TCP. Mỗi packet có header riêng, được route độc lập, và có thể đến theo thứ tự khác với lúc gửi. TCP
sắp xếp lại chúng cho đúng thứ tự trước khi đưa byte cho chương trình của bạn.

TCP giữ cho bạn hai điều: *thứ tự* và *tính đầy đủ*. Byte bạn đọc được đúng theo thứ tự chúng được ghi, và không thiếu
byte nào. TCP **không hứa gì về việc gom nhóm**. Giả sử bên gửi gọi `write()` một lần với 10.000 byte. Bên nhận có thể
thấy:
- một `read()` 10.000 byte, hoặc
- năm lần `read()`, mỗi lần 2.000 byte, hoặc
- một `read()` 3 byte rồi một `read()` 9.997 byte còn lại.

Mạng, buffer của kernel và thời điểm đều ảnh hưởng đến kết quả. Ứng dụng của bạn không kiểm soát được, và không được
giả định gì về nó.

Đây chính là lý do guide của `01-http-parser` đòi hỏi parse theo kiểu incremental (`ParseStatus::Partial`), thay vì
"đọc cả request trong một lần". Không hề có thứ gọi là "cả request trong một lần". Một `read()` chỉ trả về 3 byte của
một request dài 2.000 byte không phải bug hay edge case. Đó là hành vi TCP hoàn toàn bình thường, và code của bạn phải
xử lý nó mọi lúc, không chỉ vào những ngày xui xẻo.

### Packet, segment, datagram: cùng một ý tưởng, khác layer
Tên gọi thay đổi theo layer bạn đang nói đến. Bạn sẽ gặp cả ba, nên cần biết cái nào là cái nào:
- **Packet**: thuật ngữ chung, thường chỉ một IP packet, gồm một IP header cộng với bất cứ thứ gì layer trên đặt vào
  bên trong.
- **Segment**: cụ thể là một TCP packet. Nó gồm TCP header (sequence number, ack number, flags, window size) cộng một
  đoạn byte của stream.
- **Datagram**: một UDP packet. Nó gồm UDP header nhỏ hơn nhiều, cộng một message hoàn chỉnh, tự chứa. Khác với TCP
  segment, một UDP datagram không phải một phần của stream nào. Ứng dụng nhận nó nguyên vẹn hoặc không nhận gì.

### Connection-oriented và connectionless
TCP là **connection-oriented**. Trước khi dữ liệu nào được gửi, hai bên trao đổi một handshake (xem mục sau) để thống
nhất rằng cả hai đều sẵn sàng và có chung state. Sau đó, cả hai đầu theo dõi state của connection suốt vòng đời của nó:
sequence number, dữ liệu chưa được ack, kích thước buffer.

Chính state này làm cho reliability và thứ tự khả thi. Cũng chính nó làm cho một TCP connection trở thành một
*tài nguyên* thật, có state, trên cả hai máy. Xem [`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md) để biết
state đó tốn kém thế nào ở quy mô lớn, và [`02-addressing.md`](02-addressing.md) để biết 4-tuple định danh nó.

UDP là **connectionless**. Một datagram chỉ đơn giản được gửi đi: không có handshake, không có acknowledgment, không
đảm bảo thứ tự, không tự retransmit. Nếu bạn cần những tính chất đó trên UDP, application layer phải tự xây chúng.

Nghe có vẻ tệ hơn nhiều, và với một request/response thông thường thì đúng là vậy. Nhưng đó cũng là lý do **QUIC**
(transport bên dưới HTTP/3, xem [`01-network/18-http3.md`](18-http3.md)) được xây trên UDP thay vì TCP. Reliability
của TCP nằm trong kernel và áp dụng cho cả connection. Điều đó gây ra head-of-line blocking, vấn đề mà HTTP/2 vẫn phải
chịu khi multiplex nhiều stream ([`01-network/17-http2.md`](17-http2.md)). QUIC làm lại reliability *riêng cho từng
stream*, ở userspace, để tránh vấn đề này. Bạn không làm được điều đó trên TCP, vì đảm bảo thứ tự của TCP áp dụng cho cả
connection, không phải cho từng stream logic.

### Handshake: thống nhất state trước khi trao đổi dữ liệu
Một "handshake" là bất kỳ cuộc trao đổi nào mà hai bên dùng để thống nhất state chung trước khi dữ liệu thật được gửi.
Bạn sẽ gặp từ này ba lần trong thư mục này, mỗi lần là một ví dụ của cùng một ý tưởng:
- **3-way handshake của TCP** ([`12-tcp.md`](12-tcp.md)) thống nhất state của connection và sequence number ban đầu.
- **Handshake của TLS** ([`19-tls.md`](19-tls.md)) thống nhất encryption key, và protocol version/cipher sẽ dùng.
- **Handshake `Upgrade` của HTTP/1.1** ([`05-http-stack/10-websocket.md`](../05-http-stack/10-websocket.md)) thống nhất
  việc ngừng nói HTTP và chuyển sang nói một protocol khác trên cùng connection.

Khi nhận ra "đây là một handshake", bạn biết nên kỳ vọng gì: một cuộc trao đổi qua lại theo thứ tự cố định, một state
mà cả hai bên giờ phải thống nhất, và một kiểu lỗi trong đó một bên nghĩ handshake đã thành công còn bên kia thì không.
Rất nhiều bug thật nằm ở đúng chỗ này.

### Stateful và stateless, ở application layer
Sự khác biệt ở tầng connection phía trên (TCP có state, UDP thì không) có một phiên bản lặp lại ở application layer.

- Protocol **stateless** (HTTP/1.1 request/response thuần túy, xét ở mức ngữ nghĩa, xem
  [`01-network/15-http.md`](15-http.md)) xử lý mỗi request độc lập và không nhớ gì về các request trước.
- Tương tác **stateful** (một phiên WebSocket, một session đã xác thực được theo dõi qua cookie) đòi hỏi server phải nhớ
  điều gì đó giữa các lần trao đổi.

Một proxy load-balance request stateless có thể gửi mỗi request đi bất cứ đâu (round robin trong
[`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)). Một proxy đứng trước tương tác stateful cần
affinity (consistent hashing), hoặc state phải nằm ở một nơi dùng chung chứ không nằm riêng trên từng instance.

## Practice
1. Trong [`labs/00-tcp-server`](../../labs/00-tcp-server), viết một test client gửi payload 10.000 byte bằng đúng một
   lần gọi `write_all`, nhưng cho *server* đọc bằng buffer 256 byte và log xem cần bao nhiêu lần `read()` để nhận hết.
   Xác nhận con số đó không đúng bằng `10000 / 256`, và giải thích vì sao dựa vào mục byte stream ở trên.
2. Bắt một cuộc trao đổi UDP (ví dụ một DNS query: chạy `tcpdump -i lo udp port 53` trong khi chạy
   `dig @127.0.0.1 example.com` nhắm vào một resolver local, hoặc bất kỳ traffic UDP nào bạn tạo ra được) và một cuộc
   trao đổi TCP song song. Xác định handshake trong bản bắt TCP, và xác nhận không có handshake nào trong bản UDP.
3. Viết ra, mỗi ý một câu, state nào một TCP connection đang theo dõi mà một "connection" UDP thì không. ("Connection"
   UDP ở đây chỉ là một 4-tuple cố định mà bạn chọn dùng lại.)
4. Đọc mục 3-way handshake trong [`01-network/12-tcp.md`](12-tcp.md) và mục handshake trong
   [`01-network/19-tls.md`](19-tls.md) liền nhau. Liệt kê mỗi cái đang thống nhất điều gì, theo cách diễn giải của file
   này.
