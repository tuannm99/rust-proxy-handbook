# Link Layer: Ethernet, MAC, Switch, ARP

Một packet đi từ máy này sang máy ngay bên cạnh bằng cách nào. Mọi thứ phía
trên layer này ([`08-ip-and-icmp.md`](08-ip-and-icmp.md), [`12-tcp.md`](12-tcp.md)) đều giả định bài toán "next hop"
đã được giải; file này là nơi nó được giải.

## What to learn

### Layer chỉ là encapsulation, không hơn
Một "layer" là một header được dán vào trước dữ liệu của layer phía trên.
Proxy của bạn gọi `write(fd, b"GET / HTTP/1.1...")`. Kernel bọc các byte đó
trong một TCP header (port, sequence number), bọc tiếp trong một IP header
(source và destination IP), bọc tiếp trong một Ethernet header (source và
destination MAC), rồi đưa kết quả cho network card.

```text
| Ethernet hdr 14B | IP hdr 20B | TCP hdr 20B | HTTP bytes ... | FCS 4B |
|<--------------------------- một frame trên dây ------------------->|
```

Mỗi layer chỉ đọc header *của chính nó* và coi mọi thứ phía sau là payload
mờ đục. Switch chỉ đọc Ethernet header. Router đọc tới IP header. L4 load
balancer đọc tới TCP header; L7 proxy của bạn ([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md))
đọc xuyên vào tận HTTP. "L4 vs L7" nghĩa đen là "thiết bị nhìn sâu vào chồng
header này tới đâu". Bảng đầy đủ các layer (số OSI vs mô hình TCP/IP) nằm ở [`01-fundamentals.md`](01-fundamentals.md);
file này là layer 2.

### MAC address: danh tính trong một mạng cục bộ
**MAC address** là định danh 48-bit gắn cứng (hoặc được gán) cho một network
interface, viết dạng `aa:bb:cc:dd:ee:ff`. Nó chỉ có nghĩa trên **local
segment** — tập các máy với tới nhau được mà không phải đi qua router. IP
address mang tính toàn cục và routable; MAC address thì phẳng và cục bộ. Khi
packet đi qua một router, Ethernet header bị *vứt đi và dựng lại* cho segment
kế tiếp: IP header giữ nguyên (trừ TTL), còn MAC đổi ở mỗi hop.
`ff:ff:ff:ff:ff:ff` là địa chỉ **broadcast**: mọi interface trên segment đều
nhận frame đó.

### Switch: học, rồi forward
**Switch** nối các máy trong một segment. Nó giữ một bảng ánh xạ MAC address
-> physical port, xây bằng cách quan sát MAC *nguồn* của frame đến từ mỗi
port. Với frame có MAC đích đã học, nó forward ra đúng một port; với đích
chưa biết hoặc broadcast, nó **flood** ra mọi port trừ port frame đi vào.
**Hub** (đã lỗi thời) flood mọi thứ. Switch vô hình đối với IP: nó không đổi
địa chỉ và không có IP riêng để route. Tập các máy nhận broadcast của nhau là
một **broadcast domain**; giữ nó nhỏ là lý do mạng được chia nhỏ.

### ARP: cầu nối giữa IP và MAC
Máy bạn muốn gửi tới `10.0.0.7`, nằm trong cùng subnet
([`02-addressing.md`](02-addressing.md)). Ethernet header cần MAC, không phải IP. Nên nó
broadcast **ARP**: "ai có 10.0.0.7? báo cho 10.0.0.5." Chủ nhân reply bằng MAC
của mình; câu trả lời được cache trong **neighbor table** (`ip neigh`) vài
phút. Nếu đích *không* nằm trong subnet của bạn, bạn ARP lấy MAC của
**default gateway** và gửi frame tới gateway — trong khi IP đích vẫn là host ở
xa. Chi tiết đó (frame gửi tới gateway, packet gửi tới host cuối) chính là
routing. IPv6 thay ARP bằng Neighbor Discovery (ICMPv6), cùng ý tưởng.

