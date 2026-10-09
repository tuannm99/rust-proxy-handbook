# Network

Phase 1 của lộ trình học. Các giao thức mà một proxy phải nói, từ socket
cho tới HTTP/3 — đọc trước [`02-linux/`](../02-linux) và [`05-http-stack/`](../05-http-stack), hai phần giả
định bạn đã biết một connection và một request thực sự là gì.

## Cách đọc thư mục này

Hai cách dùng, tùy vào bạn đang ở đâu:

- **Chưa có nền tảng networking:** đọc mọi file dưới đây theo thứ tự, từ
  [`01-fundamentals.md`](01-fundamentals.md) đến [`21-life-of-a-request.md`](21-life-of-a-request.md), làm `## Practice` của
  mỗi file trước khi qua file kế tiếp. Coi cả thư mục này như một tutorial
  liên tục — các file sau giả định bạn đã nắm mọi file trước đó, đừng nhảy
  cóc dù tên file nghe có vẻ quen. File cuối,
  [`21-life-of-a-request.md`](21-life-of-a-request.md), nối mọi thứ lại và là bài kiểm tra xem phần còn
  lại đã ngấm chưa: nếu bạn kể lại nó được mà không có lỗ hổng, bạn đã làm chủ
  tài liệu.
- **Đã thoải mái với socket, TCP, và HTTP:** dùng thư mục này như một tài
  liệu tra cứu — nhảy thẳng tới file nào bù đắp đúng lỗ hổng của bạn, theo
  thứ tự bất kỳ. Nhóm Fundamentals dưới đây là một primer từ con số 0 mà
  bạn chắc không cần, nhóm "Bên dưới socket" cũng vậy trừ
  [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) nếu bạn chưa đọc packet capture thành thạo.
  [`05-proxy-taxonomy.md`](05-proxy-taxonomy.md) và [`06-crypto-basics.md`](06-crypto-basics.md) vẫn
  đáng đọc dù nền tảng bạn mạnh, vì chúng là cách đóng khung riêng của
  handbook này ([`proxy/`](../../proxy) nằm ở đâu, và vốn từ vựng crypto mà [`19-tls.md`](19-tls.md)
  giả định bạn đã biết) chứ không phải kiến thức networking chung bạn đã
  có sẵn từ nơi khác.

## Files

**Fundamentals** (bắt đầu từ đây nếu "port," "packet," "handshake," "NAT,"
hay "certificate" chưa có nghĩa chính xác với bạn — nhóm file duy nhất
trong thư mục này được viết như một primer từ con số 0 thay vì giả định
sẵn một baseline; mọi file bên dưới đều giả định bạn đã nắm phần này):
- [`01-fundamentals.md`](01-fundamentals.md) — mô hình client-server, và một index tới năm file bên dưới
- [`02-addressing.md`](02-addressing.md) — IP address, port, CIDR notation, NAT (SNAT/DNAT/CGNAT), kiến thức routing cơ bản
- [`03-byte-streams.md`](03-byte-streams.md) — ảo giác byte-stream, packet/segment/datagram, TCP vs UDP, handshake như một pattern
- [`04-latency-throughput.md`](04-latency-throughput.md) — latency, bandwidth, throughput, RTT, bandwidth-delay product
- [`05-proxy-taxonomy.md`](05-proxy-taxonomy.md) — forward vs reverse proxy, L4 vs L7, NAT gateway, API gateway, CDN, sidecar — [`proxy/`](../../proxy) nằm ở đâu trong bức tranh này
- [`06-crypto-basics.md`](06-crypto-basics.md) — mã hóa symmetric/asymmetric, hashing, HMAC, digital signature, certificate/PKI — điều kiện tiên quyết mà [`19-tls.md`](19-tls.md) và các file identity trong [`07-security/`](../07-security) giả định bạn đã biết

