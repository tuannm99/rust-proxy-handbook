# Vòng đời một Request: Mọi Layer, Từ Đầu Đến Cuối

Một lệnh `curl https://app.example.com/api` được theo dõi từ lúc gõ phím đến
lúc nhận response, đi qua một reverse proxy, gọi tên file trong thư mục này giải
thích từng bước — rồi lật ngược thành một bảng triệu chứng-sang-layer mà bạn
có thể dùng để debug. Đây là file nên đọc lại sau khi xong phần còn lại của
`01-network/`: nếu bước nào dưới đây còn mơ hồ, link cho biết nên quay lại đâu.

## What to learn

### Toàn bộ chuyến đi, theo thứ tự
```text
 1  parse URL            scheme=https host=app.example.com port=443 path=/api
 2  DNS                  app.example.com -> 203.0.113.10           [14-dns]
 3  route + ARP          dst không cùng subnet -> frame tới gateway [07, 08]
 4  NAT (router nhà)     src 192.168.1.5:51000 -> 198.51.100.7:40123 [02]
 5  TCP handshake        SYN / SYN-ACK / ACK                        [12]
 6  TLS handshake        ClientHello(SNI, ALPN) ... Finished        [19]
 7  HTTP request         GET /api, Host: app.example.com            [15, 16]
 8  proxy: accept->parse->route->upstream->forward                  [05-http-stack, 06-proxy]
 9  response byte        slow start, ACK, window                    [13]
10  reuse hoặc close     keep-alive / FIN                           [12]
```

### 1-2: Tên trước packet
Trước khi một packet nào rời đi, client parse URL (scheme chọn port mặc định: 443;
fragment không bao giờ được gửi) và resolve host ([`14-dns.md`](14-dns.md)): `getaddrinfo` tra
`/etc/hosts` và cache của resolver local, cái này hỏi một recursive resolver, mà
khi miss thì đi root, TLD, authoritative. Chi phí: 0 ms (đã cache), ~1 RTT tới
resolver, hoặc vài RTT cho một lần đi nguội. Nếu site đứng sau CDN, answer là một
địa chỉ **anycast** hoặc được chọn theo địa lý gần client
([`08-ip-and-icmp.md`](08-ip-and-icmp.md)). Giờ client có một IP, và một TTL cho biết answer đó dùng
được bao lâu.

### 3-4: Đưa một frame ra, và NAT trên đường
Client so sánh IP đích với subnet của nó ([`02-addressing.md`](02-addressing.md)): không phải local,
nên nó dùng **default route** và ARP lấy **MAC của gateway**
([`07-link-layer.md`](07-link-layer.md)). Frame được gửi tới gateway, IP packet gửi tới server. Router
nhà **ghi đè source** `192.168.1.5:51000` thành địa chỉ public của nó và một port
được chọn, ghi mapping vào bảng; traffic chiều về được khớp với bảng đó. Từ đây
packet đi qua có lẽ 10–15 router, mỗi cái giảm TTL, làm longest-prefix lookup, và
dựng lại Ethernet header cho link kế tiếp ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)). Không ai trên
đường biết đường đi; mỗi hop đưa ra quyết định cục bộ. Phía server cũng có thể đứng
sau một cloud load balancer làm destination NAT hoặc tự kết thúc connection
([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)).

### 5: TCP — một RTT
Client gọi `connect()`; kernel gửi `SYN` với initial sequence number và options
(MSS, window scale, SACK). SYN tới host của proxy, rơi vào SYN queue của một
listening socket; kernel reply `SYN+ACK`; `ACK` của client hoàn tất handshake và
connection chuyển sang **accept queue**, nơi `accept()` của proxy sẽ nhặt nó lên
([`11-socket.md`](11-socket.md), [`12-tcp.md`](12-tcp.md)). `connect()` của client trả về sau **1 RTT**; phía
server hoàn tất sau đó nửa RTT. Connection mới có congestion window nhỏ (~10
segment) ([`13-tcp-reliability.md`](13-tcp-reliability.md)).

### 6: TLS — thêm một RTT (TLS 1.3)
Client gửi `ClientHello` (các cipher hỗ trợ, một key share, **SNI** =
`app.example.com`, **ALPN** = `h2, http/1.1`). Proxy chọn certificate khớp SNI, đáp
bằng key share, certificate và `Finished` của nó; hai bên suy ra cùng session key
([`06-crypto-basics.md`](06-crypto-basics.md), [`19-tls.md`](19-tls.md)). Client kiểm tra certificate chain,
hostname và hiệu lực; kết quả ALPN quyết định HTTP/2 hay HTTP/1.1. Tổng cộng đến
đây: DNS + 1 RTT (TCP) + 1 RTT (TLS 1.3) trước byte HTTP đầu tiên — thứ mà
resumption, 0-RTT, keep-alive và QUIC ([`18-http3.md`](18-http3.md)) đều nhắm vào.

### 7: Request, dưới dạng byte
```text
GET /api HTTP/1.1\r\nHost: app.example.com\r\nAccept: */*\r\n\r\n
```
Được mã hóa trong các TLS record, TCP cắt thành segment ≤ MSS, chở trong IP
packet, trong Ethernet frame. Trong HTTP/2, cùng request đó là một HEADERS frame
trên một stream, nén bằng HPACK ([`17-http2.md`](17-http2.md)). Segment có thể tới bị cắt hoặc bị
gộp; read loop của proxy phải ghép lại cho tới terminator của header
([`16-http1-wire-format.md`](16-http1-wire-format.md), [`03-byte-streams.md`](03-byte-streams.md)).

