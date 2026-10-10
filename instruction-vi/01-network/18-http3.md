# HTTP/3

Kiến thức nền về QUIC.

## What to learn

### QUIC thay thế TCP, không chỉ HTTP framing
HTTP/3 chạy trên QUIC, chạy trên UDP — không phải TCP. QUIC tự tái hiện
reliability, ordering, và congestion control ở tầng transport, trong
userspace, thay vì dựa vào TCP stack của kernel. Đây là sự thay đổi tư duy
lớn nhất so với HTTP/1.1 và /2: "listener" cho HTTP/3 của proxy bạn là một
UDP socket, không phải một `TcpListener`.

### Vì sao QUIC tránh được head-of-line blocking
QUIC multiplex các stream độc lập giống cách HTTP/2 làm, nhưng vì việc
khôi phục mất gói diễn ra theo từng stream bên trong QUIC (không phải
theo từng connection như cách retransmission của TCP làm), một packet bị mất
trên một stream không stall các stream khác. Điều này sửa HOL blocking ở
tầng TCP mà HTTP/2 trên TCP vẫn còn (xem [`17-http2.md`](17-http2.md)).

### Connection migration và 0-RTT
Connection QUIC được định danh bởi một Connection ID, không phải một 4-tuple
(source IP, source port, dest IP, dest port) — nên một client chuyển mạng
(WiFi sang di động) có thể giữ nguyên cùng một connection QUIC. 0-RTT cho
phép một client resume một phiên trước đó và gửi dữ liệu ứng dụng ngay
trong lượt bay đầu tiên, với cái giá là rủi ro replay-attack cho các
request không idempotent — một proxy chấp nhận dữ liệu 0-RTT phải đối xử
với nó như "có thể bị replay" và chỉ cho phép nó với các request
safe/idempotent.

### TLS 1.3 là bắt buộc và được tích hợp sẵn
QUIC không chồng TLS lên trên theo cách TCP+TLS làm — handshake của QUIC
*chính là* một handshake TLS 1.3 được mang trong các tham số transport
của QUIC, nên không có QUIC dạng cleartext. Điều này nghĩa là mọi
deployment HTTP/3 cần cùng bộ máy cert/SNI/ALPN như [`19-tls.md`](19-tls.md), chỉ được
mang khác đi trên đường truyền.

### Một UDP socket, nhiều connection
Sự thay đổi vận hành lớn hơn "dùng UDP thay vì TCP". Với TCP, `accept()`
đưa cho bạn một fd riêng biệt cho mỗi connection và kernel demux giúp bạn.
Với QUIC bạn sở hữu **một** UDP socket duy nhất nhận datagram cho *mọi*
connection, và bạn tự demultiplex ở userspace bằng cách đọc Connection ID ra
khỏi mỗi packet và route nó tới đúng trạng thái connection. Không có fd riêng
cho từng connection, nên giới hạn fd không còn là ràng buộc — và bảng connection
của chính bạn trở thành ràng buộc đó.

Hệ quả xuất hiện ngay lập tức: receive loop là một điểm nóng duy nhất
(`quinn` giảm nhẹ bằng batching `recvmmsg` và `SO_REUSEPORT` giữa các
worker), và không có backpressure ở `accept()` — datagram đến bất kể bạn
đã sẵn sàng hay chưa, nên việc giới hạn accept-rate từ [`07-security/09-ddos.md`](../07-security/09-ddos.md)
phải diễn ra sau khi đã parse đủ packet để biết đó là một nỗ lực connection
mới.

Gotcha: kích thước buffer UDP quan trọng hơn hẳn so với TCP. Giá trị mặc
định `net.core.rmem_max` của kernel thường xuyên quá nhỏ cho QUIC
throughput cao, và triệu chứng là các packet bị drop âm thầm, được đếm
trong `netstat -su`, không phải một lỗi code của bạn thấy được.

### Stream, và vì sao HTTP/3 cần QPACK
QUIC cho HTTP/3 các stream hai chiều và một chiều, mỗi cái reliable và có
thứ tự *trong chính nó*. HTTP/3 ánh xạ một request/response lên một
stream hai chiều, nên tầng framing đơn giản hơn nhiều so với HTTP/2 —
không cần implement multiplexing stream, QUIC đã làm rồi.

