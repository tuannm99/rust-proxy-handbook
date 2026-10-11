# HTTP/3

Kiến thức nền về QUIC.

## What to learn

### QUIC thay TCP, không chỉ thay HTTP framing
HTTP/3 chạy trên QUIC, và QUIC chạy trên UDP, không phải TCP. QUIC tự làm lại reliability, ordering và congestion
control ở transport layer, trong userspace, thay vì dựa vào TCP stack của kernel.

Đây là thay đổi tư duy lớn nhất so với HTTP/1.1 và /2: "listener" cho HTTP/3 của proxy bạn là một UDP socket, không phải
`TcpListener`.

### Vì sao QUIC tránh được head-of-line blocking
QUIC multiplex các stream độc lập giống HTTP/2. Khác ở chỗ việc phục hồi packet bị mất diễn ra *theo từng stream* bên
trong QUIC (retransmission của TCP thì theo cả connection). Vì vậy một packet bị mất trên một stream không làm các
stream khác đứng lại. Điều này sửa HOL blocking ở TCP layer mà HTTP/2 trên TCP vẫn còn (xem
[`17-http2.md`](17-http2.md)).

### Connection migration và 0-RTT
Một QUIC connection được định danh bằng Connection ID, không phải 4-tuple (source IP, source port, dest IP, dest port).
Vì vậy khi client đổi mạng (từ WiFi sang di động), nó vẫn giữ được cùng một QUIC connection.

0-RTT cho phép client resume một phiên trước đó và gửi dữ liệu ứng dụng ngay trong lượt bay đầu tiên. Cái giá là rủi ro
replay attack với các request không idempotent. Một proxy chấp nhận dữ liệu 0-RTT phải coi nó là "có thể bị replay", và
chỉ cho phép với các request safe/idempotent.

### TLS 1.3 là bắt buộc và nằm sẵn bên trong
QUIC không xếp TLS lên trên như cách TCP+TLS làm. Handshake của QUIC *chính là* một handshake TLS 1.3, được mang trong
các tham số transport của QUIC. Vì vậy không có QUIC dạng cleartext. Mọi deployment HTTP/3 cần cùng bộ máy
cert/SNI/ALPN như [`19-tls.md`](19-tls.md), chỉ khác cách mang trên đường truyền.

### Một UDP socket, nhiều connection
Thay đổi về vận hành này lớn hơn nhiều so với "dùng UDP thay vì TCP". Với TCP, `accept()` cho bạn một fd riêng cho mỗi
connection, và kernel tự demux giúp bạn. Với QUIC, bạn có **một** UDP socket duy nhất nhận datagram cho *mọi* connection.
Bạn tự demultiplex trong userspace: đọc Connection ID từ mỗi packet và chuyển nó tới đúng state của connection. Không có
fd riêng cho từng connection, nên giới hạn fd không còn là ràng buộc. Bảng connection của chính bạn mới là ràng buộc.

Hệ quả xuất hiện ngay:
- Receive loop là một điểm nóng duy nhất (`quinn` giảm nhẹ bằng batching `recvmmsg` và `SO_REUSEPORT` giữa các worker).
- Không có backpressure ở `accept()`. Datagram đến dù bạn đã sẵn sàng hay chưa. Vì vậy việc giới hạn tốc độ accept ở
  [`07-security/09-ddos.md`](../07-security/09-ddos.md) phải diễn ra sau khi bạn đã parse đủ packet để biết đó là một nỗ
  lực connect mới.

Gotcha: kích thước UDP buffer quan trọng hơn nhiều so với TCP. Giá trị mặc định `net.core.rmem_max` của kernel thường quá
nhỏ cho QUIC throughput cao. Triệu chứng là packet bị drop âm thầm, được đếm trong `netstat -su`, và không phải là lỗi
code mà bạn nhìn thấy.

### Stream, và vì sao HTTP/3 cần QPACK
QUIC cho HTTP/3 các stream hai chiều và một chiều. Mỗi stream đáng tin cậy và có thứ tự *trong chính nó*. HTTP/3 ánh xạ
một request/response lên một stream hai chiều, nên framing đơn giản hơn HTTP/2 nhiều. Không cần tự làm multiplexing,
QUIC đã làm sẵn.