### 8: Bên trong proxy
`accept()` của proxy trả về; tokio spawn một task cho connection
([`04-runtime/`](../04-runtime)). Nó kết thúc TLS, parse request
([`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md)), normalize và áp các kiểm tra bảo mật
([`07-security/`](../07-security)), chọn route theo `Host` + path
([`05-http-stack/04-router.md`](../05-http-stack/04-router.md)), chọn một upstream
([`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)), lấy một upstream connection từ **pool** — hoặc trả thêm
một round trip DNS + TCP (+ TLS) để mở một cái mới
([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)) — strip hop-by-hop header, thêm `X-Forwarded-For`
([`15-http.md`](15-http.md)), forward request, và **stream** response về, với backpressure suốt
đường ([`13-tcp-reliability.md`](13-tcp-reliability.md)). Lỗi ở đây trở thành `502`/`503`/`504`
([`15-http.md`](15-http.md)).

### 9-10: Response, và chuyện tiếp theo
Response đi theo cùng chuỗi nhưng ngược lại: upstream -> proxy (TCP connection riêng,
window riêng) -> client. Response lớn làm `cwnd` lên tốc; mỗi ACK vừa xác nhận data
vừa quảng bá window. Khi trao đổi xong, connection ở lại mở để tái sử dụng (HTTP
keep-alive, [`05-http-stack/05-keepalive.md`](../05-http-stack/05-keepalive.md)) hoặc đóng bằng `FIN`, bên đóng vào
`TIME_WAIT` ([`12-tcp.md`](12-tcp.md)). Lưu ý suốt thời gian đó có **hai** connection độc lập —
client-proxy và proxy-upstream — với DNS, TCP state, TLS session, window và failure
mode riêng; đó là ý nghĩa của "L7 proxy" ([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)).

### Triệu chứng -> layer: dùng mô hình để debug
| Bạn quan sát thấy | Layer / nguyên nhân khả dĩ nhất | Tool đầu tiên |
|---|---|---|
| `Could not resolve host` / `NXDOMAIN` | DNS: sai tên, thiếu record, search-domain/`ndots` | `dig`, `getent hosts` |
| connect treo, rồi timeout | packet bị drop: firewall, sai route, host chết, PMTUD black hole | `tcpdump` (SYN, không reply), `traceroute` |
| `Connection refused` (tức thì) | host tới được, không có gì listen (RST) | `ss -tlnp` trên server |
| connect OK, rồi `Connection reset` | app đóng khi còn dữ liệu chưa đọc, hoặc middlebox RST | `tcpdump` tìm cờ `R` |
| lỗi TLS / sai cert | thiếu SNI, cert hết hạn/không tin cậy, ALPN lệch | `openssl s_client -servername` |
| response nhỏ ổn, lớn thì treo | MTU / PMTUD (ICMP bị chặn) | `ping -M do -s`, MSS clamp |
| thêm ~40 ms cho mọi request nhỏ | Nagle + delayed ACK | `TCP_NODELAY` |
| đầu nhanh, truyền bulk chậm | cwnd lên tốc, loss, bufferbloat | `ss -ti`, tái hiện bằng `netem` |
| `502` | upstream từ chối/không hợp lệ/đóng sớm | log proxy, `ss` trên upstream |
| `504` | upstream nhận nhưng quá chậm | phân rã latency, metric upstream |
| cạn fd, nhiều `CLOSE_WAIT` | app không đóng socket | `ss -tan state close-wait` |
| nhiều `TIME_WAIT`, `EADDRNOTAVAIL` | connection upstream ngắn, không reuse | pool, `ip_local_port_range` |

### Gotcha: bạn chỉ thấy hop của mình
Mỗi bên chỉ thấy đoạn của mình: client thấy một connection tới proxy, upstream
thấy một connection từ proxy, và không bên nào thấy bên kia. IP trong `accept()`
của backend là của proxy trừ khi bạn truyền IP của client đi
([`15-http.md`](15-http.md), [`20-proxy-protocol.md`](20-proxy-protocol.md)). Timeout cũng cộng dồn: client timeout
ngắn hơn upstream timeout của proxy khiến proxy làm việc cho một client đã bỏ đi.
Tracing ([`08-observability/03-tracing.md`](../08-observability/03-tracing.md)) tồn tại để khâu các đoạn lại thành một
câu chuyện.

## Practice

1. Chọn một URL và, chỉ dùng các tool từ [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md), lập một
   timeline có con số cho mỗi bước trên: thời gian DNS, TCP connect, TLS, time to
   first byte, tổng (`curl -w`), và đối chiếu với timestamp của `tcpdump` cho SYN /
   SYN-ACK / ClientHello.
2. Với [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) đang chạy trước một upstream local, bắt cả hai
   phía cùng lúc (`sudo tcpdump -i lo -n 'port <proxy> or port <upstream>'`) cho một
   request và xác định connection client-proxy và proxy-upstream; chỉ ra trên giấy
   packet nào thuộc connection nào.
3. Chạy cùng request hai lần qua proxy với upstream keep-alive bật và tắt, bắt từng
   lần, và đếm SYN/SYN-ACK/ACK thừa ở trường hợp "tắt"; nêu latency tăng thêm dưới
   dạng số RTT.
4. Với mỗi hàng của bảng triệu chứng, tái hiện một cái trên máy của bạn (hostname
   sai, port đóng, rule `DROP` bằng `iptables`, một
   `iptables ... -j REJECT --reject-with tcp-reset`, SNI sai, một `netem` loss/delay)
   và ghi lại `curl -v` và `ss`/`tcpdump` cho thấy gì, để mỗi triệu chứng trở thành một
   dấu vân tay nhận ra được.
5. Viết, bằng lời của bạn và không nhìn lại, chuỗi sự kiện cho request đầu tiên tới
   một site HTTPS mới qua một reverse proxy (bước 1–10), rồi đối chiếu với file này
   và ghi lại những gì bạn bỏ sót.
