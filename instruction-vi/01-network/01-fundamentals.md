# Networking Fundamentals

Bài học đầu tiên của lộ trình networking, và là bài mà mọi file sau đều dựa
vào. Nó không giả định gì: nếu bạn đã biết mô hình phân lớp, lướt qua trong
hai mươi phút; nếu chưa, hãy đọc hai lần và làm Practice trước khi đi tiếp.
Networking là một chồng các ý tưởng nhỏ, tách biệt — cách nhanh nhất để quên
chúng là học như một đống từ viết tắt, nên file này cho bạn *bản đồ* trước
và các file sau điền vào địa hình.

## What to learn

### Hai process, nói chuyện qua byte
Lột bỏ mọi từ viết tắt thì networking chỉ là: hai chương trình, có thể
nằm trên các máy khác nhau, trao đổi byte qua dây (hoặc sóng radio). Một
bên listen, một bên connect tới. Mọi thứ khác trong thư mục này — TCP,
TLS, HTTP, DNS — là một tập luật đặt chồng lên "gửi byte, nhận byte" để
hai chương trình được viết độc lập, trên hai máy tính khác nhau, đồng ý
với nhau về ý nghĩa của những byte đó.

Về mặt cấu trúc, [`proxy/`](../../proxy) chỉ là một chương trình đứng ở giữa: nó là
"server" đối với bất kỳ ai connect tới nó, và là "client" đối với bất kỳ
thứ gì nó connect tới tiếp theo. Mỗi file protocol trong thư mục này mô tả
hành vi từ một hoặc cả hai vai trò đó.

### Mô hình phân lớp: tấm bản đồ mà mọi thứ khác treo lên
Gửi byte qua nửa vòng trái đất là bài toán quá lớn để giải một lần, nên nó được
chia thành các **layer**. Mỗi layer giải đúng một vấn đề *cho layer phía trên
nó*, chỉ dùng layer phía dưới, và nói chuyện với **layer đồng cấp ở máy bên kia**
qua một header nhỏ — header đó là toàn bộ hợp đồng của layer. Hai mô hình đặt
tên các layer:

| OSI # | Layer TCP/IP | Vấn đề duy nhất nó giải | Đơn vị | Định địa chỉ bằng | Ví dụ | Thiết bị làm việc ở đây |
|---|---|---|---|---|---|---|
| 7 (cùng 6, 5) | **Application** | byte *mang nghĩa* gì | message | tên / URL | HTTP, DNS, TLS, gRPC | proxy, API gateway |
| 4 | **Transport** | *process* nào, và (TCP) tin cậy, đúng thứ tự | segment (TCP), datagram (UDP) | port | TCP, UDP, QUIC | L4 load balancer, firewall |
| 3 | **Internet** | *host* nào, qua nhiều mạng | packet | IP address | IP, ICMP | router |
| 2 | **Link** | *hàng xóm* kế tiếp trên một mạng cục bộ | frame | MAC address | Ethernet, Wi-Fi, ARP | switch |
| 1 | (Link) | bit trên dây, cáp quang hay sóng radio | bit | — | cáp, radio | cáp, hub |

Hãy nhớ nó như một chiếc thang có phạm vi rộng dần: **frame -> hop kế tiếp,
packet -> đúng host, segment -> đúng process, message -> đúng ý nghĩa.** Mô hình
OSI (7 layer) là *từ vựng* mọi người dùng — "L4 load balancer", "L7 proxy" —
còn TCP/IP (4 layer) là thứ thực sự được cài đặt; layer 5 và 6 của OSI không tồn
tại như phần mềm riêng trong thực tế. Hãy coi mô hình là bản đồ, không phải
định luật: **TLS** nằm gượng gạo giữa 4 và 7, **QUIC** làm transport và hơn thế
trên UDP ([`18-http3.md`](18-http3.md)), và **NAT** sửa port của transport-layer từ một box
network-layer ([`02-addressing.md`](02-addressing.md)). Khi một protocol "không vừa một layer", đó là
mô hình gần đúng, không phải bạn hiểu sai.

### Encapsulation: một request, bọc như các phong bì lồng nhau
Proxy của bạn gọi `write(fd, b"GET / HTTP/1.1...")`. Đi **xuống** stack, mỗi layer
bọc thứ nó nhận từ trên bằng header của riêng nó (bên gửi *encapsulate*); đi
**lên** ở bên nhận, mỗi layer bóc header của mình và đưa payload lên (nó
*decapsulate*).

```text
application   [ HTTP: GET / HTTP/1.1 ... ]
transport     [ TCP hdr | HTTP ... ]                      <- thêm port, sequence number
internet      [ IP hdr  | TCP hdr | HTTP ... ]            <- thêm source/dest IP
link          [ Eth hdr | IP hdr | TCP hdr | HTTP ... | FCS ]  <- thêm MAC; đây là frame
wire          0101101...
```

Mỗi thiết bị chỉ đọc sâu tới mức công việc của nó cần: **switch** đọc Ethernet
header; **router** đọc tới IP header, rồi *vứt Ethernet header đi và dựng cái
mới* cho link kế tiếp; **L4 load balancer** đọc tới TCP; **L7 proxy** đọc hết.
"Thiết bị này nhìn sâu bao nhiêu vào các phong bì lồng nhau?" chính là toàn bộ
khác biệt giữa thiết bị L2, L3, L4 và L7 ([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)).

