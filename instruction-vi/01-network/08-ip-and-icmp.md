# IP Layer: Packet, Routing, Fragmentation, ICMP

File này trả lời bốn câu hỏi: IP packet là gì, router quyết định chuyển packet đi
đâu như thế nào, chuyện gì xảy ra khi packet quá lớn, và mạng báo lỗi bằng cách nào.
Nó dựa trên [`02-addressing.md`](02-addressing.md) (address, CIDR, NAT) và
[`07-link-layer.md`](07-link-layer.md) (frame, ARP).

## What to learn

### IPv4 header: những gì router thực sự đọc
Một IPv4 header dài 20 byte (dài hơn nếu có options). Các field cần biết:

```text
 version | header len | DSCP/ECN | total length
 identification | flags (DF, MF) | fragment offset
 TTL | protocol (6=TCP, 17=UDP, 1=ICMP) | header checksum
 source IP
 destination IP
```

- **Total length** là kích thước cả packet, tính cả header. Tối đa là 65535.
- **TTL** (time to live) bị mỗi router trừ đi 1. Khi về 0, packet bị drop và router
  gửi thông báo lại cho bên gửi. TTL ngăn packet quay vòng mãi khi có routing loop,
  và `traceroute` cũng dựa vào nó.
- **Protocol** cho kernel bên nhận biết phải giao payload cho transport nào: TCP, UDP
  hay ICMP.
- Khi forward bình thường, router **không bao giờ đọc quá IP header**. Vì vậy một
  thiết bị L3 không thể quyết định dựa trên HTTP path, thậm chí không dựa trên port.
  Firewall hay NAT mà nhìn port là đang làm thêm việc stateful.

### Routing: longest-prefix match, từng hop một
Mọi host và router đều có một **routing table**: danh sách các dòng
`prefix -> next hop, interface`. Để forward một packet, nó tìm các dòng có prefix
chứa địa chỉ đích, và chọn dòng **cụ thể nhất**, tức là prefix dài nhất.
`0.0.0.0/0` là **default route**, nghĩa là "địa chỉ nào tôi không biết thì gửi cho
gateway".

```text
$ ip route
default via 10.0.0.1 dev eth0
10.0.0.0/24 dev eth0 proto kernel scope link src 10.0.0.5
172.17.0.0/16 dev docker0 scope link
```

Đích `10.0.0.9` khớp với `/24`. Đây là đích trực tiếp nên máy gửi ARP hỏi chính nó.
Đích `8.8.8.8` chỉ khớp default route, nên máy gửi ARP hỏi `10.0.0.1` rồi đưa packet
cho gateway này.

Routing diễn ra **theo từng packet và từng hop**. Không router nào biết toàn bộ đường
đi. Mỗi router tự quyết định cục bộ. Đường đi và đường về có thể khác nhau (gọi là
**asymmetric routing**). TCP không nhận ra điều này, nhưng nó làm hỏng stateful
firewall và NAT chỉ nhìn thấy một chiều.

Máy Linux chỉ chuyển packet giữa các interface khi `net.ipv4.ip_forward=1`. Chính cờ
này biến một máy thành router (hoặc NAT gateway của một container host).

Routing table được điền bằng tay, bằng DHCP, hoặc bằng routing protocol như OSPF và
BGP. BGP là cách cả internet thống nhất với nhau mạng nào nằm ở đâu. **Anycast** là
kỹ thuật announce cùng một IP từ nhiều nơi, và routing sẽ chọn nơi gần nhất. CDN và
các DNS resolver công cộng như `1.1.1.1` hoạt động theo cách này.

