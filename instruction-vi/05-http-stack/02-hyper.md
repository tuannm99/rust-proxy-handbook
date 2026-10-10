# hyper 1.x: How the Pieces Fit

Mọi lab từ [`labs/02-http-server`](../../labs/02-http-server) trở đi, và cả [`proxy/`](../../proxy), đều xây trên
`hyper` 1.x. hyper 1.0 cố tình nhỏ gọn: nó chỉ implement state machine của
HTTP/1 và HTTP/2, không gì khác, nên một server chạy được phải được lắp
từ năm sáu crate. File này là bản đồ của các crate đó và của những hành vi
quan trọng với một proxy, trong đó vài cái khiến nhiều người bất ngờ. Nó
nêu tên type và method cần tìm. docs.rs có signature chính xác, còn file
này nói bạn cần cái nào và vì sao. Mọi thứ ở đây đã được đối chiếu với
hyper 1.11 và hyper-util 0.1.20.

## What to learn

### Bản đồ crate
| Crate | Cho bạn cái gì |
|---|---|
| `http` | Các type dữ liệu thuần: `Request`, `Response`, `HeaderMap`, `Method`, `StatusCode`, `Uri`, `Version`. Không có I/O. |
| `http-body` | Trait `Body`: body là thứ bạn *poll* để lấy từng frame. |
| `http-body-util` | Body dựng sẵn và helper: `Full`, `Empty`, `StreamBody`, `Limited`, `BodyExt` (`.collect()`, `.boxed()`, `.map_err()`). |
| `bytes` | `Bytes`: buffer byte có reference-count. Clone hay cắt lát nó là O(1) và không bao giờ copy. |
| `hyper` | Các engine protocol: `server::conn::http1`/`http2`, `client::conn::http1`/`http2`, `body::Incoming`, `service::service_fn`. |
| `hyper-util` | Phần keo nối với tokio và tiện ích: `rt::{TokioIo, TokioExecutor, TokioTimer}`, `server::conn::auto` (HTTP/1 + HTTP/2 trên một port), `server::graceful`, `client::legacy::Client` (client có pool). |
| `h2` | Implementation HTTP/2 mà hyper dùng bên trong. Bạn cấu hình nó qua hyper, không bao giờ gọi trực tiếp. |

Nếu một cái tên không resolve được, kiểm tra Cargo feature trước. hyper và
hyper-util đặt gần như mọi thứ sau feature (`server`, `client`, `http1`,
`http2`, `server-auto`, `client-legacy`, `server-graceful`, `tokio`), và
thiếu feature sẽ hiện ra dưới dạng "no item named ... in module".

### Một connection là một future do bạn tự chạy
hyper không có accept loop và không có kiểu listener. Bạn tự sở hữu vòng
lặp: accept một `TcpStream` từ tokio, đưa nó cho connection builder cùng
một *service*, và nhận lại một future chạy toàn bộ connection. Nghĩa là
mọi request trên đó, keep-alive, và trạng thái protocol. Bạn `tokio::spawn`
future đó, mỗi connection một cái, y như echo server trong
[`labs/00-tcp-server`](../../labs/00-tcp-server). Future kết thúc khi connection đóng. Một `Err` từ nó
nghĩa là connection kết thúc không êm (client reset, lỗi parse, timeout).
Log nó ở mức debug. Đó là traffic bình thường, không phải bug của server.

Builder không nhận trực tiếp `TcpStream`. hyper định nghĩa trait I/O riêng
(`hyper::rt::Read`/`Write`) để không phụ thuộc vào tokio, và
`hyper_util::rt::TokioIo::new(stream)` chuyển một stream của tokio sang các
trait đó. Lỗi "the trait `hyper::rt::io::Read` is not implemented for
`TcpStream`" luôn luôn nghĩa là thiếu lớp bọc `TokioIo`. Cùng lớp bọc đó
cũng dùng cho TLS stream của `tokio_rustls` trong [`labs/07-tls`](../../labs/07-tls).

