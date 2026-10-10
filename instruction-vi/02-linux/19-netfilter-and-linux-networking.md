# Linux Networking: Interface, netfilter, conntrack, NAT, và Transparent Proxying

Kernel Linux route, lọc và viết lại packet *xung quanh* process của bạn như thế nào — bộ máy đằng sau
`docker -p`, Kubernetes Service, `iptables -j REJECT` trong test của bạn, và "proxy của tôi thấy sai
source IP." Nối các file network ([`01-network/07-link-layer.md`](../01-network/07-link-layer.md), [`01-network/08-ip-and-icmp.md`](../01-network/08-ip-and-icmp.md)) với các file OS
([`13-containers.md`](13-containers.md)).

## What to learn

### Đường đi của packet qua kernel
Một packet đi vào một Linux host đi qua một chuỗi cố định, với các checkpoint nơi rule có thể tác động:

```text
NIC -> [PREROUTING] -> routing decision -+-> [INPUT] -> process local (socket của proxy)
                                         |
                                         +-> [FORWARD] -> [POSTROUTING] -> NIC (khi làm router)

proxy của bạn write -> routing -> [OUTPUT] -> [POSTROUTING] -> NIC
```

Năm checkpoint là các **netfilter hook**. Một **routing decision** (interface nào, next hop nào —
[`01-network/08-ip-and-icmp.md`](../01-network/08-ip-and-icmp.md)) nằm sau PREROUTING: traffic cho một địa chỉ của chính host đi lên INPUT và một socket; traffic cho người
khác thì được forward (chỉ khi `net.ipv4.ip_forward=1`) hoặc bị drop. `PREROUTING` là nơi destination NAT xảy ra (trước routing,
nên đích mới được route); `POSTROUTING` là nơi source NAT xảy ra (sau routing, khi packet rời đi). Mọi thứ dưới đây là các rule gắn
vào những hook này.

### iptables và nftables: rule trong chain
Rule được tổ chức thành các **table** (`filter`: accept/drop; `nat`: viết lại địa chỉ; `mangle`: sửa header/mark; `raw`: bỏ qua
connection tracking) chứa các **chain** gắn với hook, mỗi chain là danh sách có thứ tự các rule `match -> target`; target kết thúc
đầu tiên khớp (`ACCEPT`, `DROP`, `REJECT`, `DNAT`, ...) thắng, nếu không thì **policy** mặc định của chain áp dụng. **nftables** là
bản thay thế hiện đại (một cú pháp, thay thế rule-set atomic, hiệu năng tốt hơn); các lệnh `iptables` trên các distro hiện tại
thường là lớp tương thích bên trên nó.

