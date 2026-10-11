# Network

Phase 1 của lộ trình học. Đây là các protocol mà một proxy phải nói, từ socket lên đến
HTTP/3. Hãy đọc thư mục này trước [`02-linux/`](../02-linux) và
[`05-http-stack/`](../05-http-stack), vì hai phần đó cần bạn đã hiểu một connection và
một request thực sự là gì.

## Cách đọc thư mục này

Có hai cách dùng, tùy bạn đang ở đâu:

- **Chưa có nền tảng về networking:** đọc mọi file dưới đây theo thứ tự, từ
  [`01-fundamentals.md`](01-fundamentals.md) đến
  [`21-life-of-a-request.md`](21-life-of-a-request.md). Làm `## Practice` của mỗi file
  trước khi sang file tiếp theo. Hãy coi cả thư mục là một tutorial liền mạch. Các file
  sau cần bạn đã nắm các file trước, nên đừng nhảy cóc, kể cả khi tên file nghe quen.
  File cuối, [`21-life-of-a-request.md`](21-life-of-a-request.md), nối mọi thứ lại với
  nhau và là bài kiểm tra xem phần còn lại đã ngấm chưa. Nếu bạn kể lại được nó mà không
  có chỗ hổng nào, tức là bạn đã làm chủ tài liệu.
- **Đã quen với socket, TCP và HTTP:** dùng thư mục này như tài liệu tra cứu. Nhảy thẳng
  tới file nào lấp đúng lỗ hổng của bạn, theo thứ tự nào cũng được. Nhóm Fundamentals
  bên dưới là bài học từ con số 0 mà có lẽ bạn không cần. Nhóm "Bên dưới socket" cũng
  vậy, ngoại trừ [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) nếu
  bạn chưa đọc packet capture thành thạo.
  [`05-proxy-taxonomy.md`](05-proxy-taxonomy.md) và
  [`06-crypto-basics.md`](06-crypto-basics.md) vẫn đáng đọc dù nền tảng của bạn mạnh. Lý
  do là chúng thể hiện cách nhìn riêng của handbook này: [`proxy/`](../../proxy) nằm ở
  đâu trong bức tranh, và vốn từ về crypto mà [`19-tls.md`](19-tls.md) cần bạn đã biết.
  Đó không phải kiến thức networking chung mà bạn đã có sẵn từ nơi khác.

## Files

**Fundamentals** (bắt đầu từ đây nếu "port", "packet", "handshake", "NAT" hay
"certificate" chưa có nghĩa chính xác với bạn. Mọi file bên dưới đều xây trên những gì
nhóm này dạy):
- [`01-fundamentals.md`](01-fundamentals.md): mô hình phân layer, mô hình client-server, và mục lục dẫn tới năm file bên dưới
- [`02-addressing.md`](02-addressing.md): IP address, port, CIDR notation, NAT (SNAT/DNAT/CGNAT), kiến thức routing cơ bản
- [`03-byte-streams.md`](03-byte-streams.md): ảo giác byte stream, packet/segment/datagram, TCP so với UDP, handshake như một pattern
- [`04-latency-throughput.md`](04-latency-throughput.md): latency, bandwidth, throughput, RTT, bandwidth-delay product
- [`05-proxy-taxonomy.md`](05-proxy-taxonomy.md): forward proxy và reverse proxy, L4 và L7, NAT gateway, API gateway, CDN, sidecar. [`proxy/`](../../proxy) nằm ở đâu trong bức tranh này
- [`06-crypto-basics.md`](06-crypto-basics.md): mã hóa symmetric/asymmetric, hashing, HMAC, digital signature, certificate/PKI. Đây là kiến thức nền mà [`19-tls.md`](19-tls.md) và các file về identity trong [`07-security/`](../07-security) cần bạn đã biết

