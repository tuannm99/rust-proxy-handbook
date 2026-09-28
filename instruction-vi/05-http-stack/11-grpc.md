# gRPC Proxying

gRPC là HTTP/2 với một quy ước framing riêng trên nền đó — proxy nó đúng
đắn nghĩa là không phá vỡ quy ước đó, không phải hiểu protobuf.

## What to learn
### Framing message của gRPC bên trong frame DATA của HTTP/2
Mỗi message gRPC là một byte cờ compression, một tiền tố độ dài 4-byte big-endian, rồi từng đó byte payload protobuf — tất cả được mang bên trong các frame `DATA` HTTP/2 bình thường (xem [`01-network/12-http2.md`](../01-network/12-http2.md)). Một proxy forward gRPC không cần hiểu protobuf hay thậm chí cả framing này; nó chỉ cần forward frame `DATA` trung thực, đúng từng byte, mà không làm bất cứ gì một code path hướng-HTTP/1.1 có thể phản xạ làm (ví dụ buffer toàn bộ body để tính `Content-Length` — body gRPC được prefix-độ-dài theo từng message, không phải một lần cho cả stream, và thường không giới hạn/streaming).

Gotcha: gRPC có compression riêng theo từng message (byte cờ đó), được
negotiate qua `grpc-encoding`/`grpc-accept-encoding`. Nó *không phải*
`Content-Encoding` của HTTP, và một proxy áp compression response của
riêng nó ([`05-http-stack/07-compression.md`](07-compression.md)) lên một body gRPC sẽ làm
hỏng nó — client sẽ cố parse một luồng gzip như các message
prefix-độ-dài. Loại trừ tường minh content type `application/grpc` khỏi
response compression.

### Trailer mang kết quả RPC thật
Không như HTTP/1.1 nơi một response là headers-rồi-body-rồi-xong, HTTP/2 (và gRPC cụ thể) hỗ trợ **trailer** — một frame HEADERS thứ hai gửi *sau* mọi frame DATA, mang `grpc-status` và `grpc-message`. Đây là nơi client biết RPC có thực sự thành công hay không — một response HTTP 200 từ một cuộc gọi gRPC vẫn có thể là một RPC thất bại một khi bạn kiểm tra trailer. Các code path proxy viết theo mô hình tinh thần HTTP/1.1 thường xuyên bỏ hoặc xử lý sai trailer vì "response đã kết thúc rồi" một khi header và body đã thấy; với gRPC giả định đó âm thầm vứt bỏ kết quả thật.

```rust
// what must survive the proxy hop, conceptually — headers, N data frames, then trailers
struct GrpcResponseShape {
    headers: HeaderMap,        // usually just ":status: 200", content-type
    data_frames: Vec<Bytes>,   // one or more length-prefixed gRPC messages
    trailers: HeaderMap,       // grpc-status, grpc-message — the real result
}
```

Gotcha: còn có một response **trailers-only** — một frame HEADERS duy nhất
với `grpc-status` và END_STREAM, không hề có DATA — dùng khi một RPC fail
ngay lập tức. Một proxy giả định "headers, rồi body, rồi trailer" và chờ
một body sẽ không bao giờ tới sẽ bị treo hoặc làm hỏng nó. Xử lý tường
minh trường hợp zero-DATA.

### Response lỗi của chính bạn cũng phải mang hình dạng gRPC
Khi chính proxy fail một request — không có upstream khỏe mạnh, circuit mở ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)), bị rate limit ([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)) — phản xạ là trả về HTTP 503 hoặc 429 kèm một body ngắn. Với một client gRPC, đó là một response dị dạng: nó đang tìm `grpc-status` trong trailer, và một lỗi HTTP thuần biểu hiện như một lỗi transport khó hiểu thay vì status code sạch mà ứng dụng biết cách xử lý.

Phát ra một response trailers-only thay vào đó, với `:status: 200` và một
`grpc-status` phù hợp — `14` (UNAVAILABLE) cho không-có-upstream hoặc
circuit mở, `8` (RESOURCE_EXHAUSTED) cho rate limiting, `4`
(DEADLINE_EXCEEDED) cho timeout. Retry và xử lý lỗi có sẵn của client khi
đó hoạt động đúng như thiết kế, đó chính là mục đích.

Gotcha: điều này đòi hỏi proxy phải biết request *là* gRPC trước khi nó
báo lỗi — kiểm tra `content-type: application/grpc*` sớm và route việc
sinh lỗi cho phù hợp. Một proxy với một đường lỗi dùng chung sẽ sai điều
này ngay từ cách xây dựng.

### Semantics streaming: đừng áp buffer request/response
gRPC có bốn hình dạng cuộc gọi: unary, client-streaming, server-streaming, và bidirectional streaming — tất cả đều chỉ là một stream HTTP/2 mang bao nhiêu frame DATA tùy ý theo cả hai chiều trước khi trailer đóng nó. Một proxy buffer toàn bộ request body trước khi forward (hợp lý cho một POST HTTP/1.1 nhỏ) phá vỡ hoàn toàn cuộc gọi client-streaming và bidi, vì upstream có thể cần bắt đầu trả lời trước khi client gửi xong. Forward frame ngay khi chúng đến; đừng chờ stream kết thúc trừ khi có lý do cụ thể.

Gotcha: điều tương tự áp dụng cho mọi timeout bạn thừa hưởng từ mô hình
request/response, y hệt như với WebSocket
([`05-http-stack/10-websocket.md`](10-websocket.md)). Một RPC server-streaming phát một cập
nhật mỗi vài phút là khỏe mạnh; một tổng-request-timeout giết nó theo
lịch. Tệ hơn, client gRPC gửi deadline riêng của chúng trong header
`grpc-timeout` — proxy nên *tôn trọng* nó (và rút ngắn theo thời gian đã
tiêu) thay vì áp một cái không liên quan, để deadline mà ứng dụng đặt ra
là cái được áp dụng.

