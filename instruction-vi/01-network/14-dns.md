# DNS

DNS biến tên thành địa chỉ. File này giải thích: cấu trúc phân cấp của DNS, một
lần lookup thực sự làm gì trên máy bạn, wire format, caching, và vì sao một proxy
chạy lâu phải coi DNS là dữ liệu luôn thay đổi.

## What to learn

### Một cái tên là một đường đi trong cây
`www.example.com.` được đọc từ phải sang trái: root (`.`), top-level domain (`com`),
domain (`example`), và host label (`www`). Cây này được chia thành các **zone**.
Mỗi zone do một nhóm server **authoritative** quản lý.

Zone cha **delegate** zone con bằng record `NS`. Ví dụ: "`example.com` do
`ns1.example.net` phục vụ". Nếu nameserver nằm ngay trong zone mà nó phục vụ, zone
cha phải kèm thêm record `A` gọi là **glue**. Nếu không, bạn sẽ cần DNS để tìm ra
chính DNS server.

13 root server và các TLD server không lưu record của host nào. Chúng chỉ lưu
delegation, tức là chỉ cho bạn biết hỏi ai tiếp. Nhờ vậy DNS mới scale được.

### Recursive và authoritative resolution
**Recursive resolver** (của ISP, hoặc loại công cộng như 1.1.1.1) đi theo chuỗi này
thay cho bạn:

1. Hỏi root server, nhận về danh sách server của `com`.
2. Hỏi server của `com`, nhận về server của `example.com`.
3. Hỏi server của `example.com`, nhận câu trả lời.

Nó cache kết quả ở mỗi bước. Server **authoritative** là nguồn dữ liệu gốc của một
zone và chỉ trả lời về những gì nó sở hữu.

Chương trình của bạn chỉ nói chuyện với recursive resolver, hầu như không bao giờ
hỏi thẳng authoritative server. Bạn cần nắm chuỗi này để debug khi một thay đổi DNS
"chưa propagate": bạn phải biết layer cache nào đang giữ dữ liệu cũ.

### Chuyện gì xảy ra khi chương trình resolve một cái tên
Code của bạn gọi `getaddrinfo` (`ToSocketAddrs` và `curl` đều dùng nó). Trên Linux,
hàm này tra theo thứ tự cấu hình trong `/etc/nsswitch.conf`: trước hết là
`/etc/hosts`, sau đó là DNS. Để dùng DNS, nó đọc `/etc/resolv.conf`, trong đó có:

- các địa chỉ `nameserver`. Thường là một stub local như `127.0.0.53` của
  `systemd-resolved`, hoặc DNS của cluster nếu chạy trong container.
- các `search` domain.
- `options ndots:N`.

**Search list** là chỗ nhiều người vấp. Nếu tên có ít hơn `ndots` dấu chấm, resolver
sẽ thử ghép từng search suffix vào *trước*, rồi mới thử tên gốc. Kubernetes đặt
`ndots:5`. Vì vậy khi bạn resolve `api.example.com` (2 dấu chấm), nó thử lần lượt:

- `api.example.com.default.svc.cluster.local`
- `api.example.com.svc.cluster.local`
- `api.example.com.cluster.local`

Đó là ba query lãng phí (mỗi cái trả về `NXDOMAIN`) trước khi tới query thật, và nó
xảy ra ở mọi lookup khi cache nguội. Thêm dấu chấm cuối (`api.example.com.`) để
đánh dấu tên đầy đủ, resolver sẽ bỏ qua search list.

Hai lưu ý nữa:
- `getaddrinfo` của glibc **không tự có cache**. Mỗi lần gọi là một query, trừ khi
  có stub resolver hoặc `nscd` đứng trước.
- musl (Alpine) xử lý một số chi tiết khác glibc, ví dụ search domain. Lỗi kiểu
  "chạy trên laptop mà fail trong container" thường là do chỗ này.

### Trên wire: một query, một response
Một DNS message gồm: header (ID 16-bit, flags, các bộ đếm), một question (name,
type, class), và các section answer/authority/additional.

Query thường đi qua **UDP port 53** ([`09-udp.md`](09-udp.md)). ID 16-bit dùng để
khớp response với query. Cùng với source port, ID là thứ duy nhất giúp bạn phân biệt
reply thật với reply giả mạo, nên source port phải được chọn ngẫu nhiên.

