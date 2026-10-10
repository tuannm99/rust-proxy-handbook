# Recall và Review: Làm cho Networking Nhớ Lâu

Đọc một file một lần tạo ra *cảm giác* hiểu, không phải *trí nhớ* về nó — vì thế
bạn học một thứ vào thứ Hai và đến thứ Tư nó đã biến mất. Trí nhớ được xây bằng
**retrieval**: gập trang lại và tự kéo câu trả lời ra từ đầu mình, rồi kiểm tra.
File này là bộ retrieval cho `01-network/`: một bộ khung để treo chi tiết lên, câu
hỏi cho từng file, hình vẽ để tái tạo từ trí nhớ, và các thí nghiệm dự đoán-rồi-chạy.
Nó không dạy gì mới; nó bắt bạn dùng những gì các file khác đã dạy.

## What to learn

### Cách dùng file này (phương pháp)
1. **Che, trả lời, kiểm tra.** Đọc một câu hỏi, trả lời *thành tiếng hoặc bằng
   chữ* mà không nhìn, rồi mở section được link và so sánh. Một câu sai phát hiện
   bây giờ đáng giá hơn một câu đúng mà bạn chỉ nhận ra.
2. **Đừng đọc lại trước.** Nếu không trả lời được, đừng lướt file lần nữa — trước
   hết thử *suy ra* nó từ bản đồ layer bên dưới ("layer này đang giải vấn đề gì?
   thiếu nó thì cái gì hỏng?"), rồi mới kiểm tra. Suy ra mới là thứ sống sót.
3. **Giãn cách.** Ôn câu hỏi của một file khoảng 1 ngày sau khi học, rồi 3 ngày,
   7 ngày, 21 ngày, rồi hàng tháng. Mỗi lần ôn chỉ vài phút. Sai một câu hai lần?
   Ghi vào log của bạn (xem [`00-introduction/03-study-loop.md`](../00-introduction/03-study-loop.md)) và làm một flashcard.
4. **Dạy lại.** Giải thích "vòng đời request" ([`21-life-of-a-request.md`](21-life-of-a-request.md)) cho một con vịt
   cao su hoặc một người bạn mà không có ghi chú. Chỗ bạn khựng là lỗ hổng.
5. **Chạy nó.** Mỗi khái niệm dưới đây đều có một lệnh cho thấy nó. Một sự thật bạn
   đã *thấy trong `tcpdump`* được nhớ lâu hơn nhiều so với thứ bạn đọc.

### Bộ khung: mười hai sự thật để giữ trong đầu
Mọi thứ khác trong thư mục này treo vào những cái này. Nếu bạn nói được cả mười hai
và giải thích mỗi cái trong một câu, bạn nắm được cấu trúc.

1. **Layer** bọc dữ liệu trong các header lồng nhau: frame (hop kế) > packet (đúng
   host) > segment (đúng process) > message (đúng ý nghĩa). [`01-fundamentals.md`](01-fundamentals.md)
2. Layer 1–3 là **hop-by-hop**; 4+ là **end-to-end** — cho tới khi một proxy kết thúc
   connection và trở thành một đầu mới. [`01-fundamentals.md`](01-fundamentals.md)
3. Một connection được định danh bởi **4-tuple**; một **socket** là một endpoint (IP +
   port), không phải port. [`02-addressing.md`](02-addressing.md)
4. **NAT** viết lại địa chỉ/port và nhớ mapping; nó chỉ chạy nếu reply quay về qua nó.
   [`02-addressing.md`](02-addressing.md), [`08-ip-and-icmp.md`](08-ip-and-icmp.md)
5. **TCP là byte stream**: không có ranh giới message — protocol của bạn cần framing.
   [`03-byte-streams.md`](03-byte-streams.md), [`12-tcp.md`](12-tcp.md)
6. Một TCP connection mới tốn **1 RTT** (handshake), TLS 1.3 thêm **1 RTT**, và một
   connection mới bắt đầu với window nhỏ — vì thế phải **tái sử dụng connection**.
   [`12-tcp.md`](12-tcp.md), [`13-tcp-reliability.md`](13-tcp-reliability.md), [`19-tls.md`](19-tls.md)
7. **Flow control** bảo vệ bên nhận, **congestion control** bảo vệ mạng; gửi tối đa
   `min(rwnd, cwnd)` đang bay. [`13-tcp-reliability.md`](13-tcp-reliability.md)
8. **Đóng là FIN (lịch sự) hoặc RST (hủy)**; bên đóng giữ `TIME_WAIT`; một đống
   `CLOSE_WAIT` là bug *của bạn*. [`12-tcp.md`](12-tcp.md)
9. **DNS** là một lần đi cây có cache; câu trả lời âm cũng được cache; process sống
   lâu phải re-resolve. [`14-dns.md`](14-dns.md)
10. **HTTP** là framing stateless dạng text/binary quanh method, status code và header;
    vài header là **hop-by-hop**. [`15-http.md`](15-http.md), [`16-http1-wire-format.md`](16-http1-wire-format.md)
11. **TLS** = bất đối xứng để thỏa thuận key, đối xứng để mã hóa, certificate để chứng
    minh danh tính; **SNI** và **ALPN** cho proxy chọn cert và protocol.
    [`06-crypto-basics.md`](06-crypto-basics.md), [`19-tls.md`](19-tls.md)
12. Một **proxy là hai connection** với state và failure mode riêng; bạn chỉ thấy hop
    của mình. [`21-life-of-a-request.md`](21-life-of-a-request.md)

### Ngân hàng câu hỏi, theo từng file
Trả lời mỗi câu không ghi chú, rồi kiểm tra theo mũi tên. Câu có dấu sao (*) là
những câu mọi người hay sai nhất.

**01 fundamentals** ([`01-fundamentals.md`](01-fundamentals.md))
- Kể tên bốn layer TCP/IP, đơn vị và địa chỉ dùng ở mỗi layer.
- Router làm gì với Ethernet header của packet nó forward, và vì sao?
- Vì sao proxy thêm được retry và TLS mà router không thể? *

**02 addressing** ([`02-addressing.md`](02-addressing.md))
- Chính xác cái gì định danh một TCP connection? "Socket" có giống "port" không? *
- Một `/24` có bao nhiêu địa chỉ? Host quyết định đích nằm trong subnet của nó thế nào?
- NAT viết lại gì trên packet đi ra, và cái gì hỏng nếu reply đi đường khác?

**03 byte streams** ([`03-byte-streams.md`](03-byte-streams.md))
- Bên gửi làm hai `write` 100 byte. Liệt kê mọi chuỗi read mà bên nhận có thể thấy. *
- Chọn UDP thay TCP, bạn được và mất gì?

**04 latency/throughput** ([`04-latency-throughput.md`](04-latency-throughput.md))
- Cái nào trong latency, bandwidth, throughput, RTT quyết định chi phí của một handshake?
- Tính bandwidth-delay product của 1 Gbit/s ở 40 ms RTT. Nếu window nhỏ hơn thì sao? *

**05 proxy taxonomy** ([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md))
- Forward vs reverse proxy: ai được cấu hình để biết proxy tồn tại?
- L7 proxy làm được gì mà L4 không, và nó tốn gì?

**06 crypto basics** ([`06-crypto-basics.md`](06-crypto-basics.md))
- Vì sao TLS dùng cả crypto bất đối xứng lẫn đối xứng?
- Certificate ràng buộc những gì với nhau, và client kiểm tra gì? *
- HMAC vs digital signature: khi nào dùng cái nào?

**07 link layer** ([`07-link-layer.md`](07-link-layer.md))
- Một host gửi tới một IP ngoài subnet. MAC đích là gì? Vì sao? *
- Switch học một MAC nằm đâu như thế nào, và làm gì với MAC chưa biết?
- MSS 1460 đến từ đâu?

**08 IP and ICMP** ([`08-ip-and-icmp.md`](08-ip-and-icmp.md))
- Hai route cùng khớp một đích. Router chọn cái nào? Default route là gì?
- Với DF được set, một packet quá lớn cho một link. Chuyện gì xảy ra, và triệu chứng gì
  hiện ra nếu ICMP bị chặn? *
- `traceroute` hoạt động thế nào bằng TTL?

**09 UDP** ([`09-udp.md`](09-udp.md))
- UDP thêm gì lên trên IP? Chuyện gì xảy ra với datagram lớn hơn receive buffer của bạn?
- Vì sao amplification attack chạy được trên UDP, và QUIC hạn chế chúng ra sao?

**10 tools** ([`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md))
- Giải mã: `Flags [P.], seq 1:79, ack 1, length 78`. Chuyện gì đã xảy ra?
- `Recv-Q`/`Send-Q` nghĩa gì trên một socket *đang listen* vs một socket *established*? *
- Field `curl -w` nào tách riêng thời gian TCP-connect và TLS-handshake?

**11 sockets** ([`11-socket.md`](11-socket.md))
- Liệt kê syscall của server và client theo thứ tự. Cái nào trả về một fd mới?
- Chuyện gì xảy ra khi listen backlog đầy, và bạn thấy nó ở đâu? *
- `SO_REUSEADDR` vs `SO_REUSEPORT`; `shutdown(SHUT_WR)` vs `close`.

**12 TCP** ([`12-tcp.md`](12-tcp.md))
- Một segment có `seq=1000` và 500 byte payload. Peer gửi `ack` bao nhiêu?
- Một đống `CLOSE_WAIT` tăng vs một đống `TIME_WAIT` tăng: bug của ai, và cách sửa? *
- Vì sao đóng một socket còn dữ liệu chưa đọc lại gửi RST, và cái gì bị mất?
- Vì sao TCP keepalive mặc định vô dụng khi đứng sau NAT?

**13 TCP reliability** ([`13-tcp-reliability.md`](13-tcp-reliability.md))
- RTO vs fast retransmit: cái gì kích hoạt mỗi cái? Duplicate ACK là gì?
- Flow-control window vs congestion window: mỗi cái bảo vệ gì, và cái nào đang giới hạn
  throughput lúc này?
- Vì sao một connection mới chậm, và slow-start-after-idle là gì? *
- Giải thích cú khựng Nagle + delayed-ACK từng bước.

**14 DNS** ([`14-dns.md`](14-dns.md))
- `getaddrinfo` tra theo thứ tự nào? Vì sao `ndots:5` thêm query?
- `NXDOMAIN` vs NODATA. Câu trả lời âm được cache bao lâu? *
- Cái gì *không* re-resolve khi DNS đổi? Phòng thủ DNS rebinding thế nào?

**15 HTTP** ([`15-http.md`](15-http.md))
- Kể tên bốn dạng request-target. Vì sao `Host` bắt buộc?
- `302` vs `307` vs `308`; `304` nghĩa là gì?
- Header nào là hop-by-hop? Vì sao `Set-Cookie` không nối bằng dấu phẩy được? Vì sao
  `X-Forwarded-For` phải được coi là giả mạo được? *

**16 HTTP/1.1 wire format** ([`16-http1-wire-format.md`](16-http1-wire-format.md))
- Một request có cả `Content-Length` và `Transfer-Encoding: chunked`. Bạn phải làm gì, và
  điều đó chặn tấn công nào? *
- Một chunked body kết thúc thế nào? Bạn biết body của request kết thúc ở đâu bằng cách nào?

**17 HTTP/2** ([`17-http2.md`](17-http2.md))
- Multiplexing giải vấn đề nào của HTTP/1.1, và vấn đề nào còn lại vì TCP?
- Vì sao state của HPACK là mối nguy với proxy? Rapid Reset là gì?

**18 HTTP/3** ([`18-http3.md`](18-http3.md))
- Vì sao QUIC chạy trên UDP, và nó tránh head-of-line blocking thế nào?
- Connection ID để làm gì? Client biết một server nói HTTP/3 bằng cách nào?

**19 TLS** ([`19-tls.md`](19-tls.md))
- Cái gì trong ClientHello cho proxy chọn certificate và protocol?
- Một TLS 1.3 handshake tốn bao nhiêu RTT, và cái gì loại bỏ chúng cho lượt truy cập lặp lại?

**20 PROXY protocol** ([`20-proxy-protocol.md`](20-proxy-protocol.md))
- Nó giải bài toán gì mà `X-Forwarded-For` không giải được? Lưu ý về niềm tin là gì? *

**21 life of a request** ([`21-life-of-a-request.md`](21-life-of-a-request.md))
- Kể bước 1–10 kèm số RTT. Có những connection nào, và ai sở hữu từng cái?
- "Connect treo rồi timeout" vs "connection refused": mỗi cái đang nói gì với bạn? *

### Vẽ từ trí nhớ
Vẽ lại từng hình trên giấy trắng, rồi so với file. Nếu không vẽ được, bạn chưa làm
chủ nó.
1. Stack bốn layer với đơn vị, địa chỉ, thiết bị ([`01-fundamentals.md`](01-fundamentals.md)).
2. Một frame với mọi header lồng nhau, ghi rõ ai đọc gì ([`07-link-layer.md`](07-link-layer.md)).
3. TCP state machine, với đường client, đường server và các đường đóng ([`12-tcp.md`](12-tcp.md)).
4. TCP handshake và một lần trao đổi data với seq/ack thật ([`12-tcp.md`](12-tcp.md)).
5. cwnd theo thời gian qua slow start, một lần mất packet, và hồi phục ([`13-tcp-reliability.md`](13-tcp-reliability.md)).
6. Chuỗi DNS resolution từ chương trình của bạn tới authoritative server ([`14-dns.md`](14-dns.md)).
7. TLS 1.3 handshake dưới dạng timeline ([`19-tls.md`](19-tls.md)).
8. Một client, một reverse proxy và một upstream với *cả hai* connection và mọi timeout
   ([`21-life-of-a-request.md`](21-life-of-a-request.md)).

### Dự đoán, rồi chạy
Với mỗi cái: viết dự đoán trước, chạy, và ghi mọi điều bất ngờ vào log của bạn.
1. `ping -M do -s 1473 <host>` — lỗi gì, và vì sao?
2. `iptables ... -j DROP` vs `-j REJECT --reject-with tcp-reset` trên một port, rồi `curl` nó — mỗi cái hiện gì trong `curl -v` và `tcpdump`?
3. Đóng một socket còn dữ liệu chưa đọc — peer thấy FIN hay RST?
4. `tc qdisc add dev lo root netem delay 50ms`, rồi `curl -w` — field timing nào tăng, và bao nhiêu?
5. `dig` một tên không tồn tại hai lần — câu trả lời lần hai có nhanh hơn không, và vì sao?
6. Mở 100 connection ngắn không keep-alive — bao nhiêu `TIME_WAIT`, ở bên nào?

### Gotcha: nhận ra không phải là nhớ lại
Cảm giác nguy hiểm nhất là "à đúng rồi, tôi nhớ cái đó" trong lúc đọc câu trả lời.
Đó là *recognition*, rẻ và không đáng tin. Nếu bạn không tự tạo ra câu trả lời trước
khi nhìn, hãy tính là đã sai.

## Practice
1. Hôm nay, gập file này lại và viết ra mười hai sự thật của bộ khung từ trí nhớ; tự
   chấm điểm so với danh sách, rồi lặp lại sau 1, 3 và 7 ngày và ghi điểm mỗi lần
   vào learning log của bạn.
2. Chọn năm câu có dấu sao bạn đã sai và biến mỗi câu thành một flashcard (câu hỏi
   một mặt, câu trả lời một dòng và một link ở mặt kia).
3. Dạy [`21-life-of-a-request.md`](21-life-of-a-request.md) thành tiếng không ghi chú trong khi vẽ các mục 1, 3 và 8
   ở trên, và liệt kê mọi chỗ bạn ngập ngừng.
4. Chạy ba thí nghiệm "Dự đoán, rồi chạy" với [`labs/00-tcp-server`](../../labs/00-tcp-server) hoặc
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) và ghi lại mỗi cái dạy bạn điều gì mà việc đọc không dạy.
