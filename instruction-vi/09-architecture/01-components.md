# Components
Listener -> ConnMgr -> Codec -> Router -> Modules

## What to learn
### Mỗi stage sở hữu gì
- **Listener**: bind (các) socket, accept kết nối, có thể làm TLS
  termination (chuyển giao một stream đã giải mã). Không sở hữu gì về
  ngữ nghĩa HTTP.
- **ConnMgr (connection manager)**: theo dõi các kết nối đang sống, áp
  đặt giới hạn/timeout theo từng kết nối, điều khiển graceful shutdown
  (ngừng nhận kết nối mới, để các kết nối hiện có drain — xem
  [`09-architecture/04-graceful-shutdown.md`](04-graceful-shutdown.md)).
- **Codec**: biến byte thành các giá trị `Request`/`Response` có kiểu và
  ngược lại (parsing HTTP/1.1, framing HTTP/2) — đây là chỗ hyper nằm nếu
  bạn dùng nó, hoặc parser của riêng bạn nếu bạn đã làm
  [`labs/01-http-parser`](../../labs/01-http-parser).
- **Router**: khớp một request với một đích — một upstream pool cụ thể,
  hoặc một handler cục bộ (health endpoint, metrics endpoint). Logic quyết
  định thuần túy, không I/O.
- **Modules**: mọi thứ bọc quanh request/response trên đường đi — auth,
  rate limiting, WAF, logging, metrics. Thứ tự quan trọng (ví dụ rate
  limit trước auth để reject rẻ; WAF trước cả hai để reject body độc hại
  sớm).

### Thứ tự module là một quyết định bảo mật, không phải sở thích
Rải rác khắp [`07-security/`](../07-security) và [`05-http-stack/`](../05-http-stack) là các ràng buộc về thứ tự
mà mỗi cái trông có vẻ cục bộ và cùng nhau định nghĩa pipeline. Gom lại:

| Vị trí | Stage | Vì sao ở đây |
| --- | --- | --- |
| 1 | Giới hạn kết nối/accept | Reject rẻ nhất có thể, trước bất kỳ parsing nào ([`07-security/09-ddos.md`](../07-security/09-ddos.md)) |
| 2 | Lọc IP trên peer thật | Trước bất cứ thứ gì đắt; dùng địa chỉ socket, không phải header ([`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md)) |
| 3 | TLS termination | Reject client-cert nên xảy ra lúc handshake, không phải sau đó ([`07-security/01-auth.md`](../07-security/01-auth.md)) |
| 4 | Codec / parse | Kiểm tra framing và reject smuggling ([`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md)) |
| 5 | **Strip header hop-by-hop và identity** | Phải xảy ra trước khi bất cứ thứ gì đọc chúng ([`05-http-stack/04-keepalive.md`](../05-http-stack/04-keepalive.md), [`07-security/01-auth.md`](../07-security/01-auth.md)) |
| 6 | Chuẩn hóa path | Trước routing, nếu không routing quyết định trên một path khác với cái upstream thấy ([`05-http-stack/03-router.md`](../05-http-stack/03-router.md)) |
| 7 | Routing | Cần để biết chính sách *nào* áp dụng cho phần còn lại |
| 8 | Rate limiting theo route | Reject rẻ trước công việc đắt ([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)) |
| 9 | Auth | Trước khi kiểm tra body và trước bất kỳ chi phí upstream nào |
| 10 | WAF / kiểm tra body | Kiểm tra đắt nhất, chạy cuối và chỉ cho traffic đã xác thực, không bị rate-limit ([`07-security/06-waf.md`](../07-security/06-waf.md)) |
| 11 | Tra cứu cache | Trước lệnh gọi upstream, sau auth (nếu không bạn phục vụ response của người này cho người khác — [`05-http-stack/07-cache.md`](../05-http-stack/07-cache.md)) |
| 12 | Lệnh gọi upstream | Load balancing, retry, circuit breaking ([`06-proxy/`](../06-proxy)) |

Logging và metrics bọc quanh toàn bộ, vì chúng phải quan sát các request bị
reject ở mọi stage phía trên.

Gotcha: stage 5 và 7 là những cái người ta hay làm sai một cách vô ý. Strip
header identity *sau khi* một module đã đọc chúng, hoặc routing trước khi
chuẩn hóa, tạo ra một pipeline đúng trong test và khai thác được trong
production.

