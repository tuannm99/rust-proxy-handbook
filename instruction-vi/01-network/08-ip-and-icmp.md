# IP Layer: Packet, Routing, Fragmentation, ICMP

IP packet là gì, router quyết định đi tiếp về đâu như thế nào, chuyện gì xảy
ra khi packet quá lớn, và mạng báo lỗi ra sao. Xây trên
[`02-addressing.md`](02-addressing.md) (address, CIDR, NAT) và [`07-link-layer.md`](07-link-layer.md) (frame, ARP).

## What to learn

### IPv4 header: thứ router thực sự đọc
Một IPv4 header dài 20 byte (nhiều hơn nếu có options). Các field cần biết:

```text
 version | header len | DSCP/ECN | total length
 identification | flags (DF, MF) | fragment offset
 TTL | protocol (6=TCP, 17=UDP, 1=ICMP) | header checksum
 source IP
 destination IP
```

- **Total length** là kích thước packet gồm cả header, tối đa 65535.
- **TTL** (time to live) bị mỗi router giảm đi 1; về 0 thì packet bị drop và
  router báo lại. Nó ngăn routing loop làm packet quay vòng mãi — và là mẹo
  mà `traceroute` dùng.
- **Protocol** cho kernel bên nhận biết giao payload cho transport nào: TCP,
  UDP, ICMP.
- Với forward thông thường, router **không bao giờ nhìn quá IP header**, nên
  một thiết bị L3 không thể quyết định dựa trên HTTP path, thậm chí cả port
  (firewall hay NAT nhìn port là đang làm thêm việc stateful).

### Routing: longest-prefix match, từng hop một
Mọi host và router giữ một **routing table**: danh sách
`prefix -> next hop, interface`. Để forward một packet, nó tìm các entry có
prefix chứa địa chỉ đích và chọn entry **cụ thể nhất** (prefix dài nhất).
`0.0.0.0/0` là **default route**: "cái gì tôi không biết thì gửi cho
gateway."

```text
$ ip route
default via 10.0.0.1 dev eth0
10.0.0.0/24 dev eth0 proto kernel scope link src 10.0.0.5
172.17.0.0/16 dev docker0 scope link
```

Đích `10.0.0.9` khớp `/24` (trực tiếp, ARP cho nó); `8.8.8.8` chỉ khớp
default (ARP cho `10.0.0.1`, đưa packet cho nó). Routing diễn ra **theo từng
packet và từng hop**: không router nào biết toàn bộ đường đi, mỗi cái đưa ra
quyết định cục bộ, và đường đi với đường về có thể khác nhau (**asymmetric
routing**). Sự bất đối xứng vô hình với TCP nhưng làm hỏng stateful firewall
và NAT chỉ thấy một chiều. Một máy Linux chỉ forward packet giữa các
interface khi `net.ipv4.ip_forward=1` — chính cờ này biến nó thành router
(hoặc NAT gateway của một container host). Routing table được điền bằng tay,
bằng DHCP, hoặc bằng protocol (OSPF, BGP) — BGP là cách cả internet đồng ý
với nhau mạng nào nằm ở đâu, và **anycast** (cùng một IP được announce từ
nhiều nơi; routing chọn nơi gần nhất) là cách CDN và các DNS resolver công
cộng như `1.1.1.1` hoạt động.