**Bên dưới socket** (các layer dưới TCP: về mặt vật lý, một packet đi từ A đến B như thế
nào, và làm sao để *nhìn thấy* nó. Hãy đọc nhóm này trước phần protocol, vì mọi Practice
bên dưới đều dùng các tool này):
- [`07-link-layer.md`](07-link-layer.md): encapsulation, Ethernet, MAC, switch, ARP, MTU, VLAN/bridge/veth
- [`08-ip-and-icmp.md`](08-ip-and-icmp.md): IPv4 header, TTL, routing theo longest-prefix, fragmentation và PMTUD, ICMP, IPv6
- [`09-udp.md`](09-udp.md): datagram so với stream, những thứ UDP không cho bạn, amplification, vì sao QUIC được xây trên UDP
- [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md): đọc `tcpdump`, `ss`, `ip`, `curl -v`, `dig`, `openssl s_client`. Đây là các tool mà mọi Practice đều dùng

**Protocols:**
- [`11-socket.md`](11-socket.md): bind/listen/accept, socket option, `SO_REUSEADDR`
- [`12-tcp.md`](12-tcp.md): segment và sequence number, handshake, state machine, FIN so với RST, `TIME_WAIT`, keepalive
- [`13-tcp-reliability.md`](13-tcp-reliability.md): retransmission, SACK, head-of-line blocking, flow control, congestion control, Nagle
- [`14-dns.md`](14-dns.md): hệ thống phân cấp, `getaddrinfo` thực sự làm gì, wire format, caching, `ndots`, rebinding
- [`15-http.md`](15-http.md): ngữ nghĩa HTTP gồm hình dạng message, `Host`, method, status code, validator, cookie, các header forwarding
- [`16-http1-wire-format.md`](16-http1-wire-format.md): grammar của HTTP/1.1 ở mức byte: request line, quy tắc header, quy tắc độ dài body, chunked coding, và khi từ chối thì trả status nào
- [`17-http2.md`](17-http2.md): vòng đời stream, trạng thái HPACK, flow control, Rapid Reset
- [`18-http3.md`](18-http3.md): QUIC, demux trên UDP, QPACK, chi phí congestion, Alt-Svc
- [`19-tls.md`](19-tls.md): handshake, SNI, ALPN, session resumption
- [`20-proxy-protocol.md`](20-proxy-protocol.md): giữ lại IP thật của client khi đứng sau một load balancer khác

**Capstone:**
- [`21-life-of-a-request.md`](21-life-of-a-request.md): một HTTPS request đi qua reverse proxy, qua mọi layer theo thứ tự, kèm bảng debug từ triệu chứng sang layer

**Review:**
- [`22-recall-and-review.md`](22-recall-and-review.md): bộ khung mười hai sự thật, câu hỏi theo từng file, hình vẽ lại từ trí nhớ, thí nghiệm dự đoán rồi chạy. Dùng theo lịch 1/3/7/21 ngày để kiến thức nhớ lâu

## Bước tiếp theo

Hãy đọc nhóm Fundamentals trước nếu bạn cần, vì mọi phần còn lại đều dựa vào nó.

- [`11-socket.md`](11-socket.md) + [`12-tcp.md`](12-tcp.md) là nền cho [`labs/00-tcp-server`](../../labs/00-tcp-server).
- [`16-http1-wire-format.md`](16-http1-wire-format.md) là nền cho [`labs/01-http-parser`](../../labs/01-http-parser).
- [`15-http.md`](15-http.md) là nền cho [`labs/02-http-server`](../../labs/02-http-server).
- [`19-tls.md`](19-tls.md) là nền cho [`labs/07-tls`](../../labs/07-tls).
- [`17-http2.md`](17-http2.md) và [`18-http3.md`](18-http3.md) là nền cho [`labs/08-http2`](../../labs/08-http2) và [`labs/09-http3`](../../labs/09-http3).

Phần OS của cùng câu chuyện nằm ở [`02-linux/`](../02-linux): process, kernel, bộ nhớ,
file, và Linux networking stack (netfilter, namespace). Cơ chế TCP phía kernel nằm ở
[`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md). Phần toán cổ điển đằng sau
congestion control và queueing nằm ở [`22-theory/`](../22-theory).