Nếu answer quá lớn, server set cờ **TC (truncated)** và client **retry qua TCP**.
Điều này cũng xảy ra với response DNSSEC/EDNS lớn. **EDNS0** nâng kích thước UDP an
toàn lên cao hơn mức 512 byte cũ.

Response code cho biết chuyện gì đã xảy ra:

- `NOERROR` kèm answer: thành công. `NOERROR` mà **không có** answer (**NODATA**)
  nghĩa là tên có tồn tại nhưng không có record thuộc loại bạn hỏi. Ví dụ: host chỉ
  có `A` mà bạn hỏi `AAAA`.
- `NXDOMAIN`: tên không tồn tại.
- `SERVFAIL`: resolver không lấy được câu trả lời (delegation hỏng, lỗi DNSSEC,
  upstream timeout). `REFUSED`: resolver từ chối trả lời bạn.

### Các record type quan trọng với proxy
- `A`/`AAAA`: địa chỉ IPv4/IPv6 của upstream.
- `CNAME`: alias. Một tên đã có `CNAME` thì không được có record loại khác, nên bạn
  không thể đặt `CNAME` ở zone apex.
- `SRV`: host + port + priority + weight. Nhiều hệ thống service discovery dựa vào
  loại này.
- `NS`/`SOA`: delegation và metadata của zone.
- `TXT`: dùng cho ACME DNS-01 challenge khi tự động cấp certificate
  ([`19-tls.md`](19-tls.md)).
- `PTR`: reverse lookup, từ IP ra tên. Log và mail dùng nó.

Một proxy resolve hostname của upstream thường chỉ cần `A`/`AAAA`, đôi khi thêm `SRV`.

### TTL và caching, cho cả answer dương lẫn âm
TTL cho mọi cache bên dưới (OS, resolver, proxy của bạn) biết một record được dùng
lại trong bao lâu. Nếu proxy tự resolve hostname của upstream, nó cần cache riêng và
phải expire record theo TTL. Nếu cứ dùng IP cũ sau khi backend đã đổi địa chỉ, traffic
sẽ bị đẩy vào một địa chỉ không còn ai nhận.

**Answer âm cũng được cache.** Một `NXDOMAIN` được nhớ trong khoảng thời gian bằng
giá trị minimum của `SOA` trong zone. Vì vậy, một record bạn *vừa tạo* có thể vẫn
"không tồn tại" trong vài phút ở những resolver đã hỏi trước đó.

Gotcha trong production: một số recursive resolver hoặc client library bỏ qua hoặc
nâng lên các TTL rất thấp (dưới 5 giây). Điều này làm hỏng các hệ thống failover dựa
trên DNS. Đừng nghĩ rằng đặt TTL 1 giây thì sẽ failover trong 1 giây.

Và một điều nữa: connection đã được thiết lập sẽ không tự resolve lại. Một pooled
connection tới IP cũ vẫn tiếp tục dùng IP đó
([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)).

### Resolve trong Rust: blocking và async
`getaddrinfo` của libc, hàm mà `std::net::ToSocketAddrs` gọi, là hàm *blocking*. Nó
cũng tự parse cấu hình của OS (`/etc/resolv.conf`, `/etc/hosts`). Nếu bạn gọi nó
trực tiếp trong một async task, nó sẽ chặn cả executor thread. Có hai cách tránh:

- Chạy nó trong `tokio::task::spawn_blocking`. `tokio::net::lookup_host` làm đúng
  việc này.
- Dùng một resolver crate async thuần như `hickory-resolver`. Crate này nói chuyện
  DNS trực tiếp qua UDP/TCP, cache theo TTL, và cho bạn chọn nameserver nào được
  hỏi.

```rust
// resolve blocking được làm ngoài async executor thread
let addrs = tokio::task::spawn_blocking(|| {
    "backend.internal:8080".to_socket_addrs()
})
.await??;
```

Một answer có thể chứa nhiều địa chỉ, cả IPv4 lẫn IPv6. Cách connect đúng là thử
chúng theo một thứ tự hợp lý. Kỹ thuật này gọi là **Happy Eyeballs**: thử IPv6
trước, và nếu sau khoảng 250 ms chưa connect được thì bắt đầu thử IPv4 song song.
Nhờ vậy, một đường `AAAA` bị hỏng không làm mỗi request mất cả một timeout dài. Nếu
bạn gọi `TcpStream::connect(name)` thẳng, nó thử các địa chỉ lần lượt từng cái, và
mỗi cái được chờ hết một connect timeout đầy đủ.