### MTU, fragmentation, và Path MTU Discovery
MTU của một link giới hạn kích thước packet ([`07-link-layer.md`](07-link-layer.md)). Khi một
router phải forward packet lớn hơn MTU của link kế tiếp, IPv4 có hai kết quả
tùy bit **DF** (Don't Fragment):

- DF clear: router **fragment** packet; bên nhận ghép lại. Fragmentation
  chậm, mong manh (mất một fragment là mất cả packet) và là bề mặt tấn công
  kinh điển.
- DF set: router **drop** nó và gửi lại một ICMP "Fragmentation Needed" mang
  MTU của next hop.

TCP set DF và dùng phản hồi này — **Path MTU Discovery (PMTUD)** — để tìm MTU
nhỏ nhất trên đường đi và thu nhỏ segment size. IPv6 không có router
fragmentation; chỉ bên gửi mới được fragment.

**Gotcha — PMTUD black hole.** Nếu một firewall drop *toàn bộ* ICMP (kiểu
hardening sai lầm phổ biến), các message "too big" không bao giờ tới nơi.
Packet nhỏ (handshake, request ngắn) chạy được; segment full-size đầu tiên
của một response lớn biến mất và connection **treo** — triệu chứng kinh điển
"connect được, trang nhỏ load được, trang lớn thì đứng", nhất là qua VPN và
tunnel có MTU giảm. Cách sửa là cho phép ICMP type 3 code 4, hoặc **MSS
clamping** (router ghi đè MSS option trong SYN để hai đầu gửi segment đủ
nhỏ).

### ICMP: kênh báo lỗi và chẩn đoán của mạng
**ICMP** chạy bên trong IP (protocol 1) và mang control message, không phải
dữ liệu ứng dụng. Những loại cần nhận ra:

- **Echo request/reply** — `ping`.
- **Destination unreachable** — kèm code: network/host unreachable, port
  unreachable (một UDP packet đập vào port không có listener — đây là cách
  quan sát "connection refused" của UDP), fragmentation needed.
- **Time exceeded** — TTL về 0. `traceroute` gửi probe với TTL 1, 2, 3, ... và
  ghi lại router nào reply "time exceeded" ở mỗi bước, vẽ ra đường đi.
- **Redirect** — router báo cho bạn first hop tốt hơn (thường bị bỏ qua vì
  lý do bảo mật).

ICMP không phải đường ống tùy chọn: block sạch nó làm hỏng PMTUD và biến lỗi
thành thầm lặng thay vì nhanh.

### IPv6 trong một trang
Địa chỉ 128-bit (`2001:db8::1`), header cố định 40 byte không có checksum và
không có router fragmentation, `::1` là loopback, địa chỉ **link-local**
`fe80::/10` trên mọi interface, `/64` là kích thước subnet chuẩn, và Neighbor
Discovery thay cho ARP. Với proxy, hệ quả thực tế: listen trên `[::]` (một
dual-stack socket cũng nhận IPv4 trừ khi set `IPV6_V6ONLY`), parse
`[addr]:port` trong header `Host` và config, lưu client IP dưới dạng `IpAddr`
(không bao giờ `u32`), và chờ DNS answer của upstream chứa cả `A` lẫn `AAAA`
([`14-dns.md`](14-dns.md)). NAT hiếm khi cần vì địa chỉ dồi dào, nên client IP thật
thường nhìn thấy được từ đầu đến cuối.

### Gotcha: TTL và hop count như công cụ debug và bảo mật
Bên gửi bắt đầu TTL ở một giá trị quen thuộc (thường 64 trên Linux, 128 trên
Windows), nên TTL của packet nhận được cho biết đại khái nó đã qua bao nhiêu
hop. Một số protocol khai thác điều đó: một service chỉ nên nói chuyện với
peer on-link có thể đòi TTL 255 (mẹo "GTSM"), vì packet nào đã qua router sẽ
có giá trị thấp hơn. Và vì packet chỉ sống một số hop giới hạn, routing loop
hiện ra thành `traceroute` lặp lại cùng hai router — thứ mà nếu không sẽ chỉ
trông như "timeout".

## Practice

1. Chạy `ip route`, `ip route get 8.8.8.8` và `ip route get 10.0.0.5` (chỉnh
   theo subnet của bạn); với mỗi lệnh giải thích routing entry nào thắng và
   vì sao, gọi tên prefix length làm nó cụ thể nhất.
2. Chạy `ping -c3 -M do -s 1472 <host>` (DF set, 1472 + 28 byte header =
   1500), rồi `-s 1473` và đọc lỗi "Message too long"; việc này cho bạn tận
   mắt thấy path MTU. Lặp lại với một host qua VPN nếu bạn có.
3. Chạy `traceroute -n 8.8.8.8` (hoặc `mtr -n -c 10 8.8.8.8`) và giải thích
   từng dòng theo TTL và ICMP time-exceeded. Bắt bằng
   `sudo tcpdump -n -i any icmp` trong lúc chạy và tìm các probe và reply.
4. Chạy `sysctl net.ipv4.ip_forward` và giải thích điều gì thay đổi nếu set
   thành 1; tìm xem thành phần nào của Docker/Kubernetes set nó trên một
   container host và vì sao.
5. Connect tới một UDP port không có gì listen
   (`echo hi | nc -u -w1 127.0.0.1 9999`) khi `sudo tcpdump -n -i lo icmp`
   đang chạy, và tìm ICMP port-unreachable reply.
6. Chạy `ip -6 addr` và `curl -6 -v http://[::1]:8080/` vào
   [`labs/00-tcp-server`](../../labs/00-tcp-server) sau khi cho nó listen trên `[::]`; xác nhận một
   IPv4 client (`curl -4 http://127.0.0.1:8080/`) có connection được không, và
   giải thích từ `IPV6_V6ONLY` vì sao có hoặc không.
