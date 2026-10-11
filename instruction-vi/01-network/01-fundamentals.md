# Networking Fundamentals

Đây là bài đầu tiên của lộ trình networking, và mọi file sau đều dựa vào nó. Bài
không giả định bạn biết gì trước. Nếu bạn đã quen với mô hình phân layer, chỉ cần
lướt qua trong khoảng hai mươi phút. Nếu chưa, hãy đọc hai lần rồi làm Practice
trước khi đi tiếp.

Networking là một chồng nhiều ý tưởng nhỏ, tách biệt nhau. Cách nhanh nhất để quên
chúng là học như một đống từ viết tắt. Vì vậy file này đưa cho bạn *bản đồ* trước,
và các file sau sẽ điền chi tiết vào bản đồ đó.

## What to learn

### Hai process nói chuyện với nhau bằng byte
Bỏ hết các từ viết tắt đi, networking chỉ là: hai chương trình, có thể nằm trên hai
máy khác nhau, trao đổi byte qua dây (hoặc sóng radio). Một bên listen, bên kia
connect tới. Mọi thứ khác trong thư mục này (TCP, TLS, HTTP, DNS) là một bộ luật đặt
lên trên việc "gửi byte, nhận byte". Bộ luật này giúp hai chương trình được viết độc
lập, chạy trên hai máy khác nhau, hiểu byte của nhau theo cùng một nghĩa.

Về cấu trúc, [`proxy/`](../../proxy) chỉ là một chương trình nằm ở giữa. Với bất kỳ
ai connect tới nó, nó là "server". Với bất kỳ thứ gì nó connect tới tiếp theo, nó là
"client". Mỗi file protocol trong thư mục này mô tả hành vi của một trong hai vai trò
đó, hoặc cả hai.

### Mô hình phân layer: tấm bản đồ để gắn mọi thứ khác vào
Gửi byte đi nửa vòng trái đất là một bài toán quá lớn để giải một lần. Vì vậy người
ta chia nó thành các **layer**. Mỗi layer chỉ giải đúng một vấn đề, và giải *cho layer
phía trên nó*. Layer chỉ dùng dịch vụ của layer phía dưới. Nó nói chuyện với **layer
cùng cấp ở máy bên kia** thông qua một header nhỏ. Header này chính là toàn bộ "hợp
đồng" của layer đó.

Có hai mô hình đặt tên cho các layer:

| OSI # | Layer TCP/IP | Vấn đề duy nhất nó giải | Đơn vị | Định địa chỉ bằng | Ví dụ | Thiết bị làm việc ở layer này |
|---|---|---|---|---|---|---|
| 7 (cùng 6, 5) | **Application** | byte có *nghĩa* là gì | message | tên / URL | HTTP, DNS, TLS, gRPC | proxy, API gateway |
| 4 | **Transport** | giao cho *process* nào, và (với TCP) giao tin cậy, đúng thứ tự | segment (TCP), datagram (UDP) | port | TCP, UDP, QUIC | L4 load balancer, firewall |
| 3 | **Internet** | tới *host* nào, đi qua nhiều mạng | packet | IP address | IP, ICMP | router |
| 2 | **Link** | tới *hàng xóm* kế tiếp trong một mạng cục bộ | frame | MAC address | Ethernet, Wi-Fi, ARP | switch |
| 1 | (Link) | bit chạy trên dây, cáp quang hay sóng radio | bit | — | cáp, radio | cáp, hub |

Có thể nhớ nó như một chiếc thang, mỗi nấc có phạm vi rộng hơn nấc dưới:
**frame → tới hop kế tiếp, packet → tới đúng host, segment → tới đúng process,
message → tới đúng ý nghĩa.**

Mô hình OSI (7 layer) là *từ vựng* mà mọi người dùng, ví dụ "L4 load balancer" hay
"L7 proxy". TCP/IP (4 layer) mới là thứ thực sự được cài đặt. Layer 5 và 6 của OSI
không tồn tại như phần mềm riêng trong thực tế.

