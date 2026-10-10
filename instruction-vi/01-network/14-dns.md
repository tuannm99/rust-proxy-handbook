# DNS

Từ tên ra địa chỉ: hệ thống phân cấp, một lần lookup thực sự làm gì trên máy
bạn, wire format, caching, và vì sao một proxy chạy lâu phải coi DNS là dữ liệu
sống.

## What to learn

### Một cái tên là một đường đi trong cây
`www.example.com.` được đọc từ phải sang trái: root (`.`), top-level domain
(`com`), domain (`example`), một host label (`www`). Cây được chia thành các
**zone**, mỗi zone do một tập server **authoritative** quản lý. Zone cha
**delegate** zone con bằng record `NS` ("`example.com` được phục vụ bởi
`ns1.example.net`"), kèm record `A` **glue** khi nameserver nằm ngay trong chính
zone mà nó phục vụ (nếu không bạn sẽ cần DNS để tìm DNS). 13 định danh root
server và các TLD server không giữ record của host nào, chỉ giữ delegation: đó
là toàn bộ lý do hệ thống scale được.

### Recursive vs authoritative resolution
**Recursive resolver** (của ISP, hoặc loại công cộng như 1.1.1.1) đi theo chuỗi
thay bạn — hỏi root server, nhận về các server của `com`; hỏi chúng, nhận về
server của `example.com`; hỏi chúng, nhận câu trả lời — và cache mọi bước.
Server **authoritative** là nguồn chân lý của một zone và chỉ trả lời cho thứ
nó sở hữu. Chương trình của bạn nói chuyện với recursive resolver; nó gần như
không bao giờ nói thẳng với authoritative server. Biết chuỗi này quan trọng khi
một thay đổi DNS "không propagate": bạn cần biết layer nào đang stale.

### Chuyện gì xảy ra khi chương trình resolve một cái tên
Code của bạn gọi `getaddrinfo` (thứ `ToSocketAddrs` và `curl` dùng). Trên Linux
nó tra theo thứ tự đặt trong `/etc/nsswitch.conf`: `/etc/hosts`, rồi DNS. Với
DNS nó đọc `/etc/resolv.conf`: các địa chỉ `nameserver` (thường là một stub
local như `127.0.0.53` từ `systemd-resolved`, hoặc DNS của cluster trong
container), các `search` domain, và `options ndots:N`.

**Search list** làm nhiều người vấp: một cái tên có ít hơn `ndots` dấu chấm sẽ
được thử với từng search suffix *trước*. Kubernetes set `ndots:5`, nên resolve
`api.example.com` (2 dấu chấm) trước hết thử
`api.example.com.default.svc.cluster.local`, `...svc.cluster.local`,
`...cluster.local` — ba query phí (mỗi cái một `NXDOMAIN`) trước query thật, với
mỗi lookup nguội. Dấu chấm cuối (`api.example.com.`) đánh dấu tên là tuyệt đối
và bỏ qua search list. Lưu ý thêm: `getaddrinfo` của glibc **không có cache
riêng** (mỗi lần gọi là một query trừ khi có stub/`nscd` cache), và musl
(Alpine) khác ở một số chi tiết như xử lý search domain — "chạy trên laptop mà
fail trong container" thường là chuyện này.

### Trên wire: một query, một response
Một DNS message gồm header (ID 16-bit, flags, các bộ đếm), một question (name,
type, class) và các section answer/authority/additional. Query đi qua **UDP
port 53** ([`09-udp.md`](09-udp.md)); ID 16-bit khớp response với query (và cùng với source
port là thứ duy nhất canh chừng reply giả mạo — vì thế mới có source port ngẫu
nhiên). Response có answer không vừa sẽ set cờ **TC (truncated)** và client
**retry qua TCP** — như cũng làm với response DNSSEC/EDNS lớn. **EDNS0** nâng
kích thước UDP an toàn lên trên mức 512 byte cũ. Response code cho biết chuyện
gì xảy ra:

- `NOERROR` kèm answer: thành công. `NOERROR` với **không có** answer
  (**NODATA**) nghĩa là tên tồn tại nhưng không có record loại đó — điển hình
  khi host chỉ có `A` mà bạn hỏi `AAAA`.
- `NXDOMAIN`: tên không tồn tại.
- `SERVFAIL`: resolver không lấy được câu trả lời (delegation hỏng, lỗi DNSSEC,
  upstream timeout). `REFUSED`: nó từ chối trả lời bạn.

### Các record type quan trọng với proxy
`A`/`AAAA` (địa chỉ upstream IPv4/IPv6), `CNAME` (alias; một tên có `CNAME` không
thể mang record khác, vì thế bạn không thể `CNAME` một zone apex), `SRV` (host +
port + priority + weight — thứ mà rất nhiều service discovery xây trên đó),
`NS`/`SOA` (delegation và metadata của zone), `TXT` (dùng cho ACME DNS-01
challenge khi tự động hóa việc cấp certificate, [`19-tls.md`](19-tls.md)), `PTR` (reverse
lookup: IP ra tên; log và mail dùng nó). Một proxy resolve hostname upstream
thường chỉ quan tâm `A`/`AAAA` và đôi khi `SRV`.

### TTL và caching — dương và âm
TTL cho mọi cache phía dưới (OS, resolver, proxy của bạn) biết một record được
dùng lại trong bao lâu. Một proxy tự resolve hostname upstream cần cache riêng
với việc expire tôn trọng TTL — dùng lại IP cũ sau khi địa chỉ backend đổi sẽ
âm thầm đẩy traffic vào hư không. **Câu trả lời âm cũng được cache**: một
`NXDOMAIN` được nhớ trong khoảng `SOA` minimum của zone, nên một record bạn *vừa
tạo* có thể vẫn "không tồn tại" hàng phút ở các resolver đã tra trước đó. Gotcha
production: một số recursive resolver hoặc client library clamp hoặc bỏ qua TTL
rất thấp (dưới 5s), làm hỏng các deployment failover dựa trên DNS — đừng giả
định TTL 1 giây cho bạn failover 1 giây. Và không gì đã connect sẽ re-resolve:
một pooled connection tới IP cũ vẫn tiếp tục dùng nó
([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)).

### Resolve trong Rust: blocking vs async
`getaddrinfo` của libc được `std::net::ToSocketAddrs` dùng là *blocking* và tự
parse cấu hình ở mức OS (`/etc/resolv.conf`, `/etc/hosts`) — gọi trực tiếp trong
một async task sẽ làm đứng executor thread. Các lựa chọn: chạy nó qua
`tokio::task::spawn_blocking` (`tokio::net::lookup_host` làm chính xác điều
này), hoặc dùng một resolver crate async thuần như `hickory-resolver` nói DNS
trực tiếp qua UDP/TCP và cho bạn caching có nhận biết TTL cùng quyền kiểm soát
nameserver nào được query.

```rust
// resolve blocking được làm ngoài async executor thread
let addrs = tokio::task::spawn_blocking(|| {
    "backend.internal:8080".to_socket_addrs()
})
.await??;
```

Một answer của resolver có thể chứa nhiều địa chỉ của cả hai họ. Connect cho tốt
nghĩa là thử chúng một cách hợp lý — **Happy Eyeballs**: bắt đầu với IPv6, và
nếu chưa connect được trong ~250 ms thì race IPv4 — để một đường `AAAA` hỏng
không làm mỗi request tốn một timeout dài. Một `TcpStream::connect(name)` trần
thử các địa chỉ tuần tự, mỗi cái với đủ connect timeout.

### DNS như cơ chế service discovery và phân phối tải
Nhiều hệ thống (Kubernetes headless Service, Consul DNS interface) phơi service
registry của chúng *dưới dạng* DNS — một record `A` trả về nhiều IP, hoặc thay
đổi theo thời gian khi pod/instance đến và đi. Một proxy re-resolve định kỳ và
hoán đổi upstream set một cách atomic gần như có service discovery động cơ bản
miễn phí; xem [`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md) để biết việc hoán đổi đó cần
xảy ra thế nào mà không làm rơi request đang bay. **DNS round-robin** (nhiều
record `A`, thứ tự xoay vòng) rải client một cách thô: không biết health, answer
được cache pin client trong suốt TTL, và vài resolver lớn có thể đẩy phần lớn
traffic vào một địa chỉ. **GeoDNS** trả answer khác nhau theo vị trí client. Cả
hai đều là công cụ thô so với một load balancer thật
([`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)).