Nhưng HPACK không thể sống sót ở đây. Bảng động của HPACK giả định header
đến theo thứ tự; các stream QUIC giao hàng độc lập, nên một cập nhật bảng
trên stream 4 có thể đến sau một tham chiếu tới nó trên stream 8.
**QPACK** giải quyết việc này bằng cách mang các cập nhật bảng trên các
stream encoder/decoder một chiều riêng và cho phép một request block cho
tới khi các entry nó tham chiếu đã đến.

Gotcha: việc block đó là một stall head-of-line mà bạn tái tạo lại bằng
cách nén quá tay. QPACK expose `SETTINGS_QPACK_BLOCKED_STREAMS` để giới
hạn nó; đặt bảng động về 0 tắt hoàn toàn rủi ro này với cái giá là tỷ lệ
nén, đây là một lựa chọn hợp lý cho một proxy coi trọng latency ổn định.

### Congestion control chuyển vào process của bạn
Congestion control của TCP là vấn đề của kernel và được operator tune.
Của QUIC là vấn đề của dependency bạn dùng, và nó chạy trên ngân sách CPU
của bạn: xử lý ACK theo từng packet, timer phát hiện mất gói, và pacing
đều diễn ra ở userspace. Hãy kỳ vọng CPU trên mỗi byte cao hơn đáng kể so
với TCP — khoảng cách đó là lý do chính khiến HTTP/3 không tự động là lựa
chọn mặc định đúng đắn cho một hop proxy nội bộ.

### Triển khai nó: Alt-Svc và đường fallback
Client không bắt đầu bằng HTTP/3. Chúng connection qua TCP và biết về HTTP/3
từ một response header `Alt-Svc: h3=":443"; ma=86400`, rồi thử QUIC ở các
request sau. Nên một deployment HTTP/3 *luôn luôn* cũng là một deployment
TCP — bạn không thể bỏ HTTP/1.1/2 — và bạn phải xử lý trường hợp UDP bị
một middlebox block, nơi client âm thầm fallback về TCP.

Gotcha: chỉ quảng cáo `Alt-Svc` một khi đường UDP của bạn thực sự tiếp cận
được. Quảng cáo nó trong khi UDP bị firewall block đẩy client vào một vòng
connect-timeout rồi retry ở mỗi request, chậm hơn cả việc chưa bao giờ
quảng cáo nó.

### Hệ sinh thái Rust hiện tại
`quinn` là implementation QUIC async chính; `h3` (xây trên `quinn`)
implement framing HTTP/3 trên nền đó. Tại thời điểm viết, cả `hyper` lẫn
`hyper-util` đều không nói HTTP/3 trực tiếp — đó là một tích hợp riêng
biệt, đó là lý do [`proxy`](../../proxy) coi HTTP/3 là một mục tiêu mở rộng chứ không
phải một yêu cầu baseline.

### quinn trong thực tế
Những gì bạn cần để [`labs/09-http3`](../../labs/09-http3) nói chuyện được, theo thứ tự bạn sẽ gặp:

- **Certificate.** QUIC luôn chạy TLS 1.3, nên ngay cả một echo server
  trong lab cũng cần certificate. Tạo một CA và một leaf đúng như phần "Một
  CA local để test" trong [`01-network/19-tls.md`](19-tls.md). Đừng phục vụ chính CA:
  client quinn verify bằng rustls, và rustls từ chối một CA certificate được
  đưa ra làm server certificate.
- **Phía server.** `quinn::ServerConfig::with_single_cert(chain, key)` dựng
  config từ cùng các giá trị `CertificateDer`/`PrivateKeyDer` mà rustls dùng.
  `quinn::Endpoint::server(config, addr)` bind một UDP socket duy nhất. Nếu
  cần ALPN hay cấu hình rustls khác, hãy tự dựng một `rustls::ServerConfig`,
  chuyển nó bằng `quinn::crypto::rustls::QuicServerConfig::try_from(...)`, rồi
  bọc trong `quinn::ServerConfig::with_crypto(Arc::new(...))`.
