# Addressing: IP, Ports, CIDR, NAT

Một phần của chuỗi fundamentals từ-con-số-0 — xem [`01-network/01-fundamentals.md`](01-fundamentals.md)
để có index đầy đủ. File này nói về cách một process cụ thể trên một máy
cụ thể được định danh đủ rõ để một packet có thể tìm thấy nó.

## What to learn

### IP address định danh một host
Một **IP address** định danh một host — một máy cụ thể (chính xác hơn, một
network interface cụ thể) trên mạng. Nó trả lời câu hỏi "máy tính nào".
Router dọc đường đi dùng nó để quyết định chuyển packet theo hướng nào,
từng hop một, mà không cần biết gì về nội dung bên trong packet.

### Port định danh một process trên host đó
Một **port** là một số 16-bit (0-65535) định danh một chương trình đang
lắng nghe cụ thể *trên* host đó. Nó trả lời câu hỏi "chương trình nào trên
máy tính đó". Một máy có thể chạy web server trên port 443 và SSH daemon
trên port 22 cùng lúc — IP address đưa packet tới đúng *máy*, port đưa nó
tới đúng *chương trình*.

Các port dưới 1024 là "well-known" (80, 443, 22, 53 — HTTP, HTTPS, SSH,
DNS) và theo quy ước cần quyền nâng cao để bind trên Unix, đó là lý do một
proxy thường hoặc chạy với quyền root trong thời gian ngắn để bind port
443 rồi drop quyền, hoặc bind một port cao và dựa vào thứ khác (một load
balancer, `iptables`, một capability grant) để đưa traffic tới nó.

Các port trên khoảng 32768 (dải chính xác có thể cấu hình, xem
`/proc/sys/net/ipv4/ip_local_port_range`) là **ephemeral**: OS tự động
gán một port cho phía *client* của một kết nối đi ra, chọn từ pool đó và
giải phóng khi kết nối đóng. Gotcha, và là một gotcha thật trong production:
một proxy mở nhiều kết nối outbound ngắn hạn tới cùng một upstream có thể
cạn kiệt pool ephemeral port của chính nó (mặc định khoảng 28,000 port khả
dụng) nhanh hơn tốc độ `TIME_WAIT` giải phóng chúng — đây là lý do thực tế
vì sao [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) và [`01-network/12-tcp.md`](12-tcp.md) nhấn mạnh việc
tái sử dụng kết nối thay vì dial mới cho mỗi request.

### Một socket, chính xác là gì
Một **socket** là sự kết hợp: một IP address cộng một port, đại diện cho
một đầu của một kết nối. Một kết nối TCP thực ra được định danh bởi *bốn*
giá trị cùng nhau (gọi là "4-tuple"): source IP, source port, destination
IP, destination port. Đó là lý do một server process lắng nghe trên một
port có thể phục vụ hàng nghìn client đồng thời — 4-tuple của mỗi client
khác nhau dù IP và port của server là cố định. [`01-network/11-socket.md`](11-socket.md)
nói về API thực sự tạo ra một socket.

### CIDR notation: mô tả một dải địa chỉ
Một IPv4 address là 32 bit, viết dưới dạng bốn octet thập phân
(`192.0.2.1`). **CIDR notation** (`10.0.0.0/8`) mô tả một *dải*: số sau dấu
gạch chéo là số bit đầu bị cố định (phần "network"), phần còn lại được tự
do (phần "host").

```
10.0.0.0/8   -> 8 bit đầu cố định  -> 2^24 địa chỉ (toàn bộ 10.x.x.x)
10.0.0.0/24  -> 24 bit đầu cố định -> 2^8 địa chỉ  (10.0.0.0-10.0.0.255)
10.0.0.5/32  -> cả 32 bit cố định  -> đúng một địa chỉ
```

IPv6 address là 128 bit, viết dưới dạng tám nhóm chữ số hex
(`2001:db8::1`, với `::` gom một chuỗi nhóm toàn số 0), và dùng cùng cú
pháp dấu gạch chéo (`2001:db8::/64`). Bạn sẽ cần dùng thành thạo cái này
cho [`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md) (allow/deny list) và
[`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md) (vì sao key một rate limiter theo cả một IPv6
address đầy đủ trao cho attacker 2^64 danh tính miễn phí trong chính `/64`
của họ — nói kỹ ở đó).