Hãy xem mô hình này là bản đồ, không phải định luật. **TLS** nằm gượng gạo giữa layer
4 và 7. **QUIC** làm việc của transport và hơn thế nữa, ngay trên UDP
([`18-http3.md`](18-http3.md)). **NAT** sửa port của transport layer, nhưng lại chạy
trên một thiết bị ở network layer ([`02-addressing.md`](02-addressing.md)). Khi một
protocol "không vừa vào layer nào", đó là vì mô hình chỉ gần đúng, không phải bạn hiểu
sai.

### Encapsulation: request được bọc như những phong bì lồng nhau
Proxy của bạn gọi `write(fd, b"GET / HTTP/1.1...")`. Khi dữ liệu đi **xuống** stack,
mỗi layer bọc thứ nó nhận từ layer trên bằng header của riêng nó. Bên gửi gọi việc này
là *encapsulate*. Khi dữ liệu đi **lên** ở bên nhận, mỗi layer gỡ header của mình ra
và đưa payload lên layer trên. Việc này gọi là *decapsulate*.

```text
application   [ HTTP: GET / HTTP/1.1 ... ]
transport     [ TCP hdr | HTTP ... ]                      <- thêm port, sequence number
internet      [ IP hdr  | TCP hdr | HTTP ... ]            <- thêm source/dest IP
link          [ Eth hdr | IP hdr | TCP hdr | HTTP ... | FCS ]  <- thêm MAC; đây là frame
wire          0101101...
```

Mỗi thiết bị chỉ đọc sâu đến mức công việc của nó cần:

- **Switch** chỉ đọc Ethernet header.
- **Router** đọc đến IP header. Sau đó nó *bỏ Ethernet header cũ đi và tạo một cái mới*
  cho link kế tiếp.
- **L4 load balancer** đọc đến TCP header.
- **L7 proxy** đọc hết, kể cả HTTP.

Câu hỏi "thiết bị này nhìn sâu bao nhiêu vào các phong bì lồng nhau?" chính là điểm
khác nhau giữa thiết bị L2, L3, L4 và L7
([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)).

### Hop-by-hop và end-to-end
Layer 1–3 làm việc **theo từng hop**. MAC address đổi ở mỗi router, và mỗi router tự
chọn đường forward ([`07-link-layer.md`](07-link-layer.md),
[`08-ip-and-icmp.md`](08-ip-and-icmp.md)).

Layer 4 trở lên làm việc **đầu-cuối** (end-to-end). Sequence number và
retransmission của một TCP connection chỉ có nghĩa giữa hai endpoint. Các router ở
giữa không bao giờ nhìn vào chúng.

Proxy cố ý thay đổi điều này. Nó *kết thúc* một transport connection và mở một
connection thứ hai, nên nó trở thành một "đầu" mới. TCP/TLS session của client kết
thúc tại proxy. Session của upstream bắt đầu từ proxy. Chỉ một ý này đã giải thích vì
sao proxy có thể: thêm TLS, pool connection, retry. Nó cũng giải thích vì sao proxy
phải chuyển IP của client tiếp đi trong một header ([`15-http.md`](15-http.md)).

### Debug từ dưới lên: lỗi ở layer nào?
Mỗi layer phụ thuộc vào layer bên dưới, nên hãy chẩn đoán từ dưới lên:

1. **Link/IP**: tôi có với tới được host không? (`ping`, `ip route`, ARP)
2. **Transport**: port có trả lời không? `connection refused` nghĩa là host ổn nhưng
   không có gì đang listen. Treo nghĩa là packet bị drop. (`ss`, `tcpdump`)
3. **Application**: nó có nói đúng protocol không? (một `403`, một TLS alert, một
   header sai định dạng)

Một lỗi `403` không bao giờ là vấn đề routing, và một lỗi timeout hiếm khi do HTTP
header. [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) dạy các
công cụ để làm việc này. [`21-life-of-a-request.md`](21-life-of-a-request.md) kết thúc
bằng một bảng "triệu chứng → layer".