### DNS làm service discovery và phân phối tải
Nhiều hệ thống (Kubernetes headless Service, Consul DNS interface) công khai service
registry của chúng *dưới dạng* DNS. Một record `A` trả về nhiều IP, và danh sách này
thay đổi khi pod/instance được thêm hoặc bị gỡ.

Nếu proxy re-resolve định kỳ và thay upstream set một cách atomic, bạn có ngay một
dạng service discovery động đơn giản. Xem
[`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md) để biết
cách thay mà không làm rơi request đang xử lý dở.

**DNS round-robin** (nhiều record `A`, thứ tự xoay vòng) chỉ phân phối client một
cách thô sơ:
- Nó không biết backend nào healthy.
- Answer nằm trong cache sẽ giữ client ở nguyên một backend trong suốt TTL.
- Một số resolver lớn có thể dồn phần lớn traffic vào một địa chỉ.

**GeoDNS** trả answer khác nhau theo vị trí của client. Cả hai đều thô sơ so với một
load balancer thật ([`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)).

### Bảo mật: DNS mặc định không được xác thực
DNS thuần là cleartext và không có chữ ký. Bất kỳ ai nằm trên đường đi đều đọc được,
hoặc giả mạo được answer (gọi là cache poisoning). **DNSSEC** ký các record để một
validating resolver phát hiện được giả mạo, nhưng chưa được enforce rộng rãi từ đầu
đến cuối. **DoT/DoH** mã hóa đoạn từ client tới resolver.

Với proxy, mối nguy cụ thể hơn là **DNS rebinding và SSRF**. Nếu bạn resolve một
hostname mà user kiểm soát được, kẻ tấn công có thể:

- Trỏ nó tới `127.0.0.1` hoặc `169.254.169.254`.
- Hoặc làm nó resolve ra địa chỉ public lúc bạn *validate*, rồi ra địa chỉ private
  lúc bạn *connect*.

Cách phòng thủ: validate **IP đã resolve**, rồi connect đúng IP đó, không resolve lại
([`07-security/04-normalization.md`](../07-security/04-normalization.md)).

## Practice

1. Dùng `dig +trace example.com` để xem chuỗi root → TLD → authoritative. So sánh
   với `dig example.com` (answer lấy từ cache của recursive resolver; chạy hai lần và
   xem TTL giảm dần) và `dig example.com AAAA`. Tìm một response `NXDOMAIN` và một
   response NODATA (`dig nonexistent.example.com`, `dig www.example.com TXT`). Đọc
   các field `status:` và `ANSWER:`.
2. Chạy `cat /etc/resolv.conf /etc/nsswitch.conf`, rồi
   `strace -f -e trace=openat,connect,sendto getent hosts example.com`. Xác định: file
   nào được đọc, nameserver nào được liên lạc, và các query packet. Bắt cùng lookup
   đó bằng `sudo tcpdump -n -i any port 53` và đọc query ID, type, response code.
3. So sánh `getent hosts name` với `dig name` cho ba trường hợp: `localhost`, một tên
   có trong `/etc/hosts`, và một tên ngắn phụ thuộc vào search domain. Giải thích mọi
   khác biệt. Thêm `ndots:5` vào một `resolv.conf` thử nghiệm và đếm số query của một
   lần lookup.
4. Viết một chương trình Rust độc lập nhỏ dùng `hickory-resolver` (async) để resolve
   một hostname ra nhiều record `A`/`AAAA`. In TTL của chúng, và cho thấy điều gì thay
   đổi ở lần gọi thứ hai trong khoảng TTL.
5. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), resolve hostname của
   upstream thay vì hardcode IP, và re-resolve theo timer, tôn trọng TTL của record.
   Mô phỏng việc IP của backend đổi (hostname → IP A, rồi → IP B, qua `/etc/hosts`
   hoặc một `dnsmasq` local). Đo xem proxy mất bao lâu để nhận ra, và có request nào
   fail trong lúc chuyển không.
6. Đọc [`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md). Ghi
   lại những phần cách làm dựa trên DNS ở mục 5 đã đáp ứng được, và những phần còn
   thiếu.

---

<!-- nav -->
← Trước: [`01-network/13-tcp-reliability.md`](13-tcp-reliability.md) · [Mục lục](00-README.md) · Tiếp theo: [`01-network/15-http.md`](15-http.md) →