### Một service biến request thành future của response
Service được gọi một lần cho mỗi request, kể cả nhiều lần trên một
connection keep-alive. `hyper::service::service_fn` biến một async closure
thành service. Nó nhận `Request<hyper::body::Incoming>` và trả về future
của `Result<Response<B>, E>`. Nó phải là `'static`, nên state dùng chung
(bảng route, upstream pool, config) nằm trong một `Arc` mà closure clone.

Chỗ ai cũng vấp: **`Err` không có nghĩa là "gửi 500".** Nó có nghĩa là
"connection này hỏng rồi, hủy nó đi". Trên HTTP/1.1 hyper đóng connection
mà không ghi bất kỳ response nào. Trên HTTP/2 nó reset stream đó. Vì vậy
mọi lỗi ứng dụng (handler lỗi, upstream chết, input xấu) phải trở thành
một `Response` với status đúng *bên trong* service của bạn. Nhiều server
đặt kiểu lỗi là `std::convert::Infallible` để compiler bắt buộc điều này.
Đó chính là thứ mục "một lỗi sinh ra `500`, không phải connection bị ngắt"
của [`labs/02-http-server`](../../labs/02-http-server) kiểm tra.

Một **panic** bên trong service còn tệ hơn. Nó unwind task connection đã
spawn, nên connection biến mất không có response và, trừ khi bạn theo dõi
`JoinHandle` của task, không có dòng log nào. Biến panic thành `500` cần một
panic boundary tường minh, xem [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md).

### Body là stream các frame, được kéo theo nhu cầu
Body không phải `Vec<u8>`. Nó implement `http_body::Body`, với một method
cốt lõi `poll_frame` trả ra từng frame một: data frame (`Bytes`) và tối đa
một trailers frame (một `HeaderMap`, dùng bởi gRPC và trailer của chunked).
Hai hệ quả định hình mọi lab:

- **Request body không được đọc sẵn cho bạn.** `Incoming` chỉ đọc từ socket
  khi bạn poll nó. `BodyExt::collect()` đọc hết vào memory, và với input do
  attacker điều khiển thì cần giới hạn: bọc nó trong
  `http_body_util::Limited` trước, nếu không một client sẽ gửi 10 GB vào RAM
  của bạn.
- **Backpressure là tự động nếu bạn stream.** hyper chỉ poll response body
  khi nó ghi được vào socket. Client chậm nghĩa là hyper ngừng poll, nghĩa
  là việc đọc file hay đọc upstream của bạn cũng dừng. Đó là toàn bộ cơ chế
  đằng sau "download 1 GB mà memory phẳng" trong [`labs/04-static-server`](../../labs/04-static-server) và
  [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy). Bạn mất nó ngay khi `collect()` một body "chỉ
  để xem thử". Một proxy có thể đưa thẳng `Incoming` nhận được vào làm body
  của request gửi lên upstream, và byte chảy qua mà không bao giờ bị giữ
  trọn trong memory.

Kiểu response body, vì một handler phải trả về một kiểu cụ thể duy nhất:
`Full<Bytes>` cho body nhỏ trong memory, `Empty<Bytes>` khi không có body,
`StreamBody` bọc một `Stream` các `Result<Frame<Bytes>, E>` để stream, ví
dụ một file qua `tokio_util::io::ReaderStream`. Khi các nhánh khác nhau
sinh ra kiểu body khác nhau, `.boxed()` xóa kiểu tất cả thành một
`BoxBody<Bytes, E>` (một lần cấp phát mỗi response, chấp nhận được).

### Header mà hyper tự ghi cho bạn
hyper tự tính framing. Nếu body biết chính xác độ dài của nó (`Full` biết,
qua `size_hint`), hyper gửi `Content-Length`. Nếu không, response HTTP/1.1
đi ra dạng chunked. Nó cũng thêm `Date`, và tôn trọng `Connection: close`
ở cả hai phía bằng cách đóng sau response. Với request `HEAD`, và với
response `204`/`304`, hyper không bao giờ ghi body ra dây, bất kể body của
bạn chứa gì. Với `HEAD` bạn vẫn nên tránh *sinh ra* body (đừng mở và
stream một file 1 GB chỉ để hyper vứt đi), và `Content-Length` vẫn phải là
giá trị của `GET`, đó là thứ mục `HEAD` của [`labs/04-static-server`](../../labs/04-static-server) kiểm tra.

