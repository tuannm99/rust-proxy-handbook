# Linux Networking: Interface, netfilter, conntrack, NAT và Transparent Proxying

File này giải thích kernel Linux route, lọc và viết lại packet *xung quanh* process của bạn như thế nào. Đây là
bộ máy đứng sau `docker -p`, Kubernetes Service, lệnh `iptables -j REJECT` trong test của bạn, và lỗi "proxy của
tôi thấy sai source IP". File này nối các file về network
([`01-network/07-link-layer.md`](../01-network/07-link-layer.md),
[`01-network/08-ip-and-icmp.md`](../01-network/08-ip-and-icmp.md)) với các file về OS
([`13-containers.md`](13-containers.md)).

## What to learn

### Đường đi của packet qua kernel
Một packet đi vào Linux host sẽ đi qua một chuỗi cố định, có các checkpoint mà rule có thể tác động:

```text
NIC -> [PREROUTING] -> routing decision -+-> [INPUT] -> process local (socket của proxy)
                                         |
                                         +-> [FORWARD] -> [POSTROUTING] -> NIC (khi làm router)

proxy của bạn write -> routing -> [OUTPUT] -> [POSTROUTING] -> NIC
```

Năm checkpoint này là các **netfilter hook**. Một **routing decision** (chọn interface nào, next hop nào,
[`01-network/08-ip-and-icmp.md`](../01-network/08-ip-and-icmp.md)) nằm sau PREROUTING:

- Traffic gửi tới địa chỉ của chính host đi lên INPUT rồi tới một socket.
- Traffic gửi cho người khác thì được forward (chỉ khi `net.ipv4.ip_forward=1`) hoặc bị drop.

`PREROUTING` là nơi destination NAT xảy ra. Nó nằm trước routing, nên đích mới được đem đi route.
`POSTROUTING` là nơi source NAT xảy ra. Nó nằm sau routing, khi packet sắp rời đi. Mọi thứ bên dưới chỉ là các
rule gắn vào những hook này.

### iptables và nftables: rule nằm trong chain
Rule được tổ chức thành các **table**:
- `filter`: accept hoặc drop.
- `nat`: viết lại địa chỉ.
- `mangle`: sửa header hoặc đánh dấu (mark).
- `raw`: bỏ qua connection tracking.

Mỗi table chứa các **chain** gắn với hook. Mỗi chain là một danh sách có thứ tự các rule dạng `match -> target`.
Target kết thúc đầu tiên khớp (`ACCEPT`, `DROP`, `REJECT`, `DNAT`, ...) sẽ thắng. Nếu không có rule nào khớp thì
**policy** mặc định của chain được áp dụng.

**nftables** là bản thay thế hiện đại: một cú pháp thống nhất, thay cả rule-set trong một thao tác atomic, hiệu
năng tốt hơn. Lệnh `iptables` trên các distro hiện nay thường chỉ là một lớp tương thích bên trên nó.

```text
iptables -L -n -v                     # filter table, kèm counter
iptables -t nat -L -n -v              # NAT table
iptables -A INPUT -p tcp --dport 8080 -j DROP
nft list ruleset
```
**`DROP` và `REJECT`** khác nhau, và điều này quan trọng khi test:
- `DROP` bỏ packet một cách im lặng, nên client thấy **treo rồi timeout**.
- `REJECT` gửi về một lỗi ngay lập tức (TCP RST qua `--reject-with tcp-reset`, hoặc ICMP port-unreachable), nên
  client thấy `Connection refused`/reset ngay.

