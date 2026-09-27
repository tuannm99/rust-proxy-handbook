# Virtual Host / Multi-Tenant Routing

Route tới backend khác nhau theo *site nào* mà một request nhắm tới,
không chỉ path của nó — thứ mà [`03-router.md`](03-router.md) giả định đã được quyết
định rồi.

## What to learn
### Routing theo Host header (sau TLS, tầng HTTP)
[`03-router.md`](03-router.md) bao quát việc match path và method trong bảng route của một backend. Một proxy đứng trước nhiều site/tenant trước tiên phải chọn *bảng route nào* để dùng, dựa trên header `Host` (HTTP/1.1) hoặc pseudo-header `:authority` (HTTP/2, xem [`01-network/11-http2.md`](../01-network/11-http2.md)) — cả hai mang cùng thông tin, chỉ framing khác nhau. Lookup này xảy ra sau khi TLS đã terminate, vì header nằm trong request đã mã hóa.

```rust
use std::collections::HashMap;

struct VirtualHost {
    upstream_pool: UpstreamPoolId,
}

fn route_by_host<'a>(
    vhosts: &'a HashMap<String, VirtualHost>,
    host_header: &str,
) -> Option<&'a VirtualHost> {
    // strip a trailing :port — clients often send "example.com:443"
    let host = host_header.split(':').next().unwrap_or(host_header);
    vhosts.get(host)
}
```
Gotcha: mặc định reject (404, hoặc một vhost catch-all riêng) bất kỳ
`Host` nào không khớp một entry đã biết. Âm thầm rơi xuống một backend
"mặc định" nào đó chính xác là cách các bug Host-header injection và
cache-poisoning xảy ra — một attacker gửi một `Host` bất ngờ và bị route
tới nơi không định trước, hoặc một cache key lỏng lẻo theo Host serve
response của tenant sai cho người khác.

Gotcha: normalize trước khi lookup, cùng cách [`03-router.md`](03-router.md) normalize
path. Hostname không phân biệt hoa thường (`EXAMPLE.com` phải khớp
`example.com`), một dấu chấm cuối là hợp lệ và mang cùng nghĩa
(`example.com.`), và các dạng IDN/punycode (`xn--...`) phải ánh xạ về
một dạng canonical duy nhất. Một bảng lookup key theo byte thô coi mỗi
biến thể là một tenant khác — hoặc là một 404 cho một người dùng hợp lệ,
hoặc, nếu một biến thể rơi xuống một mặc định, một lỗ hổng bypass routing.

Gotcha: cách tách port ở trên sai với IPv6 literal (`[::1]:8443`), thứ
chứa dấu hai chấm. Xử lý tường minh dạng có ngoặc vuông thay vì tách theo
dấu hai chấm đầu tiên.

### Sự bất khớp Host và SNI (domain fronting)
Dưới TLS, client nói cho bạn hostname **hai lần**: một lần trong SNI extension của ClientHello, ở dạng rõ, trước khi handshake hoàn tất; và một lần trong header `Host`/`:authority`, đã mã hóa, sau đó. Không gì trong protocol buộc chúng phải khớp nhau.

Một attacker khai thác khoảng trống đó: connect với
`SNI: allowed.example.com` — thứ mà bất kỳ middlebox mạng, firewall dựa
trên SNI, hay tầng routing theo SNI nào thấy và cho phép — rồi gửi
`Host: internal.example.com` bên trong request đã mã hóa, thứ mà proxy
của bạn thực sự dùng để route. Đây là domain fronting, và nó biến một
access control dựa trên SNI thành đồ trang trí.

Nếu proxy của bạn terminate TLS và route theo `Host`, validate rằng cả
hai khớp nhau và reject khi chúng không khớp. Nếu một client hợp lệ thực
sự cần chúng khác nhau, điều đó nên là một cấu hình tường minh, không
phải một tai nạn.

Gotcha: đây là cùng kiểm tra mà [`03-router.md`](03-router.md) mô tả cho tính nhất quán
`Host`/`:authority`/SNI. Làm nó một lần, ở một chỗ, tại điểm mà metadata
TLS của connection vẫn còn sẵn cùng với request — không phải ở hai
component có thể bất đồng về cái nào là authoritative.

