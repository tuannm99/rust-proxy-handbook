# Addressing: IP, Port, CIDR, NAT

File này thuộc nhóm fundamentals. Xem [`01-network/01-fundamentals.md`](01-fundamentals.md) để có mục lục đầy đủ.
Nó trả lời câu hỏi: làm sao để định danh một process cụ thể trên một máy cụ thể, đủ rõ để một packet tìm đúng nó?

## What to learn

### IP address định danh một host
Một **IP address** định danh một host, tức một máy cụ thể trên mạng (chính xác hơn là một network interface cụ
thể). Nó trả lời câu hỏi "máy nào". Các router dọc đường dùng nó để quyết định chuyển packet đi hướng nào, từng hop
một. Router không cần biết gì về nội dung bên trong packet.

### Port định danh một process trên host đó
Một **port** là một số 16-bit (0–65535). Nó định danh một chương trình cụ thể đang listen *trên* host đó, và trả lời
câu hỏi "chương trình nào trên máy đó". Một máy có thể chạy web server ở port 443 và SSH daemon ở port 22 cùng lúc.
IP address đưa packet tới đúng *máy*, còn port đưa nó tới đúng *chương trình*.

Các port dưới 1024 là "well-known" (80, 443, 22, 53 tương ứng HTTP, HTTPS, SSH, DNS). Trên Unix, theo quy ước bạn cần
quyền cao hơn để bind chúng. Vì vậy một proxy thường chọn một trong các cách sau:
- Chạy bằng root trong một thời gian ngắn để bind port 443, rồi bỏ quyền (drop privileges).
- Bind một port cao, và nhờ thứ khác (load balancer, `iptables`, một capability được cấp riêng) đưa traffic tới nó.

Các port từ khoảng 32768 trở lên (dải chính xác cấu hình được, xem `/proc/sys/net/ipv4/ip_local_port_range`) là
**ephemeral**. OS tự động gán một port trong dải này cho phía *client* của một connection đi ra, và thu hồi khi
connection đóng.

Gotcha thật trong production: một proxy mở nhiều connection đi ra, ngắn hạn, tới cùng một upstream có thể dùng hết pool
ephemeral port của chính nó (mặc định khoảng 28.000 port). Nó dùng nhanh hơn tốc độ `TIME_WAIT` trả port lại. Đây là
lý do [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) và [`01-network/12-tcp.md`](12-tcp.md) nhấn mạnh việc
reuse connection thay vì dial connection mới cho mỗi request.

### Socket chính xác là gì
Một **socket** là sự kết hợp của một IP address và một port, đại diện cho một đầu của một connection.

Một connection TCP thực ra được định danh bởi *bốn* giá trị cùng nhau, gọi là "4-tuple": source IP, source port,
destination IP, destination port. Vì vậy một server process listen trên một port có thể phục vụ hàng nghìn client cùng
lúc. 4-tuple của mỗi client khác nhau, dù IP và port của server cố định.

[`01-network/11-socket.md`](11-socket.md) nói về API tạo ra socket.

### CIDR notation: mô tả một dải địa chỉ
Một IPv4 address có 32 bit, viết thành bốn số thập phân (`192.0.2.1`). **CIDR notation** (`10.0.0.0/8`) mô tả một
*dải* địa chỉ. Số sau dấu gạch chéo là số bit đầu tiên bị cố định (phần "network"). Các bit còn lại được tự do (phần
"host").

```
10.0.0.0/8   -> 8 bit đầu cố định  -> 2^24 địa chỉ (toàn bộ 10.x.x.x)
10.0.0.0/24  -> 24 bit đầu cố định -> 2^8 địa chỉ  (10.0.0.0-10.0.0.255)
10.0.0.5/32  -> cả 32 bit cố định  -> đúng một địa chỉ
```

IPv6 address dài 128 bit, viết thành tám nhóm số hex (`2001:db8::1`, trong đó `::` thay cho một chuỗi nhóm toàn số 0).
Nó dùng cùng cú pháp gạch chéo (`2001:db8::/64`). Bạn sẽ cần thạo phần này cho
[`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md) (allow/deny list) và
[`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md). Ở file sau, bạn sẽ thấy vì sao nếu rate limiter lấy cả
một IPv6 address đầy đủ làm key, kẻ tấn công có 2^64 danh tính miễn phí trong chính `/64` của họ.