- **Phía client.** `quinn::Endpoint::client("0.0.0.0:0".parse()?)` bind một
  UDP port tạm. `quinn::ClientConfig::with_root_certificates(roots)` nhận
  một `rustls::RootCertStore` mà bạn đã thêm `ca.pem` vào. Cài nó bằng
  `set_default_client_config`. `endpoint.connect(addr, "localhost")?.await`
  connection, và tên phải là một trong các SAN của leaf.
- **ALPN.** Nếu một trong hai phía đặt ALPN, cả hai phải có chung một giá
  trị, nếu không handshake thất bại. Một echo lab thô có thể không đặt gì ở
  cả hai phía. HTTP/3 bắt buộc protocol `h3`, và đó là điểm xuất phát của
  mục tiêu mở rộng `h3`.
- **Accept.** `endpoint.accept().await` trả ra một `Incoming` cho mỗi
  connection mới, và await nó sẽ hoàn tất handshake thành một `Connection`.
  Spawn một task cho mỗi connection, giống TCP.
- **Stream.** `connection.open_bi().await` (client) và
  `connection.accept_bi().await` (server) cho một cặp
  `(SendStream, RecvStream)`. Mỗi stream có flow-control window riêng, đó
  chính là lý do một stream bạn ngừng đọc chỉ làm nghẽn riêng nó.
  `send.finish()` đánh dấu kết thúc phần bạn gửi. `recv.read_to_end(limit)`
  đọc tới khi peer kết thúc, có giới hạn. Stream rất rẻ: mở một stream cho
  mỗi request, không phải một cho mỗi connection.
- **Nhìn thấy việc demultiplex.** Mỗi connection có `stable_id()` và
  `remote_address()`. Khi cài `tracing_subscriber` và đặt
  `RUST_LOG=quinn_proto=trace`, quinn log event `new connection` kèm
  connection ID ban đầu. Hai client cho thấy hai connection ID đến trên
  cùng một socket, trong khi `ss -uanp` cho thấy chỉ có đúng một socket.

Gotcha: `netstat` (từ `net-tools`) không được cài trên nhiều distro hiện
đại. Cùng các bộ đếm UDP đó nằm ở `nstat -az | grep -i udp` hoặc
`/proc/net/snmp`. Bộ đếm tăng lên khi socket buffer tràn là
`UdpRcvbufErrors` (`RcvbufErrors` trong output của `netstat -su`).

## Practice

1. Capture traffic HTTP/3 từ một trình duyệt truy cập một site hỗ trợ nó
   (kiểm tra `chrome://net-export` hoặc một bản build `curl --http3`) và
   xác nhận đó là UDP trên đường truyền, không phải TCP.
2. Đọc ví dụ client/server của crate `quinn` và xác định handshake TLS
   1.3 diễn ra ở đâu tương ứng với handshake QUIC.
3. Xây một endpoint QUIC echo tối giản trong [`labs/09-http3`](../../labs/09-http3) bằng
   `quinn`, rồi như một bài tập mở rộng, thêm một listener HTTP/3 dùng
   `quinn` + `h3` vào [`proxy`](../../proxy) cạnh listener HTTP/1.1/2 sẵn có, và so sánh
   những gì phải thay đổi trong config TLS của bạn.
4. Giải thích bằng lời của bạn vì sao dữ liệu 0-RTT không bao giờ nên
   được tin tưởng cho một request không idempotent như `POST
   /transfer-funds`.
5. Trong [`labs/09-http3`](../../labs/09-http3), chạy hai connection QUIC đồng thời trên cùng một
   UDP socket của bạn và log việc demux Connection ID — xác nhận bạn có
   thể thấy cả hai được route từ cùng một socket.
6. Kiểm tra `net.core.rmem_max` trên máy bạn, rồi dồn tải endpoint đủ
   mạnh để thấy drop trong `netstat -su`; tăng buffer lên và xác nhận bộ
   đếm drop ngừng tăng. Rồi so sánh CPU trên mỗi megabyte truyền giữa
   endpoint QUIC của bạn và một lần truyền TCP+TLS thuần túy cùng lượng
   dữ liệu đó — tự định lượng khoảng cách thay vì tin suông vào tuyên bố
   này.