Gotcha: buffer body để kiểm tra WAF ([`07-security/06-waf.md`](../07-security/06-waf.md)) hay replay
retry ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)) về cơ bản không tương thích với RPC
streaming. Quyết định theo từng content-type, không phải toàn cục, nếu
không khách hàng bidi-streaming đầu tiên của bạn sẽ tự phát hiện ra điều
đó cho bạn.

### Load balancing gRPC không phải cùng bài toán với load balancing HTTP/1.1
Một client gRPC thường mở một connection HTTP/2 sống lâu và đa hợp nhiều RPC độc lập trên đó (xem phần multiplexing của [`01-network/12-http2.md`](../01-network/12-http2.md)). Một load balancer chọn một upstream *theo từng connection* (như một balancer L4/TCP thuần, hay một implementation [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md) ngây thơ viết theo tư duy một-request-mỗi-connection của HTTP/1.1) gửi mọi RPC trên connection đó tới cùng một upstream mãi mãi, đánh bại hoàn toàn load balancing một khi client đã kết nối. Load balancing gRPC đúng đắn phải nhận biết từng stream HTTP/2 riêng và chọn một upstream theo từng RPC, không phải theo từng connection.

Gotcha: điều này tương tác xấu với các sự kiện scaling. Connection sống
lâu bị gắn vào một tập con upstream nghĩa là upstream mới được thêm bởi
autoscaling ([`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md)) không nhận được gì —
giới hạn tuổi thọ connection từ [`05-http-stack/05-keepalive.md`](05-keepalive.md) là thứ
cuối cùng rebalance, và với gRPC tương đương là định kỳ gửi `GOAWAY` để
client reconnect và phân phối lại.

### Health checking upstream gRPC
gRPC định nghĩa protocol health checking riêng (`grpc.health.v1.Health`) với các method `Check` và `Watch` — một upstream nói gRPC có thể không serve một `/healthz` HTTP nào cả, nên probe từ [`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md) cần một biến thể nhận biết gRPC. `Watch` là cái tốt hơn trong hai cho một proxy: nó stream các thay đổi trạng thái thay vì đòi một khoảng poll, nên việc phát hiện là tức thì và chi phí probe từ `healthcheck.md` phần lớn biến mất.

### Observability cần thêm chiều gRPC
Một góc nhìn chỉ-HTTP về traffic gRPC gây hiểu lầm theo một cách cụ thể: gần như mọi response đều là HTTP 200, kể cả mọi thất bại. Một dashboard xây trên HTTP status code cho thấy một service hoàn toàn khỏe mạnh trong khi mọi RPC trả về `grpc-status: 13` (INTERNAL).

Ghi `grpc-status` như một chiều metric riêng, và lấy tên method từ path
(`/package.Service/Method`) làm nhãn route thay vì coi mỗi cái là một URL
duy nhất ([`08-observability/02-metrics.md`](../08-observability/02-metrics.md)).

## Practice
Làm theo thứ tự này.

1. Dựng một upstream gRPC thật với `tonic`, bao gồm một method trả về
   status không-OK và một method server-streaming. **Xong khi** một
   client `grpcurl` nói chuyện trực tiếp với nó và thấy cả hai hành vi.
2. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), proxy nó end-to-end. **Xong khi** một
   cuộc gọi unary thành công qua proxy và `grpc-status` không-OK tới
   được client như một status đúng đắn — không phải lỗi transport và
   không phải một thành công giả.
3. Thêm trường hợp trailers-only. **Xong khi** một RPC fail ngay lập tức
   (không có frame DATA nào) được proxy đúng thay vì bị treo.
4. Làm cho lỗi của chính proxy có hình dạng gRPC. **Xong khi** giết mọi
   upstream tạo ra `grpc-status: 14 UNAVAILABLE` ở client thay vì một
   body HTTP 503, và một cuộc gọi bị rate-limit tạo ra
   `8 RESOURCE_EXHAUSTED`.
5. Test client-streaming và bidi. **Xong khi** upstream nhận frame đúng
   lúc client gửi chúng (xác minh bằng timestamp phía upstream) thay vì
   tất cả cùng lúc ở cuối stream.
6. Tôn trọng `grpc-timeout`, trừ đi thời gian đã tiêu, và loại trừ RPC
   đã upgrade/streaming khỏi timeout kiểu request. **Xong khi** một RPC
   server-streaming phát một message mỗi 30 giây sống vô thời hạn, và
   một deadline 100ms client đặt được upstream enforce thay vì bị một
   mặc định của proxy ghi đè.
7. Loại trừ `application/grpc` khỏi response compression. **Xong khi**
   một response gRPC đi qua giống hệt byte-gốc trong khi một response
   HTML trên cùng proxy vẫn bị nén.
8. Xác minh load balancing theo từng RPC. **Xong khi** một connection
   client phát ra nhiều RPC phân phối chúng trên nhiều upstream — nếu
   tất cả rơi vào một cái, balancer của bạn đang chọn theo connection và
   cần sửa.
9. Thêm health checking gRPC (`Check`, rồi `Watch`) và `grpc-status` như
   một chiều metric gắn nhãn theo `/package.Service/Method`. **Xong khi**
   một dashboard cho thấy một upstream trả toàn INTERNAL là không khỏe
   mạnh, dù mọi HTTP status đều là 200.