```text
iptables -L -n -v                     # filter table, kèm counter
iptables -t nat -L -n -v              # NAT table
iptables -A INPUT -p tcp --dport 8080 -j DROP
nft list ruleset
```
**`DROP` vs `REJECT`** quan trọng khi bạn test: `DROP` bỏ im lặng, nên client thấy **treo rồi timeout**; `REJECT` gửi về một lỗi
tức thì (TCP RST qua `--reject-with tcp-reset`, hoặc ICMP port-unreachable), nên client thấy `Connection refused`/reset ngay. Chúng
kích hoạt các code path khác nhau trong proxy của bạn (connect timeout vs refused — [`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md),
[`01-network/21-life-of-a-request.md`](../01-network/21-life-of-a-request.md)). Rule không bền qua reboot trừ khi được lưu, và **thứ tự quan trọng**: `-I` chèn lên đầu, `-A` nối vào cuối sau
một catch-all có thể có.

### Connection tracking: kernel nhớ các flow
**conntrack** cho netfilter *state*: nó theo dõi mỗi flow (4-tuple cộng state — `NEW`, `ESTABLISHED`, `RELATED`, `INVALID`) trong một
bảng, kể cả với UDP và ICMP qua timeout. Các rule stateful dựa vào nó (`-m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT`: "cho
phép reply của các connection tôi đã cho đi ra"), và NAT dựa vào nó để nhớ mapping và dịch ngược packet reply. `conntrack -L` liệt kê
các entry; `conntrack -C` đếm chúng; `cat /proc/sys/net/netfilter/nf_conntrack_max` là dung lượng bảng.

**Gotcha: conntrack-table-full.** Một NAT box bận, một Kubernetes node, hay bất kỳ host nào có conntrack được nạp và một proxy tạo
nhiều connection ngắn đều có thể làm đầy bảng; kernel khi đó **drop connection mới** và log `nf_conntrack: table full, dropping
packet` (`dmesg`). Triệu chứng là timeout connection chập chờn trong khi service và CPU đều khỏe. Cách xử lý: tăng `nf_conntrack_max`
(để ý bộ nhớ), hạ timeout (một entry TCP `ESTABLISHED` idle sống 5 ngày theo mặc định!), tái sử dụng connection
([`01-network/12-tcp.md`](../01-network/12-tcp.md)), hoặc miễn trừ traffic khỏi tracking (`-t raw ... -j NOTRACK`). So sánh `conntrack -C` với `nf_conntrack_max` trong monitoring
([`08-observability/02-metrics.md`](../08-observability/02-metrics.md)).

### NAT trong thực tế: SNAT, MASQUERADE, DNAT
Khái niệm nằm ở [`01-network/02-addressing.md`](../01-network/02-addressing.md); đây là cách chúng được làm ra:

- **SNAT/MASQUERADE** (`POSTROUTING`): viết lại *source* để reply quay về gateway.
  `iptables -t nat -A POSTROUTING -s 172.17.0.0/16 -j MASQUERADE` là cách container ra internet (source thành IP của host).
- **DNAT** (`PREROUTING` cho traffic bên ngoài, `OUTPUT` cho local): viết lại *destination*.
  `iptables -t nat -A PREROUTING -p tcp --dport 8080 -j DNAT --to-destination 172.17.0.2:80` chính là `docker run -p 8080:80`.
  Packet tới container của bạn với **source IP gốc còn nguyên** — nhưng chỉ khi reply đi qua cùng NAT box; lỗi "asymmetric routing làm
  hỏng NAT" là reply đi vòng qua nó ([`01-network/08-ip-and-icmp.md`](../01-network/08-ip-and-icmp.md)).
- **Kubernetes** `kube-proxy` cài Service dưới dạng rule DNAT (iptables) hoặc IPVS virtual server: virtual IP của một Service không
  phải địa chỉ của interface nào — nó là một rule viết lại chọn một pod IP cho mỗi *connection*. Đây là một L4 load balancer bên trong
  kernel. Một client có connection sống lâu (HTTP/2, gRPC) vì thế bị ghim vào một pod suốt đời connection, lý do gRPC cần cân bằng L7
  ([`05-http-stack/11-grpc.md`](../05-http-stack/11-grpc.md), [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)).

NAT giấu client: proxy của bạn đứng sau một load balancer SNAT thấy IP của balancer, đó là lý do PROXY protocol và `X-Forwarded-For` tồn tại
([`01-network/20-proxy-protocol.md`](../01-network/20-proxy-protocol.md), [`01-network/15-http.md`](../01-network/15-http.md)).

### Transparent proxying: chặn traffic không gửi cho bạn
Một service mesh sidecar hay một forward proxy thường phải chặn các connection được gửi tới *người khác*, mà client không biết. Rule
redirect chúng về một port local (`-j REDIRECT --to-ports 15001`, hoặc `TPROXY` cho UDP và để giữ địa chỉ), và proxy khôi phục nơi client
**thật sự** định đến bằng `getsockopt(SO_ORIGINAL_DST)` trên socket đã accept (sau `REDIRECT`) hoặc từ `getsockname` (dưới `TPROXY` với
`IP_TRANSPARENT`). Việc tra original-destination đó là cách các sidecar Istio/Envoy và công cụ kiểu `redsocks` hoạt động
([`01-network/05-proxy-taxonomy.md`](../01-network/05-proxy-taxonomy.md)). Nó cũng là lý do traffic đi ra của chính proxy phải được miễn khỏi rule redirect (khớp theo UID hoặc `mark` của socket),
nếu không nó quay vòng về chính mình.

### Network namespace, veth, bridge
Mỗi **network namespace** ([`13-containers.md`](13-containers.md)) có interface, route, iptables rule và bảng conntrack riêng — `eth0` của một container là
một đầu của một **cặp veth** mà đầu kia nằm trên một **bridge** của host (switch phần mềm, [`01-network/07-link-layer.md`](../01-network/07-link-layer.md)). `ip netns add`,
`ip link add ... type veth`, `ip netns exec <ns> <cmd>` cho phép bạn dựng các mạng mini cô lập bằng tay — cách tốt nhất để test một proxy
trước các lỗi (drop, delay, loss) mà không đụng mạng thật của bạn. Nhớ rằng một listening socket thuộc về *một namespace*: một proxy
trong host namespace listen trên `127.0.0.1` không với tới được từ `127.0.0.1` của container.

### Traffic control: queue và netem
`tc` cấu hình các **queueing discipline (qdisc)** trên output của một interface. `tc qdisc add dev eth0 root netem delay 100ms loss 1%`
tiêm latency và loss; `tbf`/`htb` định hình bandwidth; `fq_codel` chống bufferbloat ([`01-network/13-tcp-reliability.md`](../01-network/13-tcp-reliability.md)). Đây là công cụ fault-injection
của [`12-testing/03-chaos.md`](../12-testing/03-chaos.md). `netem` áp dụng trên **egress**; để tác động traffic mà một host *nhận*, bạn áp dụng nó ở peer, hoặc trên một device `ifb`.

### Các sysctl mà một network service gặp
`net.core.somaxconn` (trần của mọi backlog `listen`; `listen(1024)` của bạn bị kẹp âm thầm, [`01-network/11-socket.md`](../01-network/11-socket.md)),
`net.ipv4.tcp_max_syn_backlog`, `net.ipv4.ip_local_port_range`, `net.ipv4.ip_forward`, `net.ipv4.tcp_tw_reuse`, `net.netfilter.nf_conntrack_max`,
`net.ipv4.conf.*.rp_filter` (drop packet mà source của nó không tới ngược lại được qua interface đến — nguồn gốc của các vụ drop bí ẩn
với asymmetric routing). `sysctl -a | grep <name>` đọc chúng; `sysctl -w` đổi chúng cho tới khi reboot; `/etc/sysctl.d/` lưu bền. Trong container nhiều cái
là theo namespace; một số cần đặc quyền.

## Practice

Hãy làm theo thứ tự.

1. Dựng hai namespace và một bridge bằng tay (`ip netns add`, `ip link add br0 type bridge`, hai cặp `veth`, địa chỉ,
   `ip link set ... up`) và ping giữa chúng. **Done when** ping thành công và bạn chỉ ra được bảng MAC của bridge
   (`bridge fdb show`) cùng route của mỗi namespace và giải thích từng entry.
2. Trong một namespace chạy [`labs/00-tcp-server`](../../labs/00-tcp-server); `curl` nó từ namespace kia. Rồi `ip netns exec <server-ns> iptables -A INPUT -p tcp
   --dport <p> -j DROP`, và sau đó thay bằng `-j REJECT --reject-with tcp-reset`. **Done when** bạn đã bắt cả hai bằng `tcpdump`
   ([`01-network/10-packet-capture-and-tools.md`](../01-network/10-packet-capture-and-tools.md)) và cho thấy DROP = SYN không có reply (client treo, rồi timeout) vs REJECT = RST tức thì
   (`Connection refused`).
3. Biến một namespace thành NAT router: bật `ip_forward`, thêm rule MASQUERADE, và để một client trong namespace private với tới một
   server trên host. **Done when** server log ra source IP *đã bị viết lại* và `conntrack -L` hiện entry tương ứng.
4. Thêm một rule DNAT công bố proxy của bạn ([`labs/05-reverse-proxy`](../../labs/05-reverse-proxy)) ở một port bên ngoài khác vào một namespace. **Done when** proxy
   log ra IP client gốc, và sau khi bạn cố tình phá đối xứng reply (đường reply đi vòng qua NAT box) bạn cho thấy được cái treo
   kết quả trong `tcpdump`.
5. Thêm `tc qdisc ... netem delay 100ms loss 1%` lên một veth và chạy một vòng request qua proxy của bạn. **Done when** bạn cho thấy được
   latency p50/p99 và số retry trước và sau, và gắn chúng với cài đặt timeout/retry của proxy.
6. Cố tình làm đầy conntrack trong một namespace (một vòng connection với `nf_conntrack_max` rất nhỏ). **Done when** `dmesg` hiện
   `table full, dropping packet` và bạn đã ghi lại điều client quan sát được.
7. Dùng `iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports <port>` với một chương trình scratch đọc `SO_ORIGINAL_DST`
   qua `getsockopt` (`libc`/`nix`). **Done when** nó in ra đích thật của một `curl` bị redirect, và bạn đã miễn UID của chính proxy để
   nó không redirect về chính mình.
