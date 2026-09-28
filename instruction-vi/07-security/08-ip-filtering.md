# IP Filtering

## What to learn
### Allow/deny list và CIDR matching
Filter theo source IP dựa trên một danh sách CIDR block (ví dụ `10.0.0.0/8`
cho internal, hoặc các dải egress mà một vendor công bố). Match bằng cách
tính xem các bit của IP, sau khi mask theo prefix length của CIDR, có bằng
các bit của network không — đừng tự viết lại chuyện này bằng tay, dùng
`ipnet` hoặc các thao tác bit `Ipv4Addr`/`Ipv6Addr` của std, và luôn hỗ trợ
cả v4 và v6 (một allowlist chỉ có v4 sẽ bị bypass dễ dàng bởi attacker có
kết nối v6, nếu listener nhận cả hai).

```rust
fn ip_in_cidr(ip: std::net::Ipv4Addr, network: std::net::Ipv4Addr, prefix_len: u8) -> bool {
    let mask = u32::MAX.checked_shl(32 - prefix_len as u32).unwrap_or(0);
    u32::from(ip) & mask == u32::from(network) & mask
}
```
Lưu ý `checked_shl` — shift một `u32` đi 32 bit là undefined behavior trong
C và panic trong Rust debug, mà `/0` lại là một CIDR thật người ta hay viết.
Chỉ riêng edge case đó là lý do vì sao "cứ dùng `ipnet`" là lựa chọn đúng
cho code production.

### Cái bẫy IPv4-mapped IPv6
Một listener dual-stack (bind vào `::` với `IPV6_V6ONLY` tắt, mặc định phổ
biến) báo địa chỉ của một client IPv4 dưới dạng địa chỉ IPv6 *mapped* từ
IPv4: `::ffff:10.0.0.1`, không phải `10.0.0.1`. So sánh cái này với rule
`10.0.0.0/8` của bạn dưới dạng địa chỉ v6 sẽ không match — âm thầm.

Hướng nó gãy tùy vào loại danh sách, và cả hai đều tệ: một *allowlist* thì
ngừng match và khóa cửa mọi client IPv4 hợp lệ; một *denylist* thì ngừng
match và cho mọi client IPv4 bị cấm đi thẳng qua. Cái thứ hai là một lỗ hổng
an ninh mà không có test nào dùng client v6 phát hiện ra được.

```rust
// normalize một lần, ở boundary, trước khi bất kỳ rule nào được tra
fn canonical(ip: std::net::IpAddr) -> std::net::IpAddr {
    match ip {
        std::net::IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => std::net::IpAddr::V4(v4),
            None => std::net::IpAddr::V6(v6),
        },
        v4 => v4,
    }
}
```
Gotcha: dùng `to_ipv4_mapped()`, không dùng `to_ipv4()` cũ hơn. Cái sau còn
convert cả địa chỉ IPv4-*compatible* (`::1.2.3.4`, một format đã deprecated)
và, nổi tiếng là, map `::1` thành `0.0.0.1` — nên một kết nối loopback có
thể lộ ra ở đầu bên kia trông như một địa chỉ public bất kỳ.

### Match với danh sách lớn
Scan tuyến tính qua các CIDR thì ổn với chục entry trong config viết tay, và
sai với một threat-intel feed có 100k block phải evaluate trên mỗi request.
Cấu trúc đúng là một prefix trie trên các bit địa chỉ — chính là cấu trúc
của [`13-algorithms/radix-tree.md`](../13-algorithms/radix-tree.md) với key cố định 32 hoặc 128 bit, cho
lookup O(độ dài prefix) không phụ thuộc kích thước danh sách. Crate
`ip_network_table` implement cái này; FIB của kernel cũng làm y hệt cho
routing.