### Hop-by-hop vs end-to-end
Layer 1–3 làm việc **từng hop**: MAC address đổi ở mỗi router, và mỗi router tự
đưa ra lựa chọn forward ([`07-link-layer.md`](07-link-layer.md), [`08-ip-and-icmp.md`](08-ip-and-icmp.md)). Layer 4 trở lên làm việc
**đầu-cuối**: sequence number và retransmission của một TCP connection chỉ giữa
hai endpoint, và router ở giữa không bao giờ nhìn chúng. Proxy thay đổi điều đó
một cách có chủ đích: nó *kết thúc* một transport connection và mở một cái thứ
hai, nên nó trở thành một "đầu" mới — TCP/TLS session của client kết thúc ở
proxy, và của upstream bắt đầu từ đó. Ý tưởng duy nhất này giải thích vì sao
proxy thêm được TLS, pool connection, retry, và vì sao chúng cũng phải gửi lại
IP của client trong một header ([`15-http.md`](15-http.md)).

### Debug từ dưới lên: là layer nào?
Vì mỗi layer phụ thuộc layer bên dưới, hãy chẩn đoán từ dưới lên:

1. **Link/IP** — tôi có với tới host không? (`ping`, `ip route`, ARP)
2. **Transport** — port có trả lời không? (`connection refused` = host ổn, không
   có gì listen; treo = packet bị drop) (`ss`, `tcpdump`)
3. **Application** — nó có nói đúng protocol không? (một `403`, một TLS alert,
   một header sai định dạng)

Một `403` không bao giờ là vấn đề routing và một timeout hiếm khi là vấn đề
HTTP header. [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) dạy các công cụ và
[`21-life-of-a-request.md`](21-life-of-a-request.md) kết thúc bằng một bảng triệu chứng-sang-layer.

### Sáu mảnh ghép, và mỗi mảnh nằm ở đâu
Phần tiếp theo của primer này được chia thành các file ngắn thay vì một file dài,
vì mỗi mảnh thực sự là một ý tưởng tách biệt và sau này bạn sẽ muốn quay
lại từng phần riêng lẻ thay vì đọc lại cả một bức tường chữ:

- **[`02-addressing.md`](02-addressing.md)** — cách một host và một process trên đó được định
  danh: IP address, port, CIDR notation, và NAT (vì sao địa chỉ mà một
  packet đến với thường không phải địa chỉ nó được gửi từ).
- **[`03-byte-streams.md`](03-byte-streams.md)** — thứ TCP thực sự đưa cho chương trình của bạn
  (một stream, không phải các message), TCP vs UDP, và handshake như một
  pattern lặp lại.
- **[`04-latency-throughput.md`](04-latency-throughput.md)** — bốn con số người ta hay lẫn lộn:
  latency, bandwidth, throughput, RTT — và vì sao một connection "nhanh" vẫn
  có thể cảm giác chậm.
- **[`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)** — forward proxy vs reverse proxy vs NAT
  gateway vs load balancer vs L4 vs L7. Repo này xây một điểm cụ thể trong
  không gian đó, và file này trả lời thẳng "cái nào, và vì sao".
- **[`06-crypto-basics.md`](06-crypto-basics.md)** — mã hóa symmetric vs asymmetric, hashing,
  HMAC, digital signature, certificate/PKI. Không phải cryptography như
  một ngành học — chỉ đủ để handshake trong [`19-tls.md`](19-tls.md) và chữ ký trong
  [`07-security/02-jwt.md`](../07-security/02-jwt.md) không còn là phép màu.

Đọc chúng theo thứ tự đó một lần; sau đó, coi mỗi file là một điểm tra cứu
độc lập.

Sáu file này giải thích connection và packet *là gì*. Nhóm kế tiếp
([`07-link-layer.md`](07-link-layer.md) đến [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md)) đi xuống một tầng — packet
di chuyển về mặt vật lý ra sao, và cách nhìn vào nó — còn
[`21-life-of-a-request.md`](21-life-of-a-request.md) ở cuối thư mục kể lại toàn bộ stack như một request.

## Practice
1. Chạy `ss -tlnp` (hoặc `netstat -tlnp`) trên máy của bạn và xác định mọi
   listening port cùng process nào sở hữu nó — xác nhận bạn giải thích
   được, với ít nhất ba trong số đó, vì sao chương trình đó chọn port đó.
2. Chạy `curl -v http://example.com` và xác định, trong output, chỗ nào
   TCP handshake diễn ra, chỗ nào HTTP request được gửi, và chỗ nào là
   response header vs body — curl gắn nhãn từng giai đoạn.
3. Đọc file 02–06 theo thứ tự, rồi quay lại đây và giải thích, mỗi
   ý một câu: một socket là gì, vì sao TCP cảm giác giống một file, NAT
   làm gì với source address, và [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) thuộc loại proxy
   nào.
4. Từ trí nhớ, vẽ stack năm layer và, với mỗi thứ sau, liệt kê header của layer nào
   có mặt và thiết bị nào đọc sâu tới đâu: một ARP request, một `ping`, một DNS
   query qua UDP, `curl http://example.com`. Đối chiếu với bảng ở trên.
5. Bắt một lần `curl http://example.com` bằng `sudo tcpdump -i any -n -e -X port 80`
   ([`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) giải thích các flag) và chỉ ra Ethernet, IP và TCP
   header cùng text HTTP trong hex dump, gọi tên layer của từng cái.
