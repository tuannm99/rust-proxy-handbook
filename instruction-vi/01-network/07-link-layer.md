# Link Layer: Ethernet, MAC, Switch, ARP

File này trả lời câu hỏi: một packet đi từ máy này sang máy ngay bên cạnh bằng cách
nào? Mọi thứ phía trên layer này ([`08-ip-and-icmp.md`](08-ip-and-icmp.md),
[`12-tcp.md`](12-tcp.md)) đều giả định bài toán "next hop" đã được giải xong. File này
là nơi nó được giải.

## What to learn

### Layer chỉ là encapsulation, không hơn
Một "layer" chỉ là một header được dán vào trước dữ liệu của layer phía trên. Proxy
của bạn gọi `write(fd, b"GET / HTTP/1.1...")`. Kernel bọc các byte đó theo thứ tự:

1. TCP header (port, sequence number).
2. IP header (source IP và destination IP).
3. Ethernet header (source MAC và destination MAC).

Sau đó kernel đưa kết quả cho network card.

```text
| Ethernet hdr 14B | IP hdr 20B | TCP hdr 20B | HTTP bytes ... | FCS 4B |
|<--------------------------- một frame trên dây ------------------->|
```

Mỗi layer chỉ đọc header *của chính nó*, và coi mọi thứ phía sau là payload mà nó
không cần hiểu. Switch chỉ đọc Ethernet header. Router đọc đến IP header. L4 load
balancer đọc đến TCP header. L7 proxy của bạn ([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md))
đọc xuyên vào tận HTTP. "L4 so với L7" nghĩa đen là "thiết bị nhìn sâu đến đâu vào
chồng header này".

Bảng đầy đủ các layer (số OSI và mô hình TCP/IP) nằm ở
[`01-fundamentals.md`](01-fundamentals.md). File này nói về layer 2.

### MAC address: danh tính trong một mạng cục bộ
**MAC address** là định danh 48-bit của một network interface, được ghi cứng hoặc
được gán, viết dạng `aa:bb:cc:dd:ee:ff`. Nó chỉ có nghĩa trong **local segment**, tức
tập các máy liên lạc được với nhau mà không cần đi qua router.

IP address mang tính toàn cục và route được. MAC address thì phẳng và chỉ có ý nghĩa
cục bộ. Khi packet đi qua một router, Ethernet header cũ bị *bỏ đi và tạo lại* cho
segment kế tiếp. IP header giữ nguyên (trừ TTL), còn MAC đổi ở mỗi hop.

`ff:ff:ff:ff:ff:ff` là địa chỉ **broadcast**: mọi interface trên segment đều nhận frame
đó.

### Switch: học rồi mới forward
**Switch** nối các máy trong cùng một segment. Nó giữ một bảng ánh xạ MAC address →
physical port. Bảng này được xây bằng cách ghi nhận MAC *nguồn* của các frame đến từ
mỗi port.

- Với frame có MAC đích đã học: switch forward ra đúng một port.
- Với frame có đích chưa biết, hoặc frame broadcast: switch **flood** ra mọi port, trừ
  port frame đi vào.

**Hub** (đã lỗi thời) thì flood mọi thứ. Switch "vô hình" đối với IP: nó không đổi địa
chỉ và không có IP riêng để route. Tập các máy nhận được broadcast của nhau gọi là một
**broadcast domain**. Người ta chia nhỏ mạng để giữ broadcast domain nhỏ.

### ARP: cầu nối giữa IP và MAC
Giả sử máy bạn muốn gửi tới `10.0.0.7`, nằm cùng subnet
([`02-addressing.md`](02-addressing.md)). Ethernet header cần MAC, không phải IP. Nên
máy bạn broadcast một gói **ARP**: "Ai có 10.0.0.7? Báo cho 10.0.0.5." Chủ nhân của địa
chỉ đó trả lời bằng MAC của mình. Câu trả lời được cache vài phút trong **neighbor
table** (`ip neigh`).

Nếu đích *không* nằm trong subnet của bạn, bạn ARP để lấy MAC của **default gateway**,
rồi gửi frame tới gateway. IP đích trong packet vẫn là host ở xa. Chi tiết này (frame
gửi tới gateway, nhưng packet gửi tới host cuối) chính là bản chất của routing. IPv6
thay ARP bằng Neighbor Discovery (ICMPv6), cùng ý tưởng.

```text
$ ip neigh
10.0.0.1 dev eth0 lladdr 52:54:00:12:35:02 REACHABLE
10.0.0.7 dev eth0 lladdr 08:00:27:aa:bb:cc STALE
```