**Bên dưới socket** (các layer dưới TCP — một packet đi từ A tới B về mặt vật lý
như thế nào, và cách *nhìn thấy* nó; đọc trước phần protocol, vì mọi Practice bên
dưới đều dùng các tool này):
- [`07-link-layer.md`](07-link-layer.md) — encapsulation, Ethernet, MAC, switch, ARP, MTU, VLAN/bridge/veth
- [`08-ip-and-icmp.md`](08-ip-and-icmp.md) — IPv4 header, TTL, routing longest-prefix, fragmentation và PMTUD, ICMP, IPv6
- [`09-udp.md`](09-udp.md) — datagram vs stream, UDP không cho bạn những gì, amplification, vì sao QUIC xây trên nó
- [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) — đọc `tcpdump`, `ss`, `ip`, `curl -v`, `dig`, `openssl s_client`; các tool mà mọi Practice dựa vào

**Protocols:**
- [`11-socket.md`](11-socket.md) — bind/listen/accept, socket option, `SO_REUSEADDR`
- [`12-tcp.md`](12-tcp.md) — segment và sequence number, handshake, state machine, FIN vs RST, `TIME_WAIT`, keepalive
- [`13-tcp-reliability.md`](13-tcp-reliability.md) — retransmission, SACK, head-of-line blocking, flow control, congestion control, Nagle
- [`14-dns.md`](14-dns.md) — hệ phân cấp, `getaddrinfo` thực sự làm gì, wire format, caching, `ndots`, rebinding
- [`15-http.md`](15-http.md) — ngữ nghĩa HTTP: hình dạng message, `Host`, method, status code, validator, cookie, forwarding header
- [`16-http1-wire-format.md`](16-http1-wire-format.md) — grammar HTTP/1.1 ở mức byte: request line, luật header, luật độ dài body, chunked coding, reject thì trả status nào
- [`17-http2.md`](17-http2.md) — vòng đời stream, trạng thái HPACK, flow control, Rapid Reset
- [`18-http3.md`](18-http3.md) — QUIC, demux trên UDP, QPACK, chi phí congestion, Alt-Svc
- [`19-tls.md`](19-tls.md) — handshake, SNI, ALPN, session resumption
- [`20-proxy-protocol.md`](20-proxy-protocol.md) — giữ lại IP client thật khi đứng sau một load balancer khác

**Capstone:**
- [`21-life-of-a-request.md`](21-life-of-a-request.md) — một HTTPS request đi qua reverse proxy, mọi layer theo thứ tự, kèm bảng debug triệu chứng-sang-layer

**Review:**
- [`22-recall-and-review.md`](22-recall-and-review.md) — bộ khung mười hai sự thật, câu hỏi theo từng file, hình vẽ lại từ trí nhớ, thí nghiệm dự đoán-rồi-chạy; dùng theo lịch 1/3/7/21 ngày để kiến thức nhớ lâu

## Bước tiếp theo

Nhóm fundamentals trước nếu bạn cần — mọi phần còn lại đều giả định bạn đã
qua nó. [`11-socket.md`](11-socket.md) + [`12-tcp.md`](12-tcp.md) là nền cho [`labs/00-tcp-server`](../../labs/00-tcp-server);
[`16-http1-wire-format.md`](16-http1-wire-format.md) là nền cho [`labs/01-http-parser`](../../labs/01-http-parser); [`15-http.md`](15-http.md) là nền cho [`labs/02-http-server`](../../labs/02-http-server); [`19-tls.md`](19-tls.md) là nền cho
[`labs/07-tls`](../../labs/07-tls); [`17-http2.md`](17-http2.md) và [`18-http3.md`](18-http3.md) là nền cho [`labs/08-http2`](../../labs/08-http2) và
[`labs/09-http3`](../../labs/09-http3). Phía OS của cùng câu chuyện — process, kernel, bộ nhớ,
file, và Linux networking stack (netfilter, namespace) — là [`02-linux/`](../02-linux); cơ chế
phía kernel của TCP nằm ở [`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md), và phần toán cổ điển đằng sau
congestion control và queueing nằm ở [`22-theory/`](../22-theory).
