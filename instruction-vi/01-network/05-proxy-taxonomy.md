# Proxy Taxonomy: What Kind of Proxy Is This

Một phần của chuỗi fundamentals từ-con-số-0 — xem [`01-network/01-fundamentals.md`](01-fundamentals.md)
để có index đầy đủ. Cả repo này xây một điểm cụ thể trong không gian này;
file này trả lời thẳng "cái nào, và vì sao", dùng vốn từ vựng từ
[`02-addressing.md`](02-addressing.md) và [`03-byte-streams.md`](03-byte-streams.md).

## What to learn

### Forward proxy vs reverse proxy: nó phục vụ lợi ích của ai
Một **forward proxy** đứng trước *client*, thay mặt họ: một proxy web
outbound của công ty mà trình duyệt của mọi nhân viên được cấu hình để đi
qua, hay một dịch vụ kiểu-VPN giấu IP thật của client khỏi các site họ
truy cập. Client biết proxy tồn tại và được cấu hình để dùng nó; server
đích thường không biết có một proxy đang tham gia (hoặc chỉ thấy IP của
proxy như thể đó là client).

Một **reverse proxy** — thứ mà cả repo này xây dựng — đứng trước
*server*, thay mặt họ. Client kết nối tới proxy nghĩ rằng họ đang nói
chuyện với service thật; proxy quyết định backend thật nào sẽ xử lý
request. Backend, chứ không phải client, mới là bên proxy tồn tại để bảo
vệ và quản lý. Đây là mối quan hệ tin cậy ngược lại so với một forward
proxy, dù cơ chế ở tầng dây (accept một kết nối, parse, forward, relay
response) trông giống nhau.

Gotcha: "proxy" khi nói suông, trong giao tiếp thường ngày, có thể có
nghĩa là một trong hai — luôn xác định rõ ý nghĩa nào đang được nói tới từ
ngữ cảnh. Handbook này không bao giờ có ý nói tới loại forward.

### NAT gateway: dịch địa chỉ mà không hiểu protocol
Một NAT gateway ([`02-addressing.md`](02-addressing.md)) viết lại địa chỉ (SNAT/DNAT) mà không
hiểu gì về protocol chạy trên nó — nó không parse HTTP, không biết một
"request" là gì, và không thể đưa ra quyết định routing dựa trên một URL
path hay header. Nó chỉ hoạt động thuần túy trên IP/port. Đây cũng chính
là tầng mà một **L4 load balancer** thuần túy hoạt động: nó chọn một
backend dựa trên 4-tuple của kết nối (hoặc một hash của nó) rồi chỉ forward
byte, cả hai chiều, mà không nhìn vào bên trong chúng.