### MTU: frame lớn nhất một link chở được
Ethernet chở tối đa **1500 byte IP packet** trong mỗi frame. Con số này là **MTU**.
Trừ đi 20 byte IP header và 20 byte TCP header, ta có **MSS**: 1460 byte TCP payload
mỗi segment. Vì vậy một response 100 KB cần khoảng 70 segment, không phải một.

Tunnel (VPN, VXLAN, GRE) thêm header riêng, nên làm *giảm* MTU khả dụng của đường đi.
Loopback có MTU rất lớn (65536), và đó là một lý do benchmark local trông đẹp hơn mạng
thật. Chuyện gì xảy ra khi packet lớn hơn MTU của link thì xem ở
[`08-ip-and-icmp.md`](08-ip-and-icmp.md).

### VLAN, bridge và virtual interface
**VLAN** thêm một tag 4 byte để một switch vật lý hoạt động như nhiều broadcast domain
tách biệt nhau.

Trên một Linux host, các ý tưởng này tồn tại dưới dạng phần mềm:
- Một cặp `veth` là sợi cáp ảo nối hai network namespace.
- Một **bridge** (`docker0`, `cni0`) là switch phần mềm nối chúng lại.
- `eth0` trong mỗi container là một đầu của sợi cáp ảo như vậy
  ([`02-linux/13-containers.md`](../02-linux/13-containers.md)).

Khi một Kubernetes pod nói chuyện với pod hàng xóm trên cùng node, "mạng" ở đây là một
Linux bridge học MAC y hệt switch thật. Không có sợi dây nào cả.

### Gotcha: dây không đáng tin, và layer này không sửa điều đó
Ethernet có checksum (FCS) để NIC *bỏ* frame bị hỏng. Nhưng nó không có
acknowledgement và không có retransmission. Nếu output queue của switch đầy, switch
drop frame mà không báo gì.

Mọi đảm bảo bạn dựa vào về sau (đúng thứ tự, đầy đủ, không trùng) đều do TCP dựng lại ở
layer trên ([`13-tcp-reliability.md`](13-tcp-reliability.md)). Vì vậy, khi một proxy
host đang chịu tải nặng mà mất packet, các counter của `ip -s link` (`dropped`,
`overrun`, `errors`) là nơi đầu tiên cần nhìn, trước cả khi code của bạn kịp nhận ra.

## Practice

1. Chạy `ip -br link` và `ip -br addr`. Xác định các interface của máy, MAC và IP của
   từng cái. Tìm `lo` và ghi lại MTU của nó bằng `ip link show lo`.
2. Chạy `ip neigh`, rồi `ping -c1 <ip-gateway-của-bạn>` (lấy IP từ
   `ip route | grep default`), rồi chạy lại `ip neigh`. Xem entry của gateway xuất hiện
   hoặc được làm mới.
3. Bắt ARP trực tiếp. Ở terminal một, chạy `sudo tcpdump -n -e -i <iface> arp` (`-e` để
   in MAC address). Ở terminal hai, chạy `sudo ip neigh flush all` rồi `ping` một host
   trong LAN. Xác định request (broadcast) và reply (unicast), và giải thích ai gửi cái
   nào.
4. Chạy `sudo tcpdump -n -e -i <iface> icmp` và `ping` một host *ngoài* subnet. So sánh
   MAC đích trong frame với MAC của default gateway trong `ip neigh`. Giải thích vì sao
   đó là MAC của gateway chứ không phải MAC của host ở xa.
5. Chạy `ip -s link show <iface>` trước và sau một lần truyền dữ liệu lớn. Đọc các
   counter packet `RX`/`TX` và `errors`/`dropped`.
6. Tạo hai network namespace nối nhau bằng một cặp `veth` (`ip netns add`,
   `ip link add ... type veth peer name ...`, `ip link set ... netns ...`). Gán địa chỉ,
   ping xuyên qua, và xem neighbor table trong mỗi namespace. Đây là cơ chế mà container
   networking dùng. Sau đó chạy [`labs/00-tcp-server`](../../labs/00-tcp-server) trong
   một namespace và connect tới nó từ namespace còn lại bằng `nc`.

---

<!-- nav -->
← Trước: [`01-network/06-crypto-basics.md`](06-crypto-basics.md) · [Mục lục](00-README.md) · Tiếp theo: [`01-network/08-ip-and-icmp.md`](08-ip-and-icmp.md) →
