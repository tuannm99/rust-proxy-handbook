# Recall và Review: Làm cho Kiến thức Networking Nhớ Lâu

Đọc một file một lần chỉ tạo ra *cảm giác* là mình hiểu, chưa tạo ra *trí nhớ*. Vì thế
bạn học một thứ vào thứ Hai thì đến thứ Tư nó đã biến mất. Trí nhớ được xây bằng
**retrieval**: gập trang lại, tự lôi câu trả lời ra từ đầu mình, rồi mới kiểm tra.

File này là bộ retrieval cho `01-network/`. Nó gồm: một bộ khung để treo các chi tiết
lên, câu hỏi cho từng file, hình để vẽ lại từ trí nhớ, và các thí nghiệm "dự đoán rồi
chạy". File này không dạy gì mới. Nó buộc bạn phải dùng những gì các file khác đã dạy.

## What to learn

### Cách dùng file này
1. **Che lại, trả lời, rồi kiểm tra.** Đọc một câu hỏi, trả lời *thành tiếng hoặc bằng
   chữ* mà không nhìn tài liệu, rồi mở section được link để so sánh. Phát hiện một câu
   sai ngay bây giờ có giá trị hơn nhiều so với một câu đúng mà bạn chỉ "nhận ra".
2. **Đừng đọc lại ngay.** Nếu không trả lời được, đừng lướt lại file. Hãy thử *suy ra*
   câu trả lời từ bản đồ layer ở dưới: "layer này đang giải vấn đề gì? Thiếu nó thì cái
   gì hỏng?" Sau đó mới kiểm tra. Thứ bạn tự suy ra được mới nhớ lâu.
3. **Giãn cách thời gian.** Ôn câu hỏi của một file sau khoảng 1 ngày kể từ lúc học, rồi
   3 ngày, 7 ngày, 21 ngày, sau đó hàng tháng. Mỗi lần ôn chỉ cần vài phút. Nếu một câu
   bạn sai hai lần, hãy ghi vào log của bạn (xem
   [`00-introduction/03-study-loop.md`](../00-introduction/03-study-loop.md)) và làm một
   flashcard.
4. **Dạy lại.** Giải thích "vòng đời request"
   ([`21-life-of-a-request.md`](21-life-of-a-request.md)) cho một con vịt cao su hoặc một
   người bạn, không dùng ghi chú. Chỗ nào bạn bị kẹt chính là lỗ hổng.
5. **Chạy thử.** Mỗi khái niệm dưới đây đều có một lệnh để tận mắt thấy nó. Một sự thật mà
   bạn đã *thấy trong `tcpdump`* sẽ nhớ lâu hơn nhiều so với một thứ bạn chỉ đọc.

### Bộ khung: mười hai sự thật cần giữ trong đầu
Mọi thứ khác trong thư mục này đều treo vào mười hai sự thật này. Nếu bạn nói được cả
mười hai và giải thích mỗi cái trong một câu, bạn đã nắm được cấu trúc.

1. **Layer** bọc dữ liệu trong các header lồng nhau: frame (tới hop kế tiếp) → packet
   (tới đúng host) → segment (tới đúng process) → message (tới đúng ý nghĩa).
   [`01-fundamentals.md`](01-fundamentals.md)
2. Layer 1–3 làm việc **hop-by-hop**. Layer 4 trở lên làm việc **end-to-end**, cho đến khi
   một proxy kết thúc connection và trở thành một đầu mới.
   [`01-fundamentals.md`](01-fundamentals.md)
3. Một connection được định danh bằng **4-tuple**. Một **socket** là một endpoint (IP +
   port), không phải là port. [`02-addressing.md`](02-addressing.md)
4. **NAT** viết lại địa chỉ/port và nhớ mapping. Nó chỉ hoạt động nếu reply quay về qua
   chính nó. [`02-addressing.md`](02-addressing.md), [`08-ip-and-icmp.md`](08-ip-and-icmp.md)
5. **TCP là byte stream**: không có ranh giới message, nên protocol của bạn cần tự làm
   framing. [`03-byte-streams.md`](03-byte-streams.md), [`12-tcp.md`](12-tcp.md)
6. Một TCP connection mới tốn **1 RTT** (handshake). TLS 1.3 tốn thêm **1 RTT**. Connection
   mới còn bắt đầu với window nhỏ. Vì vậy cần **reuse connection**.
   [`12-tcp.md`](12-tcp.md), [`13-tcp-reliability.md`](13-tcp-reliability.md), [`19-tls.md`](19-tls.md)