Gotcha: trong một proxy, đừng tự tay chép `Content-Length` và
`Transfer-Encoding` của upstream sang response của bạn. Chúng mô tả framing
của hop upstream, không phải của bạn. Xem
[`05-http-stack/03-hop-by-hop-headers.md`](03-hop-by-hop-headers.md): bỏ các framing header và để hyper
tạo lại chúng từ body bạn thực sự gửi.

### Một port cho HTTP/1.1 và HTTP/2
`hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())` phục
vụ cả hai. Nó nhìn vào những byte đầu tiên của connection: client HTTP/2
luôn mở đầu bằng preface cố định 24 byte `PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n`,
và mọi thứ khác được coi là HTTP/1.1. Đó là lý do HTTP/2 không mã hóa hoạt
động với `curl --http2-prior-knowledge`, khi curl gửi preface ngay lập tức.
Còn `curl --http2` thường trên kết nối không mã hóa thì xin một
`Upgrade: h2c` của HTTP/1.1, thứ mà hyper không implement, nên nó ở lại
HTTP/1.1. Qua TLS, protocol được chọn bằng ALPN ([`01-network/19-tls.md`](../01-network/19-tls.md)),
và client sau đó gửi preface, nên auto builder vẫn làm đúng. Executor là
bắt buộc vì HTTP/2 chạy các background task cho mỗi connection.

`.http1()` và `.http2()` trên builder trả về các sub-builder cho cấu hình
riêng từng protocol. Cần dùng `serve_connection_with_upgrades` thay vì
`serve_connection` cho bất cứ thứ gì dùng `Upgrade` của HTTP/1.1, như
WebSocket ([`05-http-stack/10-websocket.md`](10-websocket.md)).

### Timeout: gần như tất cả là việc của bạn
- **Header read timeout của HTTP/1.** `http1().header_read_timeout(d)` đóng
  connection không gửi xong request header trong vòng `d`. Nó chỉ hoạt động
  khi builder có timer: `http1().timer(TokioTimer::new())`. Đặt timeout mà
  không có timer sẽ panic. Đồng hồ bắt đầu chạy mỗi khi connection chờ một
  request head mới, *kể cả* khoảng rảnh giữa hai request keep-alive. Nên
  trong hyper 1.x, cấu hình này cũng chính là idle keep-alive timeout của
  HTTP/1. Hãy kiểm tra lại nếu bạn nâng cấp hyper, vì đây là hành vi, không
  phải một cam kết được ghi trong tài liệu.
- **Liveness của HTTP/2.** `http2().keep_alive_interval(d)` gửi PING và
  `keep_alive_timeout(d)` đóng connection nếu không được trả lời (cũng cần
  timer). Nó phát hiện peer đã chết. Nó không đóng connection rảnh nhưng
  vẫn khỏe.
- **Mọi thứ còn lại** không có timeout: handler, đọc request body, ghi
  response, lời gọi upstream. Tự bọc chúng bằng `tokio::time::timeout`. Một
  lời gọi upstream bị timeout là `504` ([`01-network/15-http.md`](../01-network/15-http.md)). Deadline
  cho từng lần đọc với sender chậm nằm ở [`07-security/10-slowloris.md`](../07-security/10-slowloris.md).

### Các cấu hình HTTP/2 bạn sẽ đụng tới
Trên `http2()`: `max_concurrent_streams` (giới hạn concurrency mỗi
connection từ [`01-network/17-http2.md`](../01-network/17-http2.md)), `initial_stream_window_size` và
`initial_connection_window_size` (flow-control window), `adaptive_window`
(để h2 tự định cỡ window theo băng thông đo được), `max_header_list_size`
(giới hạn header sau decode, chính là giới hạn chống HPACK bomb),
`max_send_buf_size`, và `max_pending_accept_reset_streams` cùng
`max_local_error_reset_streams` (giới hạn Rapid Reset có sẵn của h2: client
reset stream nhanh hơn mức này sẽ bị đóng connection). Đây là những núm
vặn mà [`labs/08-http2`](../../labs/08-http2) thực hành.