### Bảo mật: DNS mặc định không được xác thực
DNS thuần là cleartext và không ký: bất kỳ ai trên đường đi đều đọc được hoặc
giả mạo answer (cache poisoning). **DNSSEC** ký record để một validating resolver
phát hiện giả mạo (chưa được enforce đầu-cuối rộng rãi); **DoT/DoH** mã hóa
chặng client-tới-resolver. Với proxy, nguy hiểm sắc hơn là **DNS rebinding và
SSRF**: nếu bạn resolve một hostname chịu ảnh hưởng của user, kẻ tấn công có thể
trỏ nó tới `127.0.0.1` hoặc `169.254.169.254`, hoặc làm nó resolve ra địa chỉ
công cộng lúc bạn *validate* và địa chỉ private lúc bạn *connect*. Phòng thủ
bằng cách validate **IP đã resolve**, và connect đúng IP đó, không re-resolve
([`07-security/04-normalization.md`](../07-security/04-normalization.md)).

## Practice

1. Dùng `dig +trace example.com` để xem chuỗi root -> TLD -> authoritative, rồi
   so sánh với `dig example.com` (answer cache của một recursive resolver; xem
   TTL đếm ngược qua hai lần chạy) và `dig example.com AAAA`. Tìm một response
   `NXDOMAIN` và một NODATA (`dig nonexistent.example.com`,
   `dig www.example.com TXT`) và đọc các field `status:` và `ANSWER:`.
2. Chạy `cat /etc/resolv.conf /etc/nsswitch.conf`, rồi
   `strace -f -e trace=openat,connect,sendto getent hosts example.com` và xác
   định: file nào được đọc, nameserver nào được liên hệ, và các query packet. Bắt
   cùng lookup đó bằng `sudo tcpdump -n -i any port 53` và đọc query ID, type và
   response code.
3. So sánh `getent hosts name` với `dig name` cho `localhost`, một tên trong
   `/etc/hosts`, và một tên ngắn phụ thuộc search domain; giải thích mọi khác
   biệt. Đếm số query một lookup làm với `ndots:5` bằng cách thêm nó vào một
   `resolv.conf` test.
4. Viết một chương trình Rust độc lập nhỏ dùng `hickory-resolver` (async) resolve
   một hostname ra nhiều record `A`/`AAAA`, in TTL của chúng, và cho thấy điều gì
   đổi ở lần gọi thứ hai trong khoảng TTL.
5. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), resolve hostname upstream thay vì
   hardcode IP, và re-resolve theo timer tôn trọng TTL của record; mô phỏng việc
   IP backend đổi (hostname -> IP A qua `/etc/hosts` hoặc một `dnsmasq` local,
   rồi IP B) và đo proxy mất bao lâu để nhận ra và có request nào fail trong lúc
   chuyển không.
6. Đọc [`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md) và ghi lại phần nào của nó mà cách
   tiếp cận dựa trên DNS ở mục 5 đã đáp ứng và phần nào còn thiếu.