7. **Flow control** bảo vệ bên nhận, **congestion control** bảo vệ mạng. Lượng dữ liệu đang
   bay tối đa là `min(rwnd, cwnd)`. [`13-tcp-reliability.md`](13-tcp-reliability.md)
8. **Đóng connection bằng FIN (lịch sự) hoặc RST (hủy ngang).** Bên đóng trước giữ
   `TIME_WAIT`. Một đống `CLOSE_WAIT` là bug *của bạn*. [`12-tcp.md`](12-tcp.md)
9. **DNS** là một lần đi xuống cây phân cấp, có cache. Answer âm cũng được cache. Process
   chạy lâu phải re-resolve. [`14-dns.md`](14-dns.md)
10. **HTTP** là cách đóng khung stateless (dạng text hoặc binary) quanh method, status code
    và header. Một số header là **hop-by-hop**. [`15-http.md`](15-http.md),
    [`16-http1-wire-format.md`](16-http1-wire-format.md)
11. **TLS** = bất đối xứng để thỏa thuận key, đối xứng để mã hóa, certificate để chứng minh
    danh tính. **SNI** và **ALPN** cho phép proxy chọn certificate và protocol.
    [`06-crypto-basics.md`](06-crypto-basics.md), [`19-tls.md`](19-tls.md)
12. Một **proxy là hai connection**, mỗi cái có state và cách hỏng riêng. Bạn chỉ thấy hop
    của mình. [`21-life-of-a-request.md`](21-life-of-a-request.md)

### Ngân hàng câu hỏi, theo từng file
Trả lời mỗi câu mà không xem ghi chú, rồi kiểm tra theo link. Câu có dấu sao (*) là những
câu mọi người hay sai nhất.

**01 fundamentals** ([`01-fundamentals.md`](01-fundamentals.md))
- Kể tên bốn layer TCP/IP, đơn vị dữ liệu và loại địa chỉ ở mỗi layer.
- Router làm gì với Ethernet header của packet nó forward, và vì sao?
- Vì sao proxy thêm được retry và TLS mà router thì không? *

**02 addressing** ([`02-addressing.md`](02-addressing.md))
- Chính xác thì cái gì định danh một TCP connection? "Socket" có giống "port" không? *
- Một `/24` có bao nhiêu địa chỉ? Host xác định đích có nằm trong subnet của nó hay không
  bằng cách nào?
- NAT viết lại gì trên packet đi ra, và cái gì hỏng nếu reply đi đường khác?

**03 byte streams** ([`03-byte-streams.md`](03-byte-streams.md))
- Bên gửi gọi hai lần `write`, mỗi lần 100 byte. Liệt kê mọi chuỗi read mà bên nhận có thể
  thấy. *
- Nếu chọn UDP thay vì TCP, bạn được gì và mất gì?

**04 latency/throughput** ([`04-latency-throughput.md`](04-latency-throughput.md))
- Trong latency, bandwidth, throughput, RTT, cái nào quyết định chi phí của một handshake?
- Tính bandwidth-delay product của link 1 Gbit/s với RTT 40 ms. Nếu window nhỏ hơn con số
  đó thì chuyện gì xảy ra? *

**05 proxy taxonomy** ([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md))
- Forward proxy và reverse proxy: bên nào phải được cấu hình để biết proxy tồn tại?
- L7 proxy làm được gì mà L4 không làm được, và nó tốn thêm gì?

**06 crypto basics** ([`06-crypto-basics.md`](06-crypto-basics.md))
- Vì sao TLS dùng cả crypto bất đối xứng lẫn đối xứng?
- Certificate ràng buộc những thứ gì với nhau, và client kiểm tra những gì? *
- HMAC và digital signature: khi nào dùng cái nào?

**07 link layer** ([`07-link-layer.md`](07-link-layer.md))
- Một host gửi tới một IP nằm ngoài subnet. MAC đích là gì? Vì sao? *
- Switch học xem một MAC nằm ở port nào bằng cách nào, và làm gì với MAC chưa biết?
- Con số MSS 1460 đến từ đâu?

**08 IP và ICMP** ([`08-ip-and-icmp.md`](08-ip-and-icmp.md))
- Hai route cùng khớp một đích. Router chọn cái nào? Default route là gì?
- Với bit DF được set, một packet quá lớn so với một link. Chuyện gì xảy ra, và triệu chứng
  gì xuất hiện nếu ICMP bị chặn? *