### Routing theo SNI (trước TLS, không cần giải mã)
ClientHello của TLS mang hostname đích ở dạng rõ trong SNI extension — kể cả dưới TLS 1.3, cho tới khi Encrypted Client Hello (ECH) trở nên phổ biến. Một proxy có thể chỉ nhìn vào ClientHello, đọc SNI, và quyết định *backend terminate-TLS nào* để forward byte vẫn còn mã hóa tới, mà không cần giữ private key của backend đó. Đây là cách một tầng passthrough đứng trước nhiều bộ terminate TLS độc lập (mỗi cái có chứng chỉ riêng) route traffic mà không trở thành bên thứ tư của phiên TLS.

Gotcha: nhìn trộm nghĩa là đọc byte từ socket mà sau đó bạn phải forward
*bao gồm* các byte bạn đã tiêu thụ — backend cần trọn vẹn ClientHello.
Buffer và replay nó thay vì tiêu thụ nó, và giới hạn cả buffer lẫn thời
gian bạn sẽ chờ một ClientHello hoàn chỉnh, nếu không một client connect
rồi gửi 3 byte mãi mãi là một vector làm cạn connection
([`07-security/09-ddos.md`](../07-security/09-ddos.md)).

Gotcha: SNI là tùy chọn. Một client connect bằng IP, một client cũ, hay
một probe cố ý có thể không gửi gì cả — quyết định đó là một backend mặc
định hay một sự reject, và nhận thức rằng "backend mặc định" là cùng rủi
ro rơi-xuống như một `Host` không khớp.

### Routing theo SNI và routing theo Host header không thể hoán đổi cho nhau
Routing theo SNI quyết định *trước* khi giải mã và chỉ thấy hostname — nó không thấy được path, method, hay bất kỳ header nào. Routing theo Host header quyết định *sau* khi giải mã và thấy toàn bộ request, nhưng đòi hỏi proxy làm việc routing đó cũng phải là bên terminate TLS (giữ chứng chỉ). Một proxy có thể làm một trong hai, hoặc cả hai theo trình tự (route theo SNI tới đúng instance terminate-TLS, cái đó rồi route theo Host tới đúng upstream pool của tenant) — biết cái nào một deployment cụ thể thực sự cần trước khi xây nó.

### Chọn chứng chỉ ở quy mô multi-tenant
Terminate TLS cho nhiều hostname nghĩa là chọn một chứng chỉ *trong lúc*
handshake, từ SNI, trước khi bạn biết bất cứ gì khác về request. rustls
phơi bày cái này như một callback resolver (`ResolvesServerCert`), và ba
mối quan tâm thực tế theo sau:
- **Callback nằm trên hot path của handshake.** Nạp và parse một chứng
  chỉ từ đĩa ở đó thêm latency vào mỗi connection mới; giữ chứng chỉ đã
  parse trong memory, key theo hostname, và reload khi config thay đổi
  ([`09-architecture/03-config.md`](../09-architecture/03-config.md)) thay vì mỗi lần handshake.
- **Wildcard và match chính xác phải có precedence rõ ràng.** Với cả
  `example.com` lẫn `*.example.com` được cấu hình, một match chính xác
  nên thắng; wildcard chỉ match đúng một label (`*.example.com` bao phủ
  `a.example.com` nhưng không phải `a.b.example.com`).
- **Hết hạn là theo từng tenant và im lặng.** Chứng chỉ hết hạn của một
  tenant chỉ làm fail handshake của tenant đó, nên traffic tổng thể
  trông vẫn ổn. Export thời gian-tới-khi-hết-hạn như một metric theo
  từng chứng chỉ ([`08-observability/06-alerting.md`](../08-observability/06-alerting.md)); đây là gotcha mTLS
  từ [`07-security/01-auth.md`](../07-security/01-auth.md) nhân lên theo số tenant.