### Ghép các module thành một pipeline, không phải một khối nguyên
Mỗi module nên test độc lập được: cho một request (và một chút state), nó
forward, short-circuit (403/429), hay mutate (thêm một header) — mà không
biết gì về những module khác. Trong Rust điều này ánh xạ tự nhiên lên
`tower::Service`/`Layer`: một `Router` là một `Service`, mỗi module là một
`Layer` bọc nó, và toàn bộ stack ghép lại qua `ServiceBuilder`.

```rust
use tower::{ServiceBuilder, service_fn};

let svc = ServiceBuilder::new()
    .layer(RateLimitLayer::new(/* ... */))
    .layer(AuthLayer::new(/* ... */))
    .layer(MetricsLayer::new(/* ... */))
    .service(router_service);
```
Gotcha: thứ tự `Layer` trong `ServiceBuilder` bọc từ ngoài-vào-trong nhưng
*thực thi* ngoài-trước trên đường đi request — làm ngược cái này và rate
limiting chạy sau cái kiểm tra auth đắt đỏ mà nó lẽ ra phải bảo vệ.

Gotcha: `Service::poll_ready` là cơ chế backpressure của tower và nó
thường xuyên bị bỏ qua. Một service trả `Poll::Pending` từ `poll_ready`
đang nói "tôi đã đầy capacity, đừng gửi request cho tôi vội" — đó là cách
một giới hạn concurrency lan truyền *ngược lại* qua stack thay vì xếp hàng
nội bộ (lập luận shed-vs-queue của [`07-security/09-ddos.md`](../07-security/09-ddos.md)). Một module
luôn trả `Ready` và tự buffer nội bộ đã âm thầm biến backpressure thành bộ
nhớ không giới hạn.

Gotcha: `poll_ready` dành sẵn capacity nghĩa là lệnh `call` *kế tiếp* mới
là cái được quyền dùng nó. Gọi `poll_ready` một lần rồi `call` nhiều lần
là vi phạm hợp đồng mà chính các combinator của tower giả định bạn sẽ
không phạm phải.

### State theo-từng-request nằm ở đâu
Các module cần truyền thông tin về phía trước — identity đã xác thực từ
auth, route đã khớp, upstream đã chọn, các mốc thời gian. Hai lựa chọn là
truyền một context type tùy chỉnh qua từng module (tường minh, type-safe,
và một thay đổi chữ ký hàm mỗi khi thêm gì đó) hoặc dùng `http::Extensions`,
một map được key theo type gắn vào request.

Extensions là lựa chọn idiomatic trong một tower stack, với một kỷ luật:
chèn *newtype*, không phải primitive trần. `req.extensions().get::<String>()`
mơ hồ ngay khi hai module cùng chèn một `String`; `get::<UserId>()` không
thể xung đột.

Gotcha: một extension mà một module sau *yêu cầu* là một dependency vô
hình — hệ thống type sẽ không nói cho bạn biết `UpstreamSelector` panic khi
`AuthLayer` không có trong stack. Làm cho việc lấy giá trị fail tường minh
(một 5xx với một internal error rõ ràng) thay vì `unwrap()`, và ghi lại
yêu cầu đó ở chỗ module được định nghĩa.

### Một module không bao giờ được làm sập kết nối
Hai failure mode cần thiết kế trước:
- **Lỗi.** Trong tower, một lỗi `Service` lan lên trên và thường giết chết
  kết nối. Với một proxy, gần như mọi lỗi module nên trở thành một
  *response* thay vào đó (401, 429, 502) — nên các module nên infallible ở
  cấp type (`Error = Infallible`) và tự chuyển các lỗi nội bộ thành
  response. Cách đó một bug trong một module không thể làm rớt một request
  pipelined không liên quan trên cùng kết nối.
- **Panic.** Một panic trong một task request, mặc định, chỉ unwind task
  đó — tokio bắt nó và runtime sống sót — nhưng kết nối bị drop giữa
  chừng response và client thấy một reset. Bắt panic ở biên pipeline và
  chuyển chúng thành một 500 với một event được log, để một request tồi
  không kéo theo cả một kết nối keep-alive đầy các request khác.

Gotcha: `panic = "abort"` trong release profile của bạn biến mọi panic
thành một crash toàn process, thay đổi hoàn toàn phép tính này. Biết bạn
đã cấu hình cái nào.