- `traceroute` hoạt động thế nào nhờ TTL?

**09 UDP** ([`09-udp.md`](09-udp.md))
- UDP thêm gì lên trên IP? Chuyện gì xảy ra với datagram lớn hơn receive buffer của bạn?
- Vì sao amplification attack chạy được trên UDP, và QUIC hạn chế nó thế nào?

**10 tools** ([`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md))
- Giải mã dòng này: `Flags [P.], seq 1:79, ack 1, length 78`. Chuyện gì đã xảy ra?
- `Recv-Q`/`Send-Q` có nghĩa gì trên một socket *đang listen*, so với một socket
  *established*? *
- Field `curl -w` nào cho biết riêng thời gian TCP connect và TLS handshake?

**11 sockets** ([`11-socket.md`](11-socket.md))
- Liệt kê các syscall của server và của client theo thứ tự. Cái nào trả về một fd mới?
- Chuyện gì xảy ra khi listen backlog đầy, và bạn thấy nó ở đâu? *
- Phân biệt `SO_REUSEADDR` với `SO_REUSEPORT`; `shutdown(SHUT_WR)` với `close`.

**12 TCP** ([`12-tcp.md`](12-tcp.md))
- Một segment có `seq=1000` và 500 byte payload. Peer sẽ gửi `ack` bằng bao nhiêu?
- Số `CLOSE_WAIT` tăng dần và số `TIME_WAIT` tăng dần: mỗi cái là bug của ai, và sửa thế
  nào? *
- Vì sao đóng một socket khi còn dữ liệu chưa đọc lại gửi RST, và cái gì bị mất?
- Vì sao TCP keepalive mặc định gần như vô dụng khi đứng sau NAT?

**13 TCP reliability** ([`13-tcp-reliability.md`](13-tcp-reliability.md))
- RTO và fast retransmit: cái gì kích hoạt từng cái? Duplicate ACK là gì?
- Flow-control window và congestion window: mỗi cái bảo vệ cái gì, và cái nào đang giới hạn
  throughput lúc này?
- Vì sao một connection mới thì chậm, và slow-start-after-idle là gì? *
- Giải thích từng bước vì sao Nagle + delayed ACK gây stall.

**14 DNS** ([`14-dns.md`](14-dns.md))
- `getaddrinfo` tra theo thứ tự nào? Vì sao `ndots:5` làm tăng số query?
- Phân biệt `NXDOMAIN` với NODATA. Answer âm được cache bao lâu? *
- Cái gì *không* tự re-resolve khi DNS đổi? Phòng thủ DNS rebinding thế nào?

**15 HTTP** ([`15-http.md`](15-http.md))
- Kể tên bốn dạng của request target. Vì sao `Host` là bắt buộc?
- Phân biệt `302`, `307`, `308`; `304` nghĩa là gì?
- Header nào là hop-by-hop? Vì sao không nối `Set-Cookie` bằng dấu phẩy được? Vì sao phải
  coi `X-Forwarded-For` là thứ giả mạo được? *

**16 HTTP/1.1 wire format** ([`16-http1-wire-format.md`](16-http1-wire-format.md))
- Một request có cả `Content-Length` lẫn `Transfer-Encoding: chunked`. Bạn phải làm gì, và
  làm vậy ngăn được cuộc tấn công nào? *
- Một chunked body kết thúc như thế nào? Bạn biết body của request kết thúc ở đâu bằng cách
  nào?

**17 HTTP/2** ([`17-http2.md`](17-http2.md))
- Multiplexing giải quyết vấn đề nào của HTTP/1.1, và vấn đề nào vẫn còn do TCP?
- Vì sao state của HPACK là một mối nguy với proxy? Rapid Reset là gì?

**18 HTTP/3** ([`18-http3.md`](18-http3.md))
- Vì sao QUIC chạy trên UDP, và nó tránh head-of-line blocking thế nào?
- Connection ID dùng để làm gì? Client biết một server nói được HTTP/3 bằng cách nào?

**19 TLS** ([`19-tls.md`](19-tls.md))
- Trong ClientHello có gì giúp proxy chọn được certificate và protocol?
- Một TLS 1.3 handshake tốn bao nhiêu RTT, và cái gì giúp loại bỏ chúng ở những lần truy
  cập sau?

**20 PROXY protocol** ([`20-proxy-protocol.md`](20-proxy-protocol.md))
- Nó giải quyết bài toán gì mà `X-Forwarded-For` không giải quyết được? Lưu ý về mức độ tin
  cậy là gì? *

**21 life of a request** ([`21-life-of-a-request.md`](21-life-of-a-request.md))
- Kể bước 1–10 kèm số RTT. Có những connection nào, và ai sở hữu từng cái?
- "Connect bị treo rồi timeout" và "connection refused": mỗi cái đang cho bạn biết điều gì?
  *

### Vẽ lại từ trí nhớ
Vẽ lại từng hình dưới đây trên giấy trắng, rồi so với file. Nếu không vẽ được, nghĩa là
bạn chưa làm chủ nó.
1. Stack bốn layer, kèm đơn vị dữ liệu, địa chỉ và thiết bị ([`01-fundamentals.md`](01-fundamentals.md)).
2. Một frame với mọi header lồng nhau, ghi rõ ai đọc gì ([`07-link-layer.md`](07-link-layer.md)).
3. TCP state machine, với đường đi của client, đường đi của server và các đường đóng
   connection ([`12-tcp.md`](12-tcp.md)).
4. TCP handshake và một lần trao đổi dữ liệu với seq/ack cụ thể ([`12-tcp.md`](12-tcp.md)).
5. `cwnd` theo thời gian qua slow start, một lần mất packet, và giai đoạn hồi phục
   ([`13-tcp-reliability.md`](13-tcp-reliability.md)).
6. Chuỗi DNS resolution từ chương trình của bạn đến authoritative server ([`14-dns.md`](14-dns.md)).
7. TLS 1.3 handshake dưới dạng timeline ([`19-tls.md`](19-tls.md)).
8. Một client, một reverse proxy và một upstream, vẽ *cả hai* connection cùng mọi timeout
   ([`21-life-of-a-request.md`](21-life-of-a-request.md)).

### Dự đoán, rồi chạy
Với mỗi thí nghiệm: viết dự đoán ra trước, rồi chạy, và ghi mọi điều bất ngờ vào log của
bạn.
1. `ping -M do -s 1473 <host>`: lỗi gì, và vì sao?
2. `iptables ... -j DROP` so với `-j REJECT --reject-with tcp-reset` trên một port, rồi
   `curl` vào port đó. Mỗi cái hiện ra gì trong `curl -v` và `tcpdump`?
3. Đóng một socket khi còn dữ liệu chưa đọc: peer thấy FIN hay RST?
4. `tc qdisc add dev lo root netem delay 50ms`, rồi `curl -w`: field timing nào tăng, và
   tăng bao nhiêu?
5. `dig` một tên không tồn tại hai lần: lần hai có nhanh hơn không, và vì sao?
6. Mở 100 connection ngắn, không keep-alive: có bao nhiêu `TIME_WAIT`, ở phía nào?

### Gotcha: nhận ra không phải là nhớ lại
Cảm giác nguy hiểm nhất là nghĩ "à đúng rồi, tôi nhớ cái đó" ngay khi đọc câu trả lời. Đó
chỉ là *recognition* (nhận ra), rẻ và không đáng tin. Nếu bạn không tự tạo ra câu trả lời
trước khi nhìn đáp án, hãy tính là bạn đã sai.

## Practice
1. Hôm nay, gập file này lại và viết ra mười hai sự thật của bộ khung từ trí nhớ. Tự chấm
   điểm bằng cách đối chiếu với danh sách. Lặp lại sau 1, 3 và 7 ngày, và ghi điểm mỗi lần
   vào learning log của bạn.
2. Chọn năm câu có dấu sao mà bạn đã trả lời sai, và biến mỗi câu thành một flashcard (mặt
   trước là câu hỏi, mặt sau là câu trả lời một dòng kèm một link).
3. Dạy lại [`21-life-of-a-request.md`](21-life-of-a-request.md) thành tiếng, không ghi chú,
   trong lúc vẽ các hình số 1, 3 và 8 ở trên. Liệt kê mọi chỗ bạn ngập ngừng.
4. Chạy ba thí nghiệm trong mục "Dự đoán, rồi chạy" với
   [`labs/00-tcp-server`](../../labs/00-tcp-server) hoặc
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy). Ghi lại mỗi thí nghiệm dạy bạn
   điều gì mà việc chỉ đọc không dạy được.

---

<!-- nav -->
← Trước: [`01-network/21-life-of-a-request.md`](21-life-of-a-request.md) · [Mục lục](00-README.md)