Nhưng HPACK không dùng được ở đây. Bảng động của HPACK giả định header đến theo thứ tự. Các stream QUIC được giao độc
lập, nên một cập nhật bảng trên stream 4 có thể đến sau một tham chiếu tới nó trên stream 8.

**QPACK** giải quyết việc này: các cập nhật bảng đi trên các stream encoder/decoder một chiều riêng, và một request được
phép chờ cho đến khi các entry nó tham chiếu đã đến.

Gotcha: việc chờ đó là một head-of-line stall mà bạn tự tạo lại nếu nén quá tay. QPACK có
`SETTINGS_QPACK_BLOCKED_STREAMS` để giới hạn. Đặt bảng động về 0 sẽ loại bỏ hoàn toàn rủi ro này, nhưng đổi lại tỷ lệ nén
giảm. Đó là lựa chọn hợp lý cho một proxy ưu tiên latency ổn định.

### Congestion control chuyển vào process của bạn
Với TCP, congestion control là việc của kernel và do operator tune. Với QUIC, nó là việc của dependency bạn dùng, và chạy
trên ngân sách CPU của bạn: xử lý ACK cho từng packet, timer phát hiện mất packet, và pacing đều chạy trong userspace.

Hãy kỳ vọng CPU trên mỗi byte cao hơn đáng kể so với TCP. Khoảng cách đó là lý do chính khiến HTTP/3 không tự động là
lựa chọn đúng cho một hop proxy nội bộ.

### Triển khai: Alt-Svc và đường fallback
Client không bắt đầu bằng HTTP/3. Nó connect qua TCP, rồi biết về HTTP/3 từ một response header
`Alt-Svc: h3=":443"; ma=86400`, và thử QUIC ở các request sau. Vì vậy một deployment HTTP/3 *luôn luôn* cũng là một
deployment TCP. Bạn không thể bỏ HTTP/1.1/2. Bạn cũng phải xử lý trường hợp UDP bị middlebox chặn, khi đó client âm thầm
fallback về TCP.

Gotcha: chỉ quảng cáo `Alt-Svc` khi đường UDP của bạn thật sự tới được. Nếu bạn quảng cáo trong khi UDP bị firewall chặn,
client rơi vào vòng lặp connect-timeout rồi retry ở mỗi request, chậm hơn cả khi chưa bao giờ quảng cáo.

### Hệ sinh thái Rust hiện tại
`quinn` là implementation QUIC async chính. `h3` (xây trên `quinn`) implement HTTP/3 framing trên nền đó. Tại thời điểm
viết, cả `hyper` lẫn `hyper-util` đều không nói HTTP/3 trực tiếp. Đó là một tích hợp riêng. Vì vậy [`proxy`](../../proxy)
coi HTTP/3 là mục tiêu mở rộng, không phải yêu cầu cơ bản.

### quinn trong thực tế
Đây là những gì bạn cần để [`labs/09-http3`](../../labs/09-http3) nói chuyện được, theo thứ tự bạn sẽ gặp:

- **Certificate.** QUIC luôn chạy TLS 1.3, nên ngay cả một echo server trong lab cũng cần certificate. Hãy tạo một CA và
  một leaf đúng như phần "Một CA local để test" trong [`01-network/19-tls.md`](19-tls.md). Đừng phục vụ chính CA: client
  quinn verify bằng rustls, và rustls từ chối một CA certificate được đưa ra làm server certificate.
- **Phía server.** `quinn::ServerConfig::with_single_cert(chain, key)` dựng config từ cùng các giá trị
  `CertificateDer`/`PrivateKeyDer` mà rustls dùng. `quinn::Endpoint::server(config, addr)` bind một UDP socket duy nhất.
  Nếu cần ALPN hoặc cấu hình rustls khác, hãy tự dựng một `rustls::ServerConfig`, chuyển nó bằng
  `quinn::crypto::rustls::QuicServerConfig::try_from(...)`, rồi bọc trong
  `quinn::ServerConfig::with_crypto(Arc::new(...))`.