### Vì sao hình dạng này, không phải "một async fn khổng lồ"
Một hàm handler khổng lồ duy nhất hoạt động cho một đồ chơi nhưng trở nên
không test được và không đọc được một khi bạn có 5+ mối quan tâm cắt
ngang. Chia thành Listener/ConnMgr/Codec/Router/Modules nghĩa là mỗi mảnh
ánh xạ tới một phần handbook ([`01-network`](../01-network), [`04-runtime`](../04-runtime), [`05-http-stack`](../05-http-stack),
[`06-proxy`](../06-proxy), [`07-security`](../07-security)) và có thể được xây/test độc lập trước khi ghép
lại trong [`proxy`](../../proxy).

### Luồng dữ liệu qua pipeline
Vào: `TcpStream` → (giải mã TLS) → Codec decode → `Request` chảy qua stack
Module → Router chọn upstream → Codec encode `Request` đi ra → được
forward. Response chảy ngược lại qua cùng stack Module theo chiều ngược
(để một module logging thấy cả request gốc lẫn response/status cuối
cùng).

Gotcha: "theo chiều ngược" nghĩa là công việc phía-response của một module
chạy theo thứ tự ngược với công việc phía-request của nó, thường đó là
điều bạn muốn (module logging ngoài cùng thấy status cuối cùng) và thỉnh
thoảng không phải (compression phải chạy *bên trong* caching, để cache lưu
một biểu diễn mà nó có thể re-serve cho một client khác thay vì một cái đã
nén không thể — [`05-http-stack/06-compression.md`](../05-http-stack/06-compression.md),
[`05-http-stack/07-cache.md`](../05-http-stack/07-cache.md)). Ghi rõ thứ tự đường đi response; đừng giả
định nó tự nhiên đúng.

Gotcha: một response dạng streaming nghĩa là response "đi qua" các module
trước khi body được tạo ra. Một module muốn kiểm tra hoặc biến đổi body
đang chọn buffer nó (thảo luận về giới hạn trong [`07-security/06-waf.md`](../07-security/06-waf.md))
— và một module chỉ muốn status code không được vô tình ép buffer bằng
cách await toàn bộ body.

## Practice
Xây theo thứ tự.

1. Phác thảo pipeline cho [`proxy`](../../proxy) — module, thứ tự, và lý do — dựa trên
   bảng ở trên. **Xong khi** bạn có thể biện minh cho mỗi vị trí bằng một
   failure cụ thể nó ngăn chặn, không phải từ quy ước.
2. Cài đặt Router và một module (rate limiting) như các cài đặt
   `tower::Service`/`Layer` riêng biệt với `Error = Infallible`. **Xong
   khi** mỗi cái được unit-test mà không cần mạng, và một reject rate-limit
   là một response 429 thay vì một lỗi service.
3. Nối Listener → ConnMgr → Codec → Router → Modules end-to-end cho một
   upstream. **Xong khi** một request thật chảy qua và quay lại.
4. Thêm auth và xác minh thứ tự. **Xong khi** một test chứng minh một
   request bị rate-limit không bao giờ tới được auth, và một request với
   một `X-User-Id` giả mạo bị strip nó trước khi bất kỳ module nào đọc
   được nó.
5. Truyền state qua các extension có kiểu. **Xong khi** auth chèn một
   newtype `UserId` mà module logging đọc, và xóa `AuthLayer` khỏi stack
   tạo ra một 500 rõ ràng với một giải thích được log thay vì một panic.
6. Cài đặt backpressure dựa trên `poll_ready` trong một module giới hạn
   concurrency. **Xong khi** quá tải khiến stack shed
   ([`07-security/09-ddos.md`](../07-security/09-ddos.md)) thay vì buffer — đo bộ nhớ dưới quá tải kéo
   dài để chứng minh không có gì đang xếp hàng vô hình.
7. Thêm việc chặn panic ở biên pipeline. **Xong khi** một module panic
   trên một request trả 500 cho request đó và các request pipelined
   *khác* của kết nối vẫn hoàn thành.
8. Ghi lại thứ tự đường đi response. **Xong khi** compression chạy bên
   trong caching (cache lưu một biểu diễn chưa nén), và một test chứng
   minh một entry đã cache có thể phục vụ cho các client với
   `Accept-Encoding` khác nhau.
9. Thêm graceful shutdown ở lớp ConnMgr
   ([`09-architecture/04-graceful-shutdown.md`](04-graceful-shutdown.md)). **Xong khi** các request
   in-flight hoàn thành trước khi thoát.