### NAT: địa chỉ bạn thấy không phải lúc nào cũng là địa chỉ đã được gửi
**Network Address Translation (NAT)** viết lại địa chỉ nguồn và/hoặc đích
của một packet khi nó đi qua một thiết bị, để nhiều địa chỉ thật có thể
dùng chung một địa chỉ, hoặc một địa chỉ private có thể vươn ra internet
công cộng. Hai dạng quan trọng ở đây:

- **SNAT (source NAT)**, trường hợp phổ biến ở router gia đình/CGNAT: một
  thiết bị viết lại source address của nhiều client nội bộ thành một IP
  công cộng khi traffic đi ra, và viết lại reply theo chiều ngược lại,
  theo dõi client nội bộ nào sở hữu kết nối outbound nào. Từ góc nhìn của
  server, hàng nghìn user gia đình khác nhau đứng sau CGNAT của cùng một
  ISP có thể đều xuất hiện như cùng *một* source IP.
- **DNAT (destination NAT)**, thứ mà một load balancer hoặc một
  Kubernetes Service làm: một packet đến, gửi tới một IP công cộng/ảo, bị
  viết lại destination thành bất kỳ backend thật nào sẽ xử lý nó, trước
  khi proxy của bạn kịp thấy nó.

Vì sao điều này quan trọng với riêng một proxy: đến lúc một kết nối tới
được listening socket của bạn, `peer_addr()` có thể đã cách client thật
vài hop NAT — đây *chính xác* là vấn đề mà [`01-network/20-proxy-protocol.md`](20-proxy-protocol.md)
và phần thảo luận `X-Forwarded-For` trong [`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md)
tồn tại để giải quyết, và đó là lý do "cứ tin vào peer address của socket"
là ngây thơ ngay khi có bất kỳ load balancer, NAT gateway, hay CDN nào ở
phía trước bạn.

### Routing, sơ lược
Một packet có đích không nằm trong mạng nội bộ sẽ được gửi tới một
**gateway** ("default route") — một router biết (hoặc biết phải hỏi ai)
cách đưa packet gần đích hơn một hop. Máy của bạn không cần một bản đồ
đầy đủ của internet; nó chỉ cần biết default gateway của mình, và mỗi
router dọc đường đưa ra cùng một quyết định cục bộ, chỉ một hop. `ip
route` (Linux) hoặc `route -n` cho thấy routing table của máy bạn;
`traceroute`/`mtr` cho thấy đường đi thực tế, từng hop, mà một packet đi
qua để tới đích. Đây là kiến thức nền để hiểu *vì sao* latency tích lũy
theo từng hop ([`04-latency-throughput.md`](04-latency-throughput.md)) chứ không phải thứ bạn sẽ tự
implement — routing hoàn toàn là việc của kernel/router, không bao giờ là
việc của application.

## Practice
1. Chạy `ip addr` (hoặc `ifconfig`) và xác định (các) IP address của máy
   bạn; chạy `ip route` và xác định default gateway của bạn.
2. Chạy `traceroute example.com` (hoặc `mtr` để xem trực tiếp) và đếm số
   hop; so sánh số hop đó với RTT bạn đã đo trong bài tập của
   [`04-latency-throughput.md`](04-latency-throughput.md).
3. Tính bằng tay xem `10.0.0.0/8` và `2001:db8::/64` mỗi cái bao phủ bao
   nhiêu địa chỉ, rồi xác nhận bằng một CIDR calculator.
4. Nếu bạn đang đứng sau NAT (hầu như ai ở nhà cũng vậy), truy cập một
   trang "what's my IP" và so sánh địa chỉ nó báo với địa chỉ máy bạn ở
   bước 1 — chúng sẽ khác nhau; khoảng cách đó chính là SNAT của router
   bạn đang hoạt động.
5. Trong [`labs/00-tcp-server`](../../labs/00-tcp-server), kết nối hai client khác nhau cùng lúc và
   log đầy đủ 4-tuple của mỗi kết nối (`local_addr()` + `peer_addr()`) —
   xác nhận chúng chỉ khác nhau ở source port nếu cả hai client cùng nằm
   trên một máy.
