# XDP

eXpress Data Path: chạy một chương trình eBPF ([`16-kernel/09-ebpf.md`](09-ebpf.md))
ngay trong NIC driver, trước khi kernel dựng bất kỳ state per-packet nào.
Nơi rẻ nhất để drop một packet trên Linux.

## What to learn

### XDP nằm ở đâu, và vì sao điều đó khiến nó nhanh
Hành trình bình thường của một packet cấp phát một `sk_buff` (vài trăm
byte metadata), đi qua các netfilter hook, băng qua TCP/IP stack, và cuối
cùng chạm tới một socket. XDP chạy *trước* khi `sk_buff` được cấp phát,
trên frame thô đã DMA trong đường receive của driver.

Drop ở đó chỉ tốn một bounds check và một return code. Drop ở iptables
tốn việc cấp phát `sk_buff` cộng với việc đi qua netfilter; drop trong
proxy của bạn tốn tất cả những thứ đó cộng thêm một wakeup, một syscall,
và một TCP handshake. Khoảng cách đo được là gần một bậc độ lớn (order of
magnitude) mỗi bước — XDP duy trì được hàng chục triệu packet mỗi giây
trên mỗi core, trong khi iptables chạm trần ở mức thấp triệu.

Đó là toàn bộ giá trị đề xuất cho [`07-security/09-ddos.md`](../07-security/09-ddos.md): dưới một đợt
flood dạng volumetric, chi phí *từ chối* traffic là thứ quyết định bạn có
sống sót hay không.

### Năm return code
Phán quyết của một chương trình XDP là giá trị nó trả về:

- `XDP_DROP` — loại bỏ ngay lập tức. Cái quan trọng nhất cho DDoS.
- `XDP_PASS` — tiếp tục vào network stack bình thường.
- `XDP_TX` — bật packet trở lại ra cùng interface (sau khi sửa đổi). Đây
  là cách một load balancer bằng XDP trả lời hoặc redirect.
- `XDP_REDIRECT` — gửi tới một interface khác hoặc vào một socket AF_XDP.
- `XDP_ABORTED` — drop và phát ra một tracepoint; nghĩa là có bug, không
  phải chính sách.

Gotcha: `XDP_ABORTED` là thứ một đường lỗi không được xử lý trả về, và nó
drop packet âm thầm theo góc nhìn của ứng dụng bạn. Traffic biến mất mà
không có log là trải nghiệm debug XDP kinh điển — theo dõi tracepoint
`xdp:xdp_exception` trước khi nghĩ rằng network có lỗi.

### Ba attach mode, với hiệu năng rất khác nhau
- **Native** — driver implement hook XDP trực tiếp. Cái thật sự. Cần
  driver hỗ trợ (ixgbe, mlx5, i40e, virtio-net, veth; đáng chú ý là
  *không phải* mọi NIC).
- **Offloaded** — chạy ngay trên hardware của NIC. Nhanh nhất, gần như
  không tốn CPU host, chỉ được hỗ trợ bởi rất ít card (Netronome).
- **Generic (SKB mode)** — một phương án dự phòng chạy sau khi `sk_buff`
  đã được cấp phát, nên nó từ bỏ toàn bộ lợi thế hiệu năng. Hoạt động ở
  mọi nơi.

Gotcha: chế độ generic là thứ bạn âm thầm nhận được khi driver không hỗ
trợ XDP native, và nó "hoạt động" — chương trình bạn load được, test bạn
pass, và lợi ích hiệu năng thì biến mất. Luôn kiểm tra bạn thực sự đã
attach ở chế độ nào, đặc biệt trong một VM hoặc container nơi interface
thường là veth hoặc một NIC ảo hóa.

### Viết parser: mọi thứ đều là bounds check
Bạn nhận được một con trỏ tới đầu frame và một con trỏ tới cuối nó, và
verifier đòi hỏi bằng chứng rằng mọi lần đọc nằm giữa chúng. Parse tới IP
header nghĩa là kiểm tra đủ chỗ cho Ethernet header, rồi IP header, mỗi
cái trước khi chạm vào nó:

```
if (data + sizeof(ethhdr) > data_end) return XDP_PASS;
// chỉ bây giờ mới được đọc eth->h_proto
if (data + sizeof(ethhdr) + sizeof(iphdr) > data_end) return XDP_PASS;
// chỉ bây giờ mới được đọc ip->saddr
```

Trả về `XDP_PASS` cho một packet quá ngắn thay vì `XDP_DROP` là mặc định
đúng — để stack của kernel xử lý input dị dạng, vì việc của bạn là chính
sách, không phải validation.

Gotcha: VLAN tag, IPv6 extension header, và IP option đều làm dịch offset.
Một parser giả định một prefix cố định 14-byte Ethernet + 20-byte IPv4 sẽ
đọc sai bất kỳ packet nào có tag hoặc option — và một kẻ tấn công nhận ra
điều này có thể thêm một VLAN tag để bypass bộ lọc của bạn hoàn toàn. Hãy
parse cả chuỗi, hoặc tường minh `XDP_PASS` bất cứ thứ gì không khớp đúng
hình dạng bạn xử lý.

### XDP có thể và không thể làm gì cho một proxy
Nó **có thể**: drop theo IP nguồn dựa trên một `LPM_TRIE` các CIDR,
rate-limit theo từng nguồn bằng counter per-CPU, drop packet dị dạng hoặc
protocol không mong đợi, và áp SYN rate — tất cả trước khi kernel tốn bất
cứ thứ gì.

Nó **không thể**: thấy TCP stream (nó thấy từng packet riêng lẻ, không
phải dữ liệu đã reassemble), inspect HTTP (một request có thể trải dài
nhiều packet, và TLS nghĩa là nó bị mã hóa dù sao đi nữa), hoặc ra quyết
định cần state ứng dụng. Bất cứ thứ gì cần request là việc của proxy, theo
định nghĩa.

Kiến trúc đúng là phân lớp: XDP drop những gì chứng minh được là thù địch
ở tốc độ đường dây, và mọi thứ khác đi qua rate-limiting ở accept của
chính proxy cùng các lớp phòng thủ ở tầng HTTP
([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md), [`07-security/06-waf.md`](../07-security/06-waf.md)). Blocklist đến
*từ* proxy — nơi thấy được các request — và được đẩy *vào* map của XDP từ
user space, đó là vòng feedback khiến cả hai lớp trở nên hữu dụng.

Gotcha: một entry blocklist cũ hoặc quá rộng giờ đang drop traffic ở một
lớp không có logging và không có visibility ở tầng ứng dụng. Luôn cho các
block cài trong XDP một TTL mà user space làm mới, để một bug tự hết hạn
thay vì tồn tại cho tới khi ai đó nhận ra.

## Practice
1. Trong [`labs/17-ebpf`](../../labs/17-ebpf), attach một chương trình XDP đếm và pass mọi
   packet. Xác nhận bằng `bpftool net show` bạn nhận chế độ attach nào —
   nếu là generic, tìm hiểu vì sao.
2. Thêm việc parse Ethernet + IPv4 với bounds check đúng; drop packet từ
   một IP nguồn hardcode và xác minh bằng `ping` từ host đó.
3. Gửi một packet có VLAN tag vào parser của bạn và xác nhận nó *không* bị
   parse sai — sau đó xử lý hoặc tường minh pass nó.
4. Thay IP hardcode bằng một map `LPM_TRIE` được điền từ user space; thêm
   và xóa CIDR lúc runtime trong khi chương trình vẫn đang attach.
5. Benchmark tỷ lệ drop cho cùng một blocklist được implement ba cách:
   XDP, iptables, và trong accept loop của [`proxy`](../../proxy). Dùng một packet
   generator và so sánh cả throughput lẫn CPU host.
6. Đóng vòng feedback: cho [`proxy`](../../proxy) phát hiện một nguồn lạm dụng qua
   [`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md) và đẩy nó vào map XDP với một TTL; xác
   nhận traffic ngừng chạm tới user space, và entry đó tự hết hạn.
