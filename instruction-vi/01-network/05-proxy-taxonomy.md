# Proxy Taxonomy: Đây Là Loại Proxy Nào

File này thuộc nhóm fundamentals. Xem [`01-network/01-fundamentals.md`](01-fundamentals.md) để có mục lục đầy đủ. Cả
repo này xây một điểm cụ thể trong không gian các loại proxy. File này trả lời thẳng câu hỏi "là loại nào, và vì sao",
dùng vốn từ trong [`02-addressing.md`](02-addressing.md) và [`03-byte-streams.md`](03-byte-streams.md).

## What to learn

### Forward proxy và reverse proxy: nó phục vụ lợi ích của ai
Một **forward proxy** đứng trước *client* và hoạt động thay mặt họ. Ví dụ: proxy web đi ra của một công ty, mà trình
duyệt của mọi nhân viên được cấu hình để đi qua; hoặc một dịch vụ kiểu VPN giấu IP thật của client khỏi các site họ
truy cập. Client biết proxy tồn tại và được cấu hình để dùng nó. Server đích thường không biết có proxy (hoặc chỉ thấy
IP của proxy như thể đó là client).

Một **reverse proxy**, thứ mà cả repo này xây, đứng trước *server* và hoạt động thay mặt họ. Client connect tới proxy
và nghĩ rằng mình đang nói chuyện với service thật. Proxy quyết định backend nào sẽ xử lý request. Bên được proxy bảo
vệ và quản lý là backend, không phải client. Quan hệ tin cậy ngược với forward proxy, dù cơ chế trên dây (accept
connection, parse, forward, relay response) trông giống nhau.

Gotcha: khi ai đó nói "proxy" mà không nói rõ, có thể là một trong hai loại. Hãy luôn xác định từ ngữ cảnh xem là loại
nào. Handbook này không bao giờ nói đến loại forward.

### NAT gateway: dịch địa chỉ mà không hiểu protocol
Một NAT gateway ([`02-addressing.md`](02-addressing.md)) viết lại địa chỉ (SNAT/DNAT) mà không hiểu protocol chạy trên
đó. Nó không parse HTTP, không biết "request" là gì, và không thể quyết định routing dựa trên URL path hay header. Nó
chỉ làm việc thuần túy với IP/port.

Đây cũng chính là layer mà một **L4 load balancer** thuần túy làm việc. Nó chọn backend dựa trên 4-tuple của connection
(hoặc một hash của nó), rồi chỉ forward byte theo cả hai chiều mà không nhìn vào bên trong.

### L7 (reverse) proxy khác ở đâu
Một **L7 proxy**, tức [`proxy/`](../../proxy) của repo này, **kết thúc connection**: phiên TCP/TLS của client kết thúc
ngay tại proxy. Nó đọc và hiểu application protocol bên trong (HTTP), và *sau đó* mới quyết định: dùng backend nào, có
cache không, có rate-limit không, có từ chối không.

Việc này tốn kém hơn nhiều cho mỗi request (phải parse, có thể phải mã hóa lại khi gửi tới backend), nhưng mạnh hơn
nhiều. Đó là toàn bộ lý do [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md) route được theo path hay
header còn NAT gateway thì không. Cũng là lý do [`07-security/06-waf.md`](../07-security/06-waf.md) kiểm tra được request
body còn L4 balancer thuần túy thì không.

```
L4 (NAT gateway / TCP load balancer):
  client --[TCP]--> [viết lại địa chỉ, forward byte] --[TCP]--> backend
  (không bao giờ parse bên trong; hai đầu hiệu quả là một connection duy nhất)

L7 (proxy của repo này):
  client --[TCP+TLS+HTTP]--> [kết thúc, parse, quyết định] --[TCP+TLS+HTTP mới]--> backend
  (hai connection thật sự tách biệt; proxy đọc và có thể viết lại request)
```