```text
$ ip neigh
10.0.0.1 dev eth0 lladdr 52:54:00:12:35:02 REACHABLE
10.0.0.7 dev eth0 lladdr 08:00:27:aa:bb:cc STALE
```

### MTU: frame lớn nhất mà một link chở được
Ethernet chở tối đa **1500 byte IP packet** mỗi frame (**MTU**). Trừ 20 (IP)
và 20 (TCP) ra **MSS** — 1460 byte TCP payload mỗi segment — nên một response
100 KB là ~70 segment, không phải một. Tunnel (VPN, VXLAN, GRE) thêm header
riêng nên *hạ* MTU khả dụng của đường đi; loopback có MTU rất lớn (65536), là
một lý do benchmark local trông đẹp hơn mạng thật. Chuyện gì xảy ra khi
packet lớn hơn MTU của link nằm ở [`08-ip-and-icmp.md`](08-ip-and-icmp.md).

### VLAN, bridge, và virtual interface
**VLAN** thêm một tag 4 byte để một switch vật lý hoạt động như nhiều
broadcast domain cô lập. Trên một Linux host, các ý tưởng này tồn tại dưới
dạng software: một cặp `veth` là sợi cáp ảo giữa hai network namespace, một
**bridge** (`docker0`, `cni0`) là switch phần mềm nối chúng lại, và `eth0`
của mỗi container là đầu của một sợi cáp như vậy
([`02-linux/13-containers.md`](../02-linux/13-containers.md)). Khi một Kubernetes pod nói chuyện với
pod hàng xóm trên cùng node, "mạng" là một Linux bridge làm MAC learning y như
trên — không có dây nào cả.

### Gotcha: dây không đáng tin, và không gì ở layer này sửa điều đó
Ethernet có checksum (FCS) khiến NIC *vứt* frame hỏng, nhưng không có
acknowledgement và không có retransmission. Switch có output queue đầy sẽ
drop frame âm thầm. Mọi đảm bảo bạn dựa vào sau này — đúng thứ tự, đầy đủ,
không trùng — được TCP dựng lại bên trên
([`13-tcp-reliability.md`](13-tcp-reliability.md)). Đó cũng là lý do counter của `ip -s link`
(`dropped`, `overrun`, `errors`) là nơi đầu tiên cần nhìn khi một proxy host
đang tải nặng mất packet trước khi code của bạn kịp thấy.

## Practice

1. Chạy `ip -br link` và `ip -br addr`, xác định các interface của máy, MAC
   và IP của chúng. Tìm `lo` và ghi lại MTU của nó bằng `ip link show lo`.
2. Chạy `ip neigh`, rồi `ping -c1 <ip-gateway-của-bạn>` (lấy từ
   `ip route | grep default`), rồi `ip neigh` lần nữa — xem entry của
   gateway xuất hiện hoặc được refresh.
3. Bắt ARP trực tiếp: terminal một chạy `sudo tcpdump -n -e -i <iface> arp`
   (`-e` in MAC address), terminal hai chạy `sudo ip neigh flush all` rồi
   `ping` một host trong LAN. Xác định request (broadcast) và reply (unicast)
   và giải thích ai gửi cái nào.
4. Khi `sudo tcpdump -n -e -i <iface> icmp` đang chạy, `ping` một host *ngoài*
   subnet và so sánh MAC đích trong frame với MAC trong `ip neigh` của default
   gateway. Giải thích vì sao đó là MAC của gateway chứ không phải của host ở
   xa.
5. Chạy `ip -s link show <iface>` trước và sau một lần truyền lớn và đọc các
   counter packet `RX`/`TX` và `errors`/`dropped`.
6. Tạo hai network namespace nối bằng một cặp `veth` (`ip netns add`,
   `ip link add ... type veth peer name ...`, `ip link set ... netns ...`),
   gán địa chỉ, ping xuyên qua, và xem neighbor table trong mỗi namespace —
   cùng cơ chế mà container networking dùng. Sau đó chạy
   [`labs/00-tcp-server`](../../labs/00-tcp-server) trong một namespace và connect tới nó từ namespace
   còn lại bằng `nc`.