### Graceful shutdown
Future của connection có method `graceful_shutdown()`. Nó nhận
`Pin<&mut Self>`, nên hãy pin future và tiếp tục poll nó sau khi gọi. Trên
HTTP/1.1 nó hoàn thành response đang chạy rồi đóng. Trên HTTP/2 nó gửi
`GOAWAY`, để các stream đang mở chạy xong, và từ chối stream mới. Để drain
*tất cả* connection, `hyper_util::server::graceful::GracefulShutdown`
(feature `server-graceful`) bọc từng connection bằng `.watch(conn)`, và
`.shutdown().await` kích hoạt tất cả rồi chờ. Thiết kế process bao quanh
(ngừng accept, deadline drain, đóng cưỡng bức) nằm ở
[`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md).

### Phía client (cho proxy)
`hyper_util::client::legacy::Client::builder(TokioExecutor::new())
.build(connector)` cho bạn một client có pool. Connector là `HttpConnector`,
hoặc một TLS connector bọc nó. Những gì một proxy cần biết:

- **URI phải là absolute.** Client chọn pool và đích TCP từ scheme và
  authority của URI. Một request bạn nhận được với tư cách server có URI
  dạng origin-form (`/path?q`), nên chuyển tiếp nguyên như vậy sẽ lỗi
  `UserAbsoluteUriRequired`. Hãy dựng URI upstream từ địa chỉ upstream đã
  chọn cộng path và query gốc, và đặt `Host` theo policy của bạn.
- **Cấu hình pool.** `pool_idle_timeout` (giữ nó thấp hơn idle timeout của
  chính upstream, xem [`05-http-stack/05-keepalive.md`](05-keepalive.md)) và
  `pool_max_idle_per_host`.
- **Lỗi cho biết nó xảy ra ở đâu.** Kiểu lỗi của client có `is_connect()`:
  true nghĩa là kết nối TCP/TLS chưa bao giờ lên, nên request chưa hề được
  gửi và retry trên upstream khác là an toàn kể cả với `POST`. Nó tương ứng
  với `502`, và với các luật retry trong [`06-proxy/05-retry.md`](../06-proxy/05-retry.md).
  `HttpConnector::set_connect_timeout` giới hạn riêng bước connect. Deadline
  tổng thể là `tokio::time::timeout` của bạn.
- **Tự sở hữu pool.** `hyper::client::conn::http1::handshake` cho bạn một
  client connection thô không có pool. Đó là viên gạch nền nếu bạn tự
  implement pool trong [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) thay vì dùng legacy client.

## Practice
1. Trong [`labs/02-http-server`](../../labs/02-http-server), cho một route trả lời được `curl -v`, rồi cố ý cho service trả về `Err` và quan sát trong `curl -v` rằng connection đóng mà không có response nào. Giữ quan sát đó trong một comment. Đó là lý do lỗi ứng dụng phải là response.
2. Phục vụ một connection bằng `curl -v http://.../a http://.../b` và xác nhận qua debug log của hyper hoặc `ss -tn` rằng một TCP connection đã mang cả hai request.
3. Thêm `header_read_timeout` (kèm timer) và đo bằng `nc` rằng một connection rảnh bị đóng đúng hẹn, cả trước request đầu tiên lẫn giữa hai request.
4. Tạo một handler trả về body lớn dạng stream, đọc nó bằng `curl --limit-rate 100k`, và quan sát RSS của server giữ phẳng. Sau đó đổi sang `collect()` body trước và quan sát RSS tăng. **Done when** bạn giải thích được sự khác biệt từ `poll_frame`.
5. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), chuyển tiếp một request bằng legacy client, lần đầu với URI origin-form gốc (quan sát lỗi), rồi với URI absolute đã dựng lại. Tắt upstream và xác nhận lỗi báo `is_connect()`.
6. Gọi `graceful_shutdown()` khi một response chậm đang chạy và xác nhận response hoàn thành rồi connection mới đóng.