### API gateway: L7 reverse proxy, thêm một số tính năng đi kèm
Một **API gateway** thường chỉ là một L7 reverse proxy, kèm một bộ tính năng cụ thể được gộp vào theo quy ước: auth,
rate limiting theo từng client, biến đổi request/response, đôi khi chuyển đổi protocol (REST sang gRPC). Không có ranh
giới kỹ thuật cứng giữa "reverse proxy" và "API gateway". Đó là sự phân biệt về marketing và phạm vi. Mọi thứ repo này
xây trong [`07-security/`](../07-security) và [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md), cộng
lại, chính là thứ biến một reverse proxy trần thành cái người ta gọi là API gateway.

### CDN: reverse proxy ở quy mô phân tán rất lớn
Một **CDN** về cấu trúc cũng là reverse proxy: nó kết thúc connection của client và quyết định cách phục vụ. Nó chuyên
biệt hóa cho một việc: cache nội dung gần người dùng, qua nhiều điểm hiện diện về địa lý, để phần lớn request không bao
giờ chạm tới origin.

[`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md) và
[`05-http-stack/09-cache-stampede.md`](../05-http-stack/09-cache-stampede.md) mô tả cơ chế caching mà một instance proxy
đơn lẻ cần. CDN là cùng cơ chế đó, nhân lên qua hàng nghìn edge location, kèm cách giữ chúng nhất quán (eventual
consistency).

### Sidecar proxy: mỗi process một reverse proxy, thay vì một proxy cho cả hệ thống
Một **sidecar proxy** (Envoy trong service mesh, ví dụ Istio) vẫn là cùng cơ chế L7 reverse proxy, chỉ triển khai khác.
Thay vì một proxy dùng chung đứng trước cả nhóm service, mỗi instance service có một instance proxy nhỏ riêng chạy cạnh
nó (cùng pod, theo thuật ngữ Kubernetes, xem [`02-linux/13-containers.md`](../02-linux/13-containers.md)). Proxy đó xử
lý traffic đi vào và đi ra của đúng instance này.

Cơ chế handbook này dạy (load balancing, retry, circuit breaking, mTLS) hoàn toàn giống nhau. Chỉ topology triển khai
khác.

### [`proxy/`](../../proxy) của repo này nằm ở đâu
Cụ thể: [`proxy/`](../../proxy) là một proxy **reverse, L7, kết thúc connection**. Nó accept connection của client, hiểu
HTTP, và đưa ra quyết định (không chỉ chuyển byte) về việc forward tới upstream pool nào.

Không có gì trong repo này xây forward proxy. Cũng không có gì xây pure L4 forwarding làm mục tiêu cuối. Tuy vậy,
[`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md) và [`07-security/09-ddos.md`](../07-security/09-ddos.md)
có chạm tới các mối quan tâm sát L4 (giới hạn tốc độ accept, shedding ở mức connection). Lý do: một L7 proxy thật vẫn
phải sống sót ở mức connection trước khi kịp parse bất cứ thứ gì.

## Practice
1. Với mỗi thứ dưới đây, gọi tên xem nó là forward proxy, reverse proxy, L4 gateway, hay không phải cái nào trong số đó,
   và giải thích vì sao: bộ lọc web của công ty bạn; nginx đứng trước một web app; NAT của router gia đình; Cloudflare
   đứng trước một website; một Envoy sidecar cạnh một microservice.
2. Giải thích bằng lời của bạn vì sao một L4 load balancer không thể làm kiểu routing consistent-hash theo session cookie
   của [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md), nhưng [`proxy/`](../../proxy) thì làm được.
3. Đọc đoạn mở đầu của [`07-security/06-waf.md`](../07-security/06-waf.md) và giải thích một WAF phải hoạt động ở layer
   nào (L4 hay L7), và vì sao. Gắn câu trả lời của bạn với ý "kết thúc connection" trong file này.
4. Vẽ (trên giấy, không phải code) topology connection cho một request đi qua một NAT router gia đình, một CDN, và cuối
   cùng là [`proxy/`](../../proxy) của repo này, trước khi tới application server. Gắn nhãn mỗi hop là L4 hay L7.