- **Phía client.** `quinn::Endpoint::client("0.0.0.0:0".parse()?)` bind một UDP port tạm.
  `quinn::ClientConfig::with_root_certificates(roots)` nhận một `rustls::RootCertStore` mà bạn đã thêm `ca.pem` vào. Cài
  nó bằng `set_default_client_config`. `endpoint.connect(addr, "localhost")?.await` tạo connection, và tên phải là một
  trong các SAN của leaf.
- **ALPN.** Nếu một trong hai phía đặt ALPN, cả hai phải có chung ít nhất một giá trị, nếu không handshake thất bại. Một
  echo lab thô có thể không đặt ALPN ở cả hai phía. HTTP/3 bắt buộc protocol `h3`, và đó là điểm xuất phát của mục tiêu
  mở rộng `h3`.
- **Accept.** `endpoint.accept().await` trả ra một `Incoming` cho mỗi connection mới. Await nó sẽ hoàn tất handshake
  thành một `Connection`. Hãy spawn một task cho mỗi connection, giống như với TCP.
- **Stream.** `connection.open_bi().await` (client) và `connection.accept_bi().await` (server) cho một cặp
  `(SendStream, RecvStream)`. Mỗi stream có flow-control window riêng, nên một stream mà bạn ngừng đọc chỉ làm nghẽn
  riêng nó. `send.finish()` đánh dấu bạn đã gửi xong. `recv.read_to_end(limit)` đọc cho đến khi peer kết thúc, có giới
  hạn. Stream rất rẻ: hãy mở một stream cho mỗi request, không phải một cho mỗi connection.
- **Nhìn thấy việc demultiplex.** Mỗi connection có `stable_id()` và `remote_address()`. Khi cài `tracing_subscriber` và
  đặt `RUST_LOG=quinn_proto=trace`, quinn log event `new connection` kèm connection ID ban đầu. Hai client cho thấy hai
  connection ID đến trên cùng một socket, trong khi `ss -uanp` cho thấy chỉ có đúng một socket.

Gotcha: `netstat` (từ `net-tools`) không được cài sẵn trên nhiều distro hiện đại. Các bộ đếm UDP tương tự nằm ở
`nstat -az | grep -i udp` hoặc `/proc/net/snmp`. Bộ đếm tăng khi socket buffer tràn là `UdpRcvbufErrors`
(`RcvbufErrors` trong output của `netstat -su`).

## Practice

1. Bắt traffic HTTP/3 từ một trình duyệt truy cập một site hỗ trợ nó (dùng `chrome://net-export`, hoặc một bản build
   `curl --http3`) và xác nhận trên đường truyền đó là UDP, không phải TCP.
2. Đọc ví dụ client/server của crate `quinn` và xác định handshake TLS 1.3 diễn ra ở đâu so với handshake QUIC.
3. Xây một endpoint QUIC echo tối giản trong [`labs/09-http3`](../../labs/09-http3) bằng `quinn`. Như một bài mở rộng,
   thêm một listener HTTP/3 dùng `quinn` + `h3` vào [`proxy`](../../proxy) cạnh listener HTTP/1.1/2 hiện có, và so sánh
   những gì phải thay đổi trong config TLS của bạn.
4. Giải thích bằng lời của bạn vì sao dữ liệu 0-RTT không bao giờ nên được tin cho một request không idempotent như
   `POST /transfer-funds`.
5. Trong [`labs/09-http3`](../../labs/09-http3), chạy hai QUIC connection đồng thời trên cùng một UDP socket và log việc
   demux Connection ID. Xác nhận bạn thấy cả hai được route từ cùng một socket.
6. Kiểm tra `net.core.rmem_max` trên máy bạn. Sau đó dồn tải vào endpoint đủ mạnh để thấy drop trong `netstat -su`. Tăng
   buffer và xác nhận bộ đếm drop ngừng tăng. Rồi so sánh CPU trên mỗi megabyte truyền giữa endpoint QUIC của bạn và một
   lần truyền TCP+TLS thuần cùng lượng dữ liệu. Hãy tự đo khoảng cách đó thay vì tin suông.

---

<!-- nav -->
← Trước: [`01-network/17-http2.md`](17-http2.md) · [Mục lục](00-README.md) · Tiếp theo: [`01-network/19-tls.md`](19-tls.md) →