Gotcha: rebuild trie khi config reload và swap nó atomic (pattern `ArcSwap`
của [`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md)) thay vì mutate nó dưới lock — các
request-path read không bao giờ nên block chờ một lần update danh sách.

### Chỉ tin forwarded-for header từ các proxy đã biết
`X-Forwarded-For` (hoặc header PROXY protocol, xem
[`01-network/15-proxy-protocol.md`](../01-network/15-proxy-protocol.md)) là dữ liệu do client cung cấp trừ khi
chính bạn strip và set lại nó ở một trust boundary. Nếu proxy tin mù quáng
bất kỳ giá trị `X-Forwarded-For` nào tới, client nào cũng có thể tự nhận là
`127.0.0.1` hoặc một IP internal nằm trong allowlist và bypass hoàn toàn IP
filtering.

Mô hình đúng: chỉ tin các header forwarded-address khi *TCP peer trực tiếp*
chính nó là một load balancer upstream đã biết và được tin (check địa chỉ
peer thật của socket với một danh sách trusted-proxies); ngược lại, bỏ qua
header đó và dùng địa chỉ peer thật, hoặc strip hẳn header trước khi nó tới
logic filtering của bạn.

### Parse X-Forwarded-For đúng cách
Header này là một danh sách phân tách bằng dấu phẩy mà mỗi hop *append*
vào, nên nó đọc là `client, proxy1, proxy2` — và mọi entry ở bên trái các
hop được tin của bạn là do ai đó bạn không tin viết ra. Attacker gửi
`X-Forwarded-For: 127.0.0.1` và CDN của bạn append địa chỉ thật, ra
`127.0.0.1, 203.0.113.9`. Lấy entry **trái nhất** — cách đọc hiển nhiên
"client gốc" — là đưa cho attacker đúng giá trị họ chọn.

Thuật toán đúng là **rightmost-untrusted**: đi từ danh sách từ phải sang,
bỏ qua các entry nằm trong tập trusted-proxy của bạn; entry đầu tiên không
được tin chính là client thật. Mọi thứ bên trái nó là do attacker kiểm soát
và phải bị loại bỏ, không được log như sự thật.

Gotcha: cũng phải xử lý các trường hợp malformed, vì đó là cách parser bị
bypass — một entry không phải IP hợp lệ, một địa chỉ IPv6 kèm port trong
dấu ngoặc (`[2001:db8::1]:443`), các biến thể whitespace, và một danh sách
50 entry được thiết kế để bắt bạn phải allocate. Reject thay vì đoán, và
giới hạn số entry bạn sẽ parse.

Gotcha: header `Forwarded: for=...;proto=...;by=...` của RFC 7239 là bản
chuẩn hóa của cùng ý tưởng này, với quy tắc quoting riêng. Nếu bạn nhận cả
hai, phải đảm bảo chúng không thể mâu thuẫn — attacker cung cấp một cái và
CDN cung cấp cái khác là một dạng khác của parser differential
([`05-request-smuggling.md`](05-request-smuggling.md)).

### Lưu ý về IP spoofing
Spoof source IP trên TCP là khó trong thực tế (bắt tay 3 bước nghĩa là một
SYN có source giả không thể hoàn thành một kết nối thật mà không thấy được
SYN-ACK), đó là vì sao IP allowlisting mạnh một cách có ý nghĩa cho các
giao thức dựa trên TCP — nhưng chính trust boundary ở *forwarded-header*
phía trên mới là lỗ hổng thực sự khai thác được trong hầu hết incident
thật, không phải raw IP spoofing.

### Những gì IP filtering về cơ bản không làm được
Đáng nói rõ, vì block IP thường được dùng như một biện pháp phòng thủ
chung và yếu khi dùng như vậy:
- **Địa chỉ được chia sẻ.** Carrier-grade NAT đặt hàng ngàn user di động
  sau một địa chỉ; block nó là block cả một thành phố. Mạng đại học và
  công ty cũng vậy.
- **Địa chỉ rẻ.** Một tài khoản cloud thuê một địa chỉ mới với vài cent, và
  các dịch vụ residential-proxy thuê hàng triệu IP consumer thật chính là
  để đánh bại control này.
- **Địa chỉ bị tái cấp.** Một block bạn thêm hôm nay tháng sau rơi vào một
  khách hàng không liên quan, và không ai nhớ vì sao rule đó tồn tại.

Vậy: allowlist mạnh (một tập ngắn, đã biết các peer, ví dụ endpoint admin
hoặc tích hợp đối tác), denylist yếu và tạm thời. Coi một entry denylist
như một công cụ rate-limiting hoặc incident-response có TTL, không phải một
control an ninh vĩnh viễn, và kết hợp nó với [`07-security/07-ratelimit.md`](07-ratelimit.md)
vốn degrade nhẹ nhàng hơn nhiều với các địa chỉ được chia sẻ.

### Ban động, và chặn giới hạn nó
Dạng denylist hữu ích là được sinh ra, không phải viết tay: một client làm
trigger rate limiter hoặc WAF nhiều lần bị ban tạm thời, tránh phải chạy
lại các check đắt đỏ trên traffic đã bị đánh giá rồi.

Có hai ràng buộc để làm việc này an toàn. Mọi entry cần một **TTL** (vài
phút đến vài giờ), vừa vì địa chỉ được chia sẻ vừa vì một autoban list vĩnh
viễn cuối cùng sẽ tự gây ra outage. Và bảng phải **bị chặn giới hạn** — nó
được key bằng dữ liệu do attacker kiểm soát, nên một map không giới hạn là
vector cạn kiệt bộ nhớ được mô tả trong [`13-algorithms/count-min-sketch.md`](../13-algorithms/count-min-sketch.md).
Chặn giới hạn nó và evict (LRU, hoặc TTL cũ nhất trước) thay vì cho nó lớn
mãi.

### Enforce ở đâu: proxy hay kernel
Đến lúc proxy của bạn evaluate một rule, nó đã hoàn thành một bắt tay TCP
và thường cả TLS — những phần đắt đỏ — cho một peer mà bạn đang chuẩn bị từ
chối. Vậy ổn cho các quyết định chính sách trên traffic bình thường, nhưng
vô dụng chống lại một cuộc flood, nơi chi phí trên mỗi kết nối bị reject
chính là thứ attacker đang chi budget của bạn để tạo ra.

Block theo dạng volumetric nên thuộc về lớp thấp hơn: `nftables`/`ipset`
trong kernel, hoặc XDP ở driver ([`16-kernel/10-xdp.md`](../16-kernel/10-xdp.md)), nơi một packet bị
drop trước khi stack cấp một socket. Vai trò của proxy là *quyết định* (nó
có context ứng dụng) và đẩy quyết định đó xuống, chính là feedback loop mà
[`07-security/09-ddos.md`](09-ddos.md) và [`labs/17-ebpf`](../../labs/17-ebpf) xây dựng.

## Practice
Làm theo thứ tự này.

1. Trong [`proxy`](../../proxy), implement CIDR allow/deny trên địa chỉ TCP peer thật,
   dùng `ipnet`, cho cả v4 và v6. **Xong khi** một rule `/0` không panic và
   các rule theo family match đúng.
2. Bind một listener dual-stack và connect qua IPv4. **Xong khi** bạn đã
   thấy địa chỉ peer đến dưới dạng `::ffff:...` và xác nhận rule v4 của bạn
   âm thầm không match — rồi thêm normalization `canonical()` và xác nhận
   chúng match.
3. Thêm `trusted_proxies` và thuật toán XFF rightmost-untrusted. **Xong
   khi** một request từ peer không được tin mang
   `X-Forwarded-For: 127.0.0.1` bị filter theo địa chỉ thật của nó, và một
   request qua trusted proxy với `1.2.3.4, <trusted>` resolve ra `1.2.3.4`.
4. Fuzz parser XFF với input malformed (không phải IP, v6 kèm port trong
   ngoặc, 10k entry, whitespace nhúng vào). **Xong khi** không cái nào
   panic, allocate không giới hạn, hoặc cho ra verdict được tin — xem
   [`12-testing/02-fuzzing.md`](../12-testing/02-fuzzing.md).
5. Đổi CIDR matching tuyến tính thành prefix trie và load một danh sách
   100k entry. **Xong khi** latency match trên mỗi request bằng nhau giữa
   danh sách 10 entry và 100k entry, và một config reload swap bảng mà
   không block các read trên request-path.
6. Thêm ban động có TTL, được kích hoạt bởi vi phạm rate-limit, với một
   bảng bị chặn giới hạn. **Xong khi** một client bị ban bị reject trước
   khi rate limiter chạy, ban hết hạn đúng lịch, và lấp bảng với 1M địa chỉ
   giả plateau về bộ nhớ thay vì tăng mãi.
7. (Stretch) Nối parsing PROXY protocol v2
   ([`01-network/15-proxy-protocol.md`](../01-network/15-proxy-protocol.md)) như một lựa chọn thay thế có cấu
   trúc cho XFF. **Xong khi** IP client thật được lấy lại từ header binary
   và một kết nối *không có* header mong đợi trên một listener PROXY
   protocol bị reject thay vì bị parse như HTTP.