### MTU, fragmentation và Path MTU Discovery
MTU của một link giới hạn kích thước packet ([`07-link-layer.md`](07-link-layer.md)).
Khi router phải forward một packet lớn hơn MTU của link kế tiếp, IPv4 xử lý theo bit
**DF** (Don't Fragment) của packet:

- DF = 0: router **fragment** packet, tức là cắt nhỏ ra, và bên nhận ghép lại.
  Fragmentation chậm và mong manh (mất một fragment là mất cả packet), và là bề mặt
  tấn công kinh điển.
- DF = 1: router **drop** packet và gửi lại một ICMP "Fragmentation Needed" kèm MTU
  của next hop.

TCP luôn set DF và dùng ICMP phản hồi này để tìm MTU nhỏ nhất trên đường đi, rồi giảm
segment size. Cơ chế này gọi là **Path MTU Discovery (PMTUD)**. IPv6 không cho router
fragment. Chỉ bên gửi mới được fragment.

**Gotcha: PMTUD black hole.** Giả sử một firewall drop *toàn bộ* ICMP (kiểu "hardening"
sai lầm khá phổ biến). Khi đó các message "too big" không bao giờ tới được bên gửi.
Packet nhỏ (handshake, request ngắn) vẫn đi qua bình thường. Nhưng segment full-size
đầu tiên của một response lớn biến mất, và connection **bị treo**. Triệu chứng kinh
điển là: "connect được, trang nhỏ load được, trang lớn thì đứng". Lỗi này hay gặp qua
VPN và tunnel vì chúng có MTU nhỏ hơn.

Có hai cách sửa: cho phép ICMP type 3 code 4 đi qua, hoặc dùng **MSS clamping**. MSS
clamping là để router ghi đè MSS option trong gói SYN, để hai đầu chủ động gửi segment
đủ nhỏ.

### ICMP: kênh báo lỗi và chẩn đoán của mạng
**ICMP** chạy bên trong IP (protocol 1). Nó mang control message chứ không mang dữ
liệu ứng dụng. Các loại cần nhận ra:

- **Echo request/reply**: chính là `ping`.
- **Destination unreachable**, kèm code cho biết lý do: network unreachable, host
  unreachable, port unreachable, fragmentation needed. Port unreachable xảy ra khi một
  UDP packet tới port không có ai listen. Đó là cách "connection refused" của UDP xuất
  hiện.
- **Time exceeded**: TTL đã về 0. `traceroute` gửi probe với TTL = 1, 2, 3, ... và ghi
  lại router nào trả lời "time exceeded" ở mỗi bước. Từ đó nó vẽ ra được đường đi.
- **Redirect**: router báo cho bạn biết có first hop tốt hơn. Thường bị bỏ qua vì lý do
  bảo mật.

ICMP không phải thứ có cũng được, không có cũng không sao. Chặn hoàn toàn ICMP làm hỏng
PMTUD, và biến lỗi từ chỗ báo ngay thành chỗ im lặng.

### IPv6 trong một trang
- Địa chỉ dài 128 bit, ví dụ `2001:db8::1`.
- Header cố định 40 byte, không có checksum, và không có router fragmentation.
- `::1` là loopback.
- Địa chỉ **link-local** `fe80::/10` có trên mọi interface.
- `/64` là kích thước subnet chuẩn.
- Neighbor Discovery thay cho ARP.

Với proxy, các hệ quả thực tế là:
- Listen trên `[::]`. Một dual-stack socket cũng nhận được IPv4, trừ khi set
  `IPV6_V6ONLY`.
- Parse được dạng `[addr]:port` trong header `Host` và trong config.
- Lưu client IP bằng kiểu `IpAddr`, không bao giờ dùng `u32`.
- Chấp nhận DNS answer của upstream chứa cả `A` lẫn `AAAA` ([`14-dns.md`](14-dns.md)).

IPv6 hiếm khi cần NAT vì địa chỉ dồi dào, nên IP thật của client thường nhìn thấy được
từ đầu đến cuối.

### Gotcha: TTL và hop count là công cụ debug và bảo mật
Bên gửi thường bắt đầu TTL ở một giá trị quen thuộc (64 trên Linux, 128 trên Windows).
Vì vậy TTL của packet nhận được cho biết đại khái nó đã đi qua bao nhiêu hop.

Một số protocol tận dụng điều này. Ví dụ, một service chỉ muốn nói chuyện với peer
on-link có thể yêu cầu TTL = 255 (kỹ thuật "GTSM"). Packet nào đã đi qua router thì TTL
chắc chắn thấp hơn 255, nên bị loại.

Vì mỗi packet chỉ sống được một số hop giới hạn, routing loop hiện ra thành một
`traceroute` lặp lại cùng hai router. Nếu không có `traceroute`, lỗi này chỉ trông như
"timeout".

## Practice

1. Chạy `ip route`, `ip route get 8.8.8.8` và `ip route get 10.0.0.5` (sửa lại cho
   khớp subnet của bạn). Với mỗi lệnh, giải thích dòng routing nào được chọn và vì sao,
   chỉ rõ prefix length nào làm nó cụ thể nhất.
2. Chạy `ping -c3 -M do -s 1472 <host>` (DF được set, 1472 + 28 byte header = 1500),
   rồi chạy `-s 1473` và đọc lỗi "Message too long". Bạn sẽ tận mắt thấy path MTU. Lặp
   lại với một host đi qua VPN nếu có.
3. Chạy `traceroute -n 8.8.8.8` (hoặc `mtr -n -c 10 8.8.8.8`) và giải thích từng dòng
   kết quả theo TTL và ICMP time-exceeded. Trong lúc chạy, bắt packet bằng
   `sudo tcpdump -n -i any icmp` và tìm các probe cùng reply tương ứng.
4. Chạy `sysctl net.ipv4.ip_forward` và giải thích điều gì thay đổi nếu đặt giá trị
   thành 1. Tìm xem thành phần nào của Docker/Kubernetes bật nó trên một container
   host, và vì sao.
5. Gửi dữ liệu tới một UDP port không có ai listen
   (`echo hi | nc -u -w1 127.0.0.1 9999`) trong khi `sudo tcpdump -n -i lo icmp` đang
   chạy. Tìm ICMP port-unreachable reply.
6. Chạy `ip -6 addr` và `curl -6 -v http://[::1]:8080/` vào
   [`labs/00-tcp-server`](../../labs/00-tcp-server) sau khi cho nó listen trên `[::]`.
   Kiểm tra xem một IPv4 client (`curl -4 http://127.0.0.1:8080/`) có connect được
   không, và dùng `IPV6_V6ONLY` để giải thích vì sao được hoặc không.
