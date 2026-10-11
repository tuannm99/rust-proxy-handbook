# Vòng đời một Request: Đi Qua Mọi Layer, Từ Đầu Đến Cuối

File này đi theo một lệnh `curl https://app.example.com/api` từ lúc bạn gõ phím
đến lúc nhận được response, qua một reverse proxy. Mỗi bước có link về file giải
thích nó trong thư mục này. Cuối file là một bảng "triệu chứng → layer" để bạn
dùng khi debug. Hãy đọc file này sau khi học xong phần còn lại của `01-network/`.
Bước nào còn mơ hồ thì link cho bạn biết nên quay lại file nào.

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
 9  response về          slow start, ACK, window                    [13]
10  reuse hoặc close     keep-alive / FIN                           [12]
```

### Bước 1-2: Phải có địa chỉ trước khi gửi packet
Trước khi gửi packet nào, client làm hai việc.

Thứ nhất, nó parse URL. `https` cho biết port mặc định là 443. Phần fragment
(`#...`) không bao giờ được gửi đi.

Thứ hai, nó resolve hostname ([`14-dns.md`](14-dns.md)). Hàm `getaddrinfo` tra
`/etc/hosts` và cache của resolver local trước. Nếu không có, nó hỏi một
recursive resolver. Khi resolver cũng chưa có trong cache, nó đi từ root, sang
TLD, rồi tới authoritative server. Chi phí là 0 ms nếu đã có cache, khoảng 1 RTT
tới resolver, hoặc vài RTT nếu mọi cache đều nguội. Nếu site đứng sau CDN, answer
là một địa chỉ **anycast**, hoặc một địa chỉ được chọn theo vị trí địa lý gần
client ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)). Sau bước này client có một IP
và một TTL cho biết answer dùng lại được bao lâu.

### Bước 3-4: Gửi frame ra khỏi máy, và NAT trên đường đi
Client so IP đích với subnet của chính nó ([`02-addressing.md`](02-addressing.md)).
IP đích không nằm trong subnet, nên client dùng **default route** và gửi ARP để
lấy **MAC của gateway** ([`07-link-layer.md`](07-link-layer.md)). Frame có MAC
đích là gateway, còn IP packet bên trong có IP đích là server.

Router nhà **ghi đè source** `192.168.1.5:51000` thành địa chỉ public của nó cùng
một port mới. Nó lưu mapping này vào bảng, và packet chiều về được khớp lại theo
bảng đó. Sau đó packet đi qua khoảng 10–15 router. Mỗi router giảm TTL đi 1, tra
bảng routing bằng longest-prefix match, và tạo Ethernet header mới cho link kế
tiếp ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)). Không router nào biết toàn bộ
đường đi. Mỗi hop tự quyết định riêng. Ở phía server cũng có thể có một cloud
load balancer làm destination NAT, hoặc tự kết thúc connection
([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)).

### Bước 5: TCP, tốn một RTT
Client gọi `connect()`. Kernel gửi `SYN` kèm initial sequence number và các option
(MSS, window scale, SACK). SYN tới host của proxy và được đặt vào SYN queue của
listening socket. Kernel reply `SYN+ACK`. Khi `ACK` của client tới, handshake xong
và connection chuyển sang **accept queue**. Đây là chỗ `accept()` của proxy lấy nó
ra ([`11-socket.md`](11-socket.md), [`12-tcp.md`](12-tcp.md)).

`connect()` ở client trả về sau **1 RTT**. Server hoàn tất muộn hơn nửa RTT.
Connection mới bắt đầu với congestion window nhỏ, khoảng 10 segment
([`13-tcp-reliability.md`](13-tcp-reliability.md)).

### Bước 6: TLS, thêm một RTT (với TLS 1.3)
Client gửi `ClientHello`, gồm: danh sách cipher nó hỗ trợ, một key share,
**SNI** = `app.example.com` và **ALPN** = `h2, http/1.1`. Proxy chọn certificate
khớp với SNI, rồi gửi lại key share, certificate và `Finished`. Hai bên cùng tính
ra session key ([`06-crypto-basics.md`](06-crypto-basics.md),
[`19-tls.md`](19-tls.md)).