### Năm mảnh ghép, và mỗi mảnh nằm ở đâu
Phần còn lại của primer này được chia thành các file ngắn thay vì một file dài. Lý do
là mỗi mảnh là một ý tưởng riêng, và sau này bạn sẽ muốn quay lại đúng từng phần, thay
vì đọc lại cả một bức tường chữ.

- **[`02-addressing.md`](02-addressing.md)**: cách định danh một host và một process
  trên host đó. Gồm IP address, port, CIDR notation, và NAT (vì sao địa chỉ mà packet
  tới nơi thường không phải địa chỉ nó được gửi đi).
- **[`03-byte-streams.md`](03-byte-streams.md)**: thứ TCP thực sự đưa cho chương trình
  của bạn (một stream, không phải các message), TCP so với UDP, và handshake như một
  pattern lặp đi lặp lại.
- **[`04-latency-throughput.md`](04-latency-throughput.md)**: bốn con số hay bị nhầm
  với nhau: latency, bandwidth, throughput, RTT. Và vì sao một connection "nhanh" vẫn
  có thể cảm giác chậm.
- **[`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)**: forward proxy, reverse proxy, NAT
  gateway, load balancer, L4 và L7 khác nhau thế nào. Repo này xây một điểm cụ thể
  trong không gian đó, và file này trả lời thẳng "là loại nào, và vì sao".
- **[`06-crypto-basics.md`](06-crypto-basics.md)**: mã hóa symmetric và asymmetric,
  hashing, HMAC, digital signature, certificate/PKI. Đây không phải cryptography như
  một môn học. Nó chỉ đủ để handshake trong [`19-tls.md`](19-tls.md) và chữ ký trong
  [`07-security/02-jwt.md`](../07-security/02-jwt.md) không còn là chuyện thần bí.

Hãy đọc theo đúng thứ tự trên một lần. Sau đó, coi mỗi file là một chỗ tra cứu độc
lập.

Năm file này giải thích connection và packet *là gì*. Nhóm kế tiếp
([`07-link-layer.md`](07-link-layer.md) đến
[`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md)) đi xuống một tầng:
packet di chuyển về mặt vật lý như thế nào, và làm sao để quan sát nó. Còn
[`21-life-of-a-request.md`](21-life-of-a-request.md) ở cuối thư mục kể lại toàn bộ
stack như hành trình của một request.

## Practice
1. Chạy `ss -tlnp` (hoặc `netstat -tlnp`) trên máy của bạn. Xác định từng listening
   port và process sở hữu nó. Với ít nhất ba port, giải thích được vì sao chương
   trình đó chọn port đó.
2. Chạy `curl -v http://example.com`. Trong output, chỉ ra: chỗ nào TCP handshake diễn
   ra, chỗ nào HTTP request được gửi, chỗ nào là response header và chỗ nào là body.
   curl có gắn nhãn cho từng giai đoạn.
3. Đọc các file 02–06 theo thứ tự, rồi quay lại đây. Giải thích mỗi ý bằng một câu:
   socket là gì, vì sao TCP giống như đọc ghi một file, NAT làm gì với source address,
   và [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) thuộc loại proxy nào.
4. Không nhìn tài liệu, vẽ lại stack năm layer. Với mỗi trường hợp sau, liệt kê header
   của layer nào có mặt, và thiết bị nào đọc sâu đến đâu: một ARP request, một `ping`,
   một DNS query qua UDP, `curl http://example.com`. Sau đó đối chiếu với bảng ở trên.
5. Bắt packet của một lần `curl http://example.com` bằng
   `sudo tcpdump -i any -n -e -X port 80`
   ([`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) giải thích các
   flag). Trong hex dump, chỉ ra Ethernet header, IP header, TCP header và phần text
   HTTP, và gọi tên layer của từng phần.