### Chứng chỉ wildcard/multi-domain tương tác với cả hai
Một chứng chỉ wildcard (`*.example.com`) hay một chứng chỉ SAN bao phủ nhiều hostname cho phép một instance terminate-TLS trả lời cho nhiều vhost dưới một handshake — đơn giản hóa routing theo Host header (một chứng chỉ, nhiều giá trị `Host`) nhưng làm routing theo SNI trở nên vô nghĩa cho các hostname đó (chúng đều là cùng một backend theo định nghĩa). Xem [`01-network/13-tls.md`](../01-network/13-tls.md) cho cơ chế handshake mà điều này phụ thuộc vào.

### Cách ly giữa các tenant, không chỉ routing
Routing tách *traffic* của các tenant; nó không làm gì để tách *tiêu thụ tài nguyên* của chúng. Một đợt tăng traffic của một tenant tiêu thụ ngân sách connection chung ([`07-security/09-ddos.md`](../07-security/09-ddos.md)), concurrency của upstream pool chung, dung lượng cache chung ([`05-http-stack/07-cache.md`](07-cache.md)), và worker thread — nên mọi tenant khác đều tệ đi. Đó là vấn đề noisy-neighbor, và trong một proxy multi-tenant nó là hành vi mặc định trừ khi bạn thiết kế để chống lại nó.

Các cơ chế kiểm soát là phiên bản theo-từng-tenant của những thứ bạn đã
có: rate limit key theo tenant ([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)), một giới
hạn concurrency theo từng vhost để một tenant không thể giữ toàn bộ
connection upstream, và tính toán cache theo từng tenant để object lớn
của một tenant không evict working set của tenant khác.

Gotcha: làm identity của tenant thành một phần của mọi key xuyên suốt, và
làm điều đó sớm. Thêm chiều tenant vào cache key, nhãn metric
([`08-observability/02-metrics.md`](../08-observability/02-metrics.md)), và key rate-limit sau khi đã xây xong
là một thay đổi lớn, dễ sai — và kiểu sai đó là serve response đã cache
của một tenant cho tenant khác.

## Practice
Làm theo thứ tự này.

1. Trong [`proxy`](../../proxy), thêm một vhost map key theo `Host` đã normalize (viết
   thường, bỏ dấu chấm cuối, bỏ port với IPv6 được xử lý riêng), nạp từ
   config. **Xong khi** `EXAMPLE.com.`, `example.com:443`, và
   `example.com` đều resolve về cùng một tenant.
2. Reject các host không khớp một cách tường minh. **Xong khi** một
   request với `Host` không xác định nhận một response 404 hoặc
   catch-all và không bao giờ chạm tới backend của một tenant thật —
   xác minh bằng logging phía upstream, không chỉ góc nhìn của client.
3. Thêm validate tính nhất quán `Host`/SNI. **Xong khi** một connection
   với `SNI: a.example.com` mang `Host: b.example.com` bị reject; dùng
   `openssl s_client -servername` để dựng nó.
4. Thêm chọn chứng chỉ theo từng hostname qua một resolver rustls dựa
   trên một map trong memory. **Xong khi** hai hostname trình bày chứng
   chỉ khác nhau trên cùng một listener, match chính xác thắng wildcard,
   và resolver không truy cập filesystem nào mỗi lần handshake.
5. Export thời gian-tới-khi-hết-hạn theo từng chứng chỉ như một metric.
   **Xong khi** một chứng chỉ hết hạn trong 7 ngày thấy được mà không ai
   phải tự kiểm tra.
6. Thêm một test config-reload cho vhost map ([`labs/13-hot-reload`](../../labs/13-hot-reload)).
   **Xong khi** swap map dưới tải đồng thời thay đổi routing cho request
   mới mà không ảnh hưởng bất kỳ request nào đang bay.
7. Thêm giới hạn concurrency và rate limit theo từng tenant, với tenant
   là một nhãn trên metric và một thành phần của cache key. **Xong khi**
   một tenant đẩy một load test tới bão hòa để lại p99 của tenant khác
   không đổi — đo cả hai, vì đó chính là mục đích.
8. (Stretch) Implement passthrough nhìn-trộm-SNI: buffer ClientHello,
   trích SNI, forward byte thô bao gồm cả những gì bạn đã buffer. **Xong
   khi** một connection TLS terminate ở đúng backend mà proxy không bao
   giờ giữ private key của nó, và một client bị treo giữa ClientHello bị
   timeout thay vì bị giữ mãi.