Client kiểm tra certificate chain, hostname và thời hạn hiệu lực. Kết quả ALPN
quyết định dùng HTTP/2 hay HTTP/1.1. Tới đây ta đã tốn: DNS + 1 RTT (TCP) + 1 RTT
(TLS 1.3) mà chưa gửi byte HTTP nào. Đây chính là chi phí mà resumption, 0-RTT,
keep-alive và QUIC ([`18-http3.md`](18-http3.md)) cố giảm.

### Bước 7: Request ở dạng byte
```text
GET /api HTTP/1.1\r\nHost: app.example.com\r\nAccept: */*\r\n\r\n
```
Các byte này được mã hóa thành TLS record. TCP cắt chúng thành các segment có kích
thước ≤ MSS. Mỗi segment nằm trong một IP packet, và IP packet nằm trong một
Ethernet frame. Với HTTP/2, cùng request đó là một HEADERS frame trên một stream,
header được nén bằng HPACK ([`17-http2.md`](17-http2.md)).

Request có thể tới proxy bị chia nhỏ hoặc dính liền với request khác. Vì vậy read
loop của proxy phải đọc và ghép lại cho đến khi gặp terminator của header
([`16-http1-wire-format.md`](16-http1-wire-format.md),
[`03-byte-streams.md`](03-byte-streams.md)).

### Bước 8: Bên trong proxy
`accept()` của proxy trả về, và tokio spawn một task cho connection này
([`04-runtime/`](../04-runtime)). Task lần lượt làm các việc sau:

1. Kết thúc TLS.
2. Parse request ([`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md)).
3. Normalize request và chạy các kiểm tra bảo mật ([`07-security/`](../07-security)).
4. Chọn route theo `Host` và path ([`05-http-stack/04-router.md`](../05-http-stack/04-router.md)).
5. Chọn một upstream ([`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)).
6. Lấy một upstream connection từ **pool**. Nếu pool không có sẵn thì phải mở
   connection mới, tốn thêm DNS + TCP (+ TLS)
   ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)).
7. Bỏ các hop-by-hop header, thêm `X-Forwarded-For` ([`15-http.md`](15-http.md)).
8. Forward request, rồi **stream** response về cho client, có backpressure xuyên
   suốt ([`13-tcp-reliability.md`](13-tcp-reliability.md)).

Nếu có lỗi ở giữa chừng, proxy trả `502`, `503` hoặc `504` ([`15-http.md`](15-http.md)).

### Bước 9-10: Response, và chuyện xảy ra sau đó
Response đi theo đúng đường đó nhưng ngược chiều: upstream → proxy → client. Hai
đoạn này là hai TCP connection riêng, mỗi đoạn có window riêng. Với response lớn,
`cwnd` tăng dần theo từng ACK. Mỗi ACK vừa xác nhận dữ liệu đã nhận, vừa báo cho
bên gửi biết window còn bao nhiêu.

Khi xong, connection có thể giữ mở để dùng lại (HTTP keep-alive,
[`05-http-stack/05-keepalive.md`](../05-http-stack/05-keepalive.md)), hoặc bị đóng
bằng `FIN`. Bên đóng trước sẽ vào trạng thái `TIME_WAIT`
([`12-tcp.md`](12-tcp.md)).

Hãy chú ý: suốt thời gian đó có **hai** connection độc lập, client↔proxy và
proxy↔upstream. Mỗi connection có DNS, TCP state, TLS session, window và cách hỏng
riêng. Đó là ý nghĩa của "L7 proxy" ([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)).