### L7 (reverse) proxy khác ở đâu
Một **L7 proxy** — [`proxy/`](../../proxy) của repo này — chấm dứt kết nối (kết thúc
phiên TCP/TLS của client ngay tại proxy), đọc và hiểu application
protocol bên trong nó (HTTP), và *rồi* mới quyết định phải làm gì: backend
nào, có cache hay không, có rate-limit hay không, có từ chối hay không.
Việc này đắt hơn hẳn cho mỗi request (parsing, có thể re-encrypt tới
backend) và mạnh hơn hẳn về khả năng — đó là toàn bộ lý do
[`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md) có thể route theo path hay header còn một
NAT gateway thì không, và toàn bộ lý do [`07-security/06-waf.md`](../07-security/06-waf.md) có thể
kiểm tra request body còn một L4 balancer thuần túy thì không.

```
L4 (NAT gateway / TCP load balancer):
  client --[TCP]--> [viết lại địa chỉ, forward byte] --[TCP]--> backend
  (không bao giờ parse bên trong; hai đầu của một kết nối hiệu quả duy nhất)

L7 (proxy của repo này):
  client --[TCP+TLS+HTTP]--> [chấm dứt, parse, quyết định] --[TCP+TLS+HTTP mới]--> backend
  (hai kết nối thực sự tách biệt; proxy đọc và có thể viết lại request)
```

### API gateway: một L7 reverse proxy nhiều "quan điểm" hơn
Một **API gateway** thường chỉ là một L7 reverse proxy với một bộ tính
năng cụ thể được gộp vào theo quy ước — auth, rate limiting theo từng
client, biến đổi request/response, đôi khi chuyển đổi protocol
(REST-to-gRPC). Không có ranh giới kỹ thuật cứng nào giữa "reverse proxy"
và "API gateway"; đó là một sự phân biệt về marketing/phạm vi. Mọi thứ
repo này xây trong [`07-security/`](../07-security) và [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md),
cộng dồn lại, chính là thứ biến một reverse proxy trần trụi thành thứ
người ta gọi là API gateway.

### CDN: một reverse proxy ở quy mô phân tán khổng lồ
Một **CDN** về mặt cấu trúc cũng là một reverse proxy — nó chấm dứt kết
nối của client và quyết định cách phục vụ chúng — chuyên biệt hóa cho một
việc: cache content gần người dùng qua nhiều điểm hiện diện địa lý, để
phần lớn request không bao giờ chạm tới origin. [`05-http-stack/07-cache.md`](../05-http-stack/07-cache.md)
và [`05-http-stack/08-cache-stampede.md`](../05-http-stack/08-cache-stampede.md) mô tả cơ chế caching mà một
instance proxy đơn lẻ cần; một CDN là cùng cơ chế đó nhân lên qua hàng
nghìn edge location cùng một cách để giữ chúng nhất quán (cuối cùng).

### Sidecar proxy: một reverse proxy cho mỗi process, không phải một cho cả hạm đội
Một **sidecar proxy** (Envoy trong một service mesh, ví dụ Istio) vẫn là
cùng cơ chế L7 reverse-proxy, chỉ triển khai khác đi: thay vì một proxy
dùng chung đứng trước cả một hạm đội service, mỗi instance service riêng
lẻ có một instance proxy tí hon của riêng nó chạy cạnh nó (cùng pod, theo
thuật ngữ Kubernetes — xem [`02-linux/06-containers.md`](../02-linux/06-containers.md)), xử lý traffic
inbound và outbound của riêng instance đó. Cơ chế mà handbook này dạy —
load balancing, retry, circuit breaking, mTLS — hoàn toàn giống nhau; chỉ
topology triển khai là khác.

### [`proxy/`](../../proxy) của repo này nằm ở đâu
Cụ thể: [`proxy/`](../../proxy) là một proxy **reverse, L7, terminating** — nó accept
kết nối client, hiểu HTTP, và forward các quyết định (không chỉ byte) tới
một upstream pool. Không có gì trong repo này xây một forward proxy, và
không có gì ở đây xây pure L4 forwarding như mục tiêu cuối cùng (dù
[`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md) và [`07-security/09-ddos.md`](../07-security/09-ddos.md) đều chạm tới
những mối quan tâm sát-L4 — accept-rate limiting, connection-level
shedding — vì một L7 proxy thật sự vẫn phải sống sót ở tầng kết nối trước
khi kịp parse bất cứ thứ gì).

## Practice
1. Với mỗi thứ dưới đây, gọi tên xem nó là forward proxy, reverse proxy,
   L4 gateway, hay không cái nào trong số đó, và giải thích vì sao: bộ
   lọc web của công ty bạn; nginx đứng trước một web app; NAT của router
   gia đình; Cloudflare đứng trước một website; một Envoy sidecar cạnh
   một microservice.
2. Giải thích, bằng lời của bạn, vì sao một L4 load balancer không thể
   implement kiểu routing consistent-hash-theo-session-cookie của
   [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md), nhưng [`proxy/`](../../proxy) thì có thể.
3. Đọc đoạn mở đầu của [`07-security/06-waf.md`](../07-security/06-waf.md) và giải thích một WAF phải
   hoạt động ở tầng nào (L4 hay L7), và vì sao — gắn câu trả lời của bạn
   với "chấm dứt kết nối" từ file này.
4. Vẽ (trên giấy, không phải code) topology kết nối cho một request đi
   qua một NAT router gia đình, một CDN, và cuối cùng là [`proxy/`](../../proxy) của
   repo này trước khi tới một application server — gắn nhãn mỗi hop là L4
   hay L7.