Hai cách này kích hoạt hai code path khác nhau trong proxy của bạn (connect timeout và refused,
[`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md),
[`01-network/21-life-of-a-request.md`](../01-network/21-life-of-a-request.md)).

Rule không tồn tại sau reboot trừ khi bạn lưu chúng. **Thứ tự rất quan trọng**: `-I` chèn lên đầu, còn `-A` nối
vào cuối, có thể nằm sau một rule catch-all đã có.

### Connection tracking: kernel nhớ các flow
**conntrack** cho netfilter khả năng nhớ *state*. Nó theo dõi mỗi flow (4-tuple cộng state: `NEW`,
`ESTABLISHED`, `RELATED`, `INVALID`) trong một bảng, kể cả với UDP và ICMP (dùng timeout).

- Các rule stateful dựa vào nó. Ví dụ `-m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT` nghĩa là "cho phép
  reply của các connection tôi đã cho đi ra".
- NAT dựa vào nó để nhớ mapping và dịch ngược các packet reply.

`conntrack -L` liệt kê các entry. `conntrack -C` đếm chúng. `cat /proc/sys/net/netfilter/nf_conntrack_max` cho biết
dung lượng bảng.

**Gotcha: bảng conntrack bị đầy.** Một NAT box bận, một Kubernetes node, hay bất kỳ host nào có load conntrack và
một proxy tạo nhiều connection ngắn đều có thể làm đầy bảng. Khi đó kernel **drop các connection mới** và log
`nf_conntrack: table full, dropping packet` (xem `dmesg`). Triệu chứng là connection bị timeout chập chờn trong khi
service và CPU đều khỏe mạnh.

Cách xử lý:
- Tăng `nf_conntrack_max` (chú ý bộ nhớ).
- Giảm timeout. Một entry TCP `ESTABLISHED` đang idle sống 5 ngày theo mặc định!
- Reuse connection ([`01-network/12-tcp.md`](../01-network/12-tcp.md)).
- Miễn traffic khỏi tracking (`-t raw ... -j NOTRACK`).

Hãy so sánh `conntrack -C` với `nf_conntrack_max` trong monitoring
([`08-observability/02-metrics.md`](../08-observability/02-metrics.md)).

### NAT trong thực tế: SNAT, MASQUERADE, DNAT
Khái niệm nằm ở [`01-network/02-addressing.md`](../01-network/02-addressing.md). Ở đây là cách chúng được làm ra:

- **SNAT/MASQUERADE** (`POSTROUTING`): viết lại *source* để reply quay về gateway.
  `iptables -t nat -A POSTROUTING -s 172.17.0.0/16 -j MASQUERADE` là cách container ra internet (source được đổi
  thành IP của host).
- **DNAT** (`PREROUTING` cho traffic từ bên ngoài, `OUTPUT` cho traffic local): viết lại *destination*.
  `iptables -t nat -A PREROUTING -p tcp --dport 8080 -j DNAT --to-destination 172.17.0.2:80` chính là
  `docker run -p 8080:80`. Packet tới container của bạn vẫn **giữ nguyên source IP gốc**, nhưng chỉ khi reply đi qua
  chính NAT box đó. Lỗi "asymmetric routing làm hỏng NAT" xảy ra khi reply đi đường vòng không qua nó
  ([`01-network/08-ip-and-icmp.md`](../01-network/08-ip-and-icmp.md)).
- **Kubernetes** `kube-proxy` cài Service dưới dạng rule DNAT (iptables) hoặc IPVS virtual server. Virtual IP của
  một Service không phải địa chỉ của interface nào. Nó là một rule viết lại, chọn một pod IP cho mỗi
  *connection*. Về bản chất đây là một L4 load balancer nằm trong kernel. Hệ quả: một client có connection sống
  lâu (HTTP/2, gRPC) bị gắn với một pod suốt đời connection. Đó là lý do gRPC cần load balancing ở L7
  ([`05-http-stack/11-grpc.md`](../05-http-stack/11-grpc.md),
  [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)).

NAT che giấu client. Nếu proxy của bạn đứng sau một load balancer có SNAT, nó chỉ thấy IP của balancer. Đó là lý do
PROXY protocol và `X-Forwarded-For` tồn tại
([`01-network/20-proxy-protocol.md`](../01-network/20-proxy-protocol.md),
[`01-network/15-http.md`](../01-network/15-http.md)).

### Transparent proxying: intercept traffic không được gửi cho bạn
Một sidecar của service mesh, hay một forward proxy, thường phải intercept các connection được gửi tới
*người khác*, mà client không hề biết. Các rule chuyển hướng chúng về một port local (`-j REDIRECT --to-ports
15001`, hoặc `TPROXY` cho UDP và để giữ nguyên địa chỉ). Proxy khôi phục đích đến **thật sự** của client bằng:
- `getsockopt(SO_ORIGINAL_DST)` trên socket đã accept (sau `REDIRECT`), hoặc
- `getsockname` (dưới `TPROXY` với `IP_TRANSPARENT`).

Việc tra original destination này là cách các sidecar Istio/Envoy và các công cụ như `redsocks` hoạt động
([`01-network/05-proxy-taxonomy.md`](../01-network/05-proxy-taxonomy.md)). Nó cũng là lý do traffic đi ra của chính
proxy phải được miễn khỏi rule redirect (khớp theo UID hoặc `mark` của socket). Nếu không, traffic đó sẽ bị chuyển
ngược về chính proxy.

### Network namespace, veth, bridge
Mỗi **network namespace** ([`13-containers.md`](13-containers.md)) có interface, route, iptables rule và bảng
conntrack riêng. `eth0` của một container là một đầu của một **cặp veth**, đầu kia nằm trên một **bridge** của host
(một switch phần mềm, [`01-network/07-link-layer.md`](../01-network/07-link-layer.md)).

`ip netns add`, `ip link add ... type veth` và `ip netns exec <ns> <cmd>` cho phép bạn tự dựng các mạng mini cô
lập. Đây là cách tốt nhất để test proxy trước các lỗi mạng (drop, delay, loss) mà không đụng vào mạng thật của bạn.

Hãy nhớ rằng một listening socket thuộc về *một namespace*. Một proxy trong host namespace đang listen trên
`127.0.0.1` sẽ không thể truy cập được từ `127.0.0.1` bên trong container.

### Traffic control: queue và netem
`tc` cấu hình các **queueing discipline (qdisc)** trên output của một interface.

- `tc qdisc add dev eth0 root netem delay 100ms loss 1%` thêm latency và loss.
- `tbf`/`htb` định hình bandwidth.
- `fq_codel` chống bufferbloat ([`01-network/13-tcp-reliability.md`](../01-network/13-tcp-reliability.md)).

Đây là công cụ fault-injection của [`12-testing/03-chaos.md`](../12-testing/03-chaos.md). `netem` chỉ áp dụng trên
**egress**. Muốn tác động lên traffic mà một host *nhận về*, hãy áp dụng nó ở phía peer, hoặc trên một device `ifb`.

### Các sysctl mà một network service gặp
- `net.core.somaxconn`: trần của mọi backlog `listen`. `listen(1024)` của bạn bị giới hạn lại một cách âm thầm
  ([`01-network/11-socket.md`](../01-network/11-socket.md)).
- `net.ipv4.tcp_max_syn_backlog`.
- `net.ipv4.ip_local_port_range`.
- `net.ipv4.ip_forward`.
- `net.ipv4.tcp_tw_reuse`.
- `net.netfilter.nf_conntrack_max`.
- `net.ipv4.conf.*.rp_filter`: drop packet mà source của nó không đi ngược lại được qua interface mà packet đến. Đây
  là nguồn gốc của các vụ drop bí ẩn khi có asymmetric routing.

`sysctl -a | grep <name>` để đọc. `sysctl -w` để đổi (có hiệu lực đến khi reboot). `/etc/sysctl.d/` để lưu bền. Trong
container, nhiều sysctl chỉ áp dụng trong namespace của nó, và một số cần đặc quyền.

## Practice

Hãy làm theo thứ tự.

1. Dựng hai namespace và một bridge bằng tay (`ip netns add`, `ip link add br0 type bridge`, hai cặp `veth`, gán
   địa chỉ, `ip link set ... up`) và ping giữa chúng. **Done when** ping thành công, và bạn chỉ ra được bảng MAC của
   bridge (`bridge fdb show`) cùng route của mỗi namespace, đồng thời giải thích từng entry.
2. Trong một namespace, chạy [`labs/00-tcp-server`](../../labs/00-tcp-server). Dùng `curl` gọi nó từ namespace kia.
   Sau đó chạy `ip netns exec <server-ns> iptables -A INPUT -p tcp --dport <p> -j DROP`, rồi thay bằng
   `-j REJECT --reject-with tcp-reset`. **Done when** bạn đã bắt cả hai trường hợp bằng `tcpdump`
   ([`01-network/10-packet-capture-and-tools.md`](../01-network/10-packet-capture-and-tools.md)) và cho thấy: DROP =
   SYN không có reply (client treo rồi timeout), còn REJECT = RST ngay lập tức (`Connection refused`).
3. Biến một namespace thành NAT router: bật `ip_forward`, thêm rule MASQUERADE, và cho một client trong namespace
   private kết nối tới một server trên host. **Done when** server log ra source IP *đã bị viết lại*, và `conntrack -L`
   hiện entry tương ứng.
4. Thêm một rule DNAT để công bố proxy của bạn ([`labs/05-reverse-proxy`](../../labs/05-reverse-proxy)) ở một port
   bên ngoài khác vào một namespace. **Done when** proxy log ra IP gốc của client. Sau đó cố tình phá tính đối xứng
   của reply (cho đường reply đi vòng không qua NAT box) và cho thấy chỗ bị treo trong `tcpdump`.
5. Thêm `tc qdisc ... netem delay 100ms loss 1%` lên một veth và chạy một vòng request qua proxy của bạn. **Done
   when** bạn cho thấy latency p50/p99 và số lần retry trước và sau khi thêm, và gắn chúng với cài đặt timeout/retry
   của proxy.
6. Cố tình làm đầy conntrack trong một namespace (một vòng lặp tạo connection, với `nf_conntrack_max` rất nhỏ).
   **Done when** `dmesg` hiện `table full, dropping packet`, và bạn đã ghi lại những gì client quan sát được.
7. Dùng `iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports <port>` cùng một chương trình scratch đọc
   `SO_ORIGINAL_DST` qua `getsockopt` (`libc`/`nix`). **Done when** nó in ra đích thật của một lệnh `curl` bị
   redirect, và bạn đã miễn UID của chính proxy khỏi rule để nó không redirect về chính mình.