### Triệu chứng → layer: dùng mô hình để debug
| Bạn thấy | Layer / nguyên nhân khả dĩ nhất | Tool đầu tiên |
|---|---|---|
| `Could not resolve host` / `NXDOMAIN` | DNS: sai tên, thiếu record, search-domain/`ndots` | `dig`, `getent hosts` |
| connect bị treo, rồi timeout | packet bị drop: firewall, sai route, host chết, PMTUD black hole | `tcpdump` (có SYN, không có reply), `traceroute` |
| `Connection refused` (xuất hiện ngay) | tới được host, nhưng không có gì đang listen (RST) | `ss -tlnp` trên server |
| connect OK, rồi `Connection reset` | app đóng socket khi còn dữ liệu chưa đọc, hoặc middlebox gửi RST | `tcpdump`, tìm cờ `R` |
| lỗi TLS / sai cert | thiếu SNI, cert hết hạn hoặc không tin cậy, ALPN không khớp | `openssl s_client -servername` |
| response nhỏ thì ổn, response lớn thì treo | MTU / PMTUD (ICMP bị chặn) | `ping -M do -s`, MSS clamp |
| mọi request nhỏ đều chậm thêm ~40 ms | Nagle + delayed ACK | `TCP_NODELAY` |
| lúc đầu nhanh, truyền dữ liệu lớn thì chậm | cwnd còn nhỏ, có loss, bufferbloat | `ss -ti`, tái hiện bằng `netem` |
| `502` | upstream từ chối, trả response không hợp lệ, hoặc đóng sớm | log của proxy, `ss` trên upstream |
| `504` | upstream đã nhận request nhưng trả lời quá chậm | chia nhỏ latency, metric của upstream |
| hết fd, nhiều `CLOSE_WAIT` | app không đóng socket | `ss -tan state close-wait` |
| nhiều `TIME_WAIT`, lỗi `EADDRNOTAVAIL` | connection tới upstream sống ngắn, không được reuse | pool, `ip_local_port_range` |

### Gotcha: mỗi bên chỉ thấy hop của mình
Client chỉ thấy một connection tới proxy. Upstream chỉ thấy một connection từ
proxy. Không bên nào thấy bên kia. Vì vậy IP mà backend lấy từ `accept()` là IP của
proxy, trừ khi bạn chủ động chuyển IP của client đi tiếp
([`15-http.md`](15-http.md), [`20-proxy-protocol.md`](20-proxy-protocol.md)).

Timeout cũng cộng dồn. Nếu timeout của client ngắn hơn timeout của proxy với
upstream, client đã bỏ đi mà proxy vẫn tiếp tục làm việc cho nó. Distributed
tracing ([`08-observability/03-tracing.md`](../08-observability/03-tracing.md))
sinh ra để nối các đoạn rời này lại thành một câu chuyện liền mạch.

## Practice

1. Chọn một URL. Chỉ dùng các tool trong
   [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md), lập một
   timeline có số liệu cho từng bước ở trên: thời gian DNS, TCP connect, TLS, time
   to first byte, và tổng thời gian (`curl -w`). Sau đó đối chiếu với timestamp của
   `tcpdump` cho SYN, SYN-ACK và ClientHello.
2. Chạy [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) trước một upstream
   local. Bắt cả hai phía cùng lúc
   (`sudo tcpdump -i lo -n 'port <proxy> or port <upstream>'`) cho một request. Xác
   định connection client↔proxy và connection proxy↔upstream. Ghi ra giấy packet
   nào thuộc connection nào.
3. Chạy cùng một request hai lần qua proxy, một lần bật keep-alive tới upstream,
   một lần tắt. Bắt packet cả hai lần. Đếm số SYN/SYN-ACK/ACK dư ở lần tắt. Quy
   latency tăng thêm ra số RTT.
4. Với mỗi hàng trong bảng triệu chứng, tự tạo lại lỗi đó trên máy bạn: hostname
   sai, port đóng, rule `DROP` bằng `iptables`,
   `iptables ... -j REJECT --reject-with tcp-reset`, SNI sai, `netem` loss/delay.
   Ghi lại `curl -v` và `ss`/`tcpdump` hiển thị gì. Mục tiêu là mỗi triệu chứng trở
   thành một "dấu vân tay" bạn nhận ra ngay.
5. Không nhìn lại file này, viết bằng lời của bạn chuỗi sự kiện cho request đầu
   tiên tới một site HTTPS mới, đi qua một reverse proxy (bước 1–10). Sau đó đối
   chiếu với file và ghi lại những gì bạn đã bỏ sót.

---

<!-- nav -->
← Trước: [`01-network/20-proxy-protocol.md`](20-proxy-protocol.md) · [Mục lục](00-README.md) · Tiếp theo: [`01-network/22-recall-and-review.md`](22-recall-and-review.md) →