### NAT: địa chỉ bạn thấy không phải lúc nào cũng là địa chỉ đã được gửi
**Network Address Translation (NAT)** viết lại địa chỉ nguồn và/hoặc địa chỉ đích của một packet khi nó đi qua một
thiết bị. Nhờ đó nhiều máy có thể dùng chung một địa chỉ, hoặc một địa chỉ private có thể ra internet công cộng. Có
hai dạng quan trọng ở đây:

- **SNAT (source NAT)**: phổ biến ở router gia đình và CGNAT. Thiết bị viết lại source address của nhiều client nội bộ
  thành một IP công cộng khi traffic đi ra, và viết lại reply theo chiều ngược lại. Nó ghi nhớ client nội bộ nào sở hữu
  connection đi ra nào. Từ góc nhìn của server, hàng nghìn user gia đình đứng sau CGNAT của cùng một ISP có thể đều
  hiện ra với cùng *một* source IP.
- **DNAT (destination NAT)**: thứ mà load balancer hoặc Kubernetes Service làm. Một packet gửi tới một IP công cộng
  hoặc IP ảo bị viết lại destination thành backend thật sẽ xử lý nó, trước khi proxy của bạn kịp thấy.

Vì sao điều này quan trọng với proxy: khi một connection tới listening socket của bạn, `peer_addr()` có thể đã cách
client thật vài hop NAT. Đây *chính xác* là vấn đề mà [`01-network/20-proxy-protocol.md`](20-proxy-protocol.md) và phần
`X-Forwarded-For` trong [`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md) giải quyết. Cũng vì vậy,
cứ tin vào peer address của socket là ngây thơ, ngay khi có bất kỳ load balancer, NAT gateway hay CDN nào đứng trước
bạn.

### Routing, nói sơ lược
Nếu đích của packet không nằm trong mạng nội bộ, nó được gửi tới một **gateway** ("default route"). Đó là một router
biết cách đưa packet gần đích hơn một hop (hoặc biết nên hỏi ai). Máy của bạn không cần bản đồ đầy đủ của internet. Nó
chỉ cần biết default gateway của mình. Mỗi router dọc đường đưa ra cùng một quyết định cục bộ, chỉ cho một hop.

`ip route` (Linux) hoặc `route -n` cho thấy routing table của máy bạn. `traceroute`/`mtr` cho thấy đường đi thực tế, từng
hop, mà một packet đi qua để tới đích. Đây là kiến thức nền để hiểu *vì sao* latency cộng dồn theo từng hop
([`04-latency-throughput.md`](04-latency-throughput.md)). Bạn sẽ không tự implement routing. Routing hoàn toàn là việc
của kernel và router, không phải của application.

## Practice
1. Chạy `ip addr` (hoặc `ifconfig`) và xác định (các) IP address của máy bạn. Chạy `ip route` và xác định default
   gateway.
2. Chạy `traceroute example.com` (hoặc `mtr` để xem trực tiếp) và đếm số hop. So sánh số hop đó với RTT bạn đã đo ở
   bài tập của [`04-latency-throughput.md`](04-latency-throughput.md).
3. Tính bằng tay xem `10.0.0.0/8` và `2001:db8::/64` mỗi cái bao gồm bao nhiêu địa chỉ, rồi xác nhận bằng một CIDR
   calculator.
4. Nếu bạn đang đứng sau NAT (hầu như ai ở nhà cũng vậy), mở một trang "what's my IP" và so sánh địa chỉ nó báo với
   địa chỉ máy bạn ở bước 1. Hai địa chỉ sẽ khác nhau. Khoảng cách đó chính là SNAT của router bạn đang hoạt động.
5. Trong [`labs/00-tcp-server`](../../labs/00-tcp-server), connect hai client khác nhau cùng lúc và log đầy đủ 4-tuple
   của mỗi connection (`local_addr()` + `peer_addr()`). Xác nhận chúng chỉ khác nhau ở source port, nếu cả hai client
   nằm trên cùng một máy.
