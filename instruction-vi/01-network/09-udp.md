# UDP

UDP là transport protocol còn lại. Nó không có connection, không đảm bảo thứ
tự, không retransmit. QUIC/HTTP/3 và DNS đều được xây trên UDP. Khi bạn biết rõ
UDP *không* cho bạn những gì, bạn sẽ hiểu vì sao TCP phải có nhiều cơ chế như vậy
([`12-tcp.md`](12-tcp.md), [`13-tcp-reliability.md`](13-tcp-reliability.md)).

## What to learn

### UDP là gì: port cộng với một checksum
UDP header dài 8 byte: source port, destination port, length, checksum. Protocol
chỉ có vậy. IP đưa packet tới đúng một *máy* ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)).
Việc duy nhất UDP thêm vào là **port**, để chọn ra *process* nào trên máy đó nhận
packet ([`02-addressing.md`](02-addressing.md)).

Mọi thứ khác mà một transport protocol có thể làm đều không có: thiết lập
connection, giữ thứ tự, đảm bảo giao hàng, điều chỉnh tốc độ. Checksum chỉ dùng để
phát hiện dữ liệu bị hỏng. Packet sai checksum bị drop mà không báo gì.

### Datagram giữ ranh giới message, stream thì không
Một lần `sendto` 300 byte tạo ra đúng một datagram. Ở phía bên kia, một lần
`recvfrom` trả về đúng 300 byte đó, hoặc không trả về gì. Ranh giới message được
giữ nguyên. TCP thì khác: hai lần write có thể tới thành một lần read, và một lần
write có thể bị tách thành hai lần read ([`03-byte-streams.md`](03-byte-streams.md)).

Đổi lại có một điều cần cẩn thận: nếu buffer của bạn nhỏ hơn datagram, phần thừa
bị **cắt bỏ và mất luôn**. Nó không được giữ lại cho lần read sau.

```rust
let sock = tokio::net::UdpSocket::bind("0.0.0.0:5353").await?;
let mut buf = [0u8; 1500];
let (n, peer) = sock.recv_from(&mut buf).await?; // một datagram nguyên vẹn
sock.send_to(&buf[..n], peer).await?;
```

UDP không có `accept`, và không có socket riêng cho từng peer. Một socket nhận
packet từ tất cả mọi người, và mỗi lần `recv_from` cho bạn biết ai gửi. Nếu bạn cần
khái niệm "connection", ứng dụng phải tự xây.

### Cái gì có thể sai, và ai xử lý
Một UDP datagram có thể bị **mất**, bị **nhân đôi**, **đến sai thứ tự**, hoặc **đến
trễ**. Bên gửi không bao giờ được báo. Cách duy nhất để biết là gián tiếp: nếu
không có gì listen ở port đích, bạn nhận một ICMP port-unreachable
([`08-ip-and-icmp.md`](08-ip-and-icmp.md)). Nếu bạn dùng *connected* UDP socket
(gọi `connect()` trên UDP socket chỉ để ghi nhớ peer mặc định), ICMP đó hiện ra
thành lỗi `ECONNREFUSED` ở lần gọi kế tiếp.

Bất cứ thứ gì cần độ tin cậy đều phải tự thêm vào. DNS tự gửi lại query sau
timeout. QUIC tự làm lại việc sắp xếp thứ tự, loss recovery, congestion control và
mã hóa, tất cả ở user space ([`18-http3.md`](18-http3.md)).

### Không có congestion control: bên gửi phải tự biết điều
TCP tự chậm lại khi mạng bị congestion. UDP thì cứ gửi ở tốc độ tối đa, kể cả khi
link đã quá tải. Việc đó làm mọi thứ tệ hơn cho tất cả, kể cả chính bạn. Vì vậy
protocol chạy trên UDP phải tự có congestion control (QUIC có), hoặc giữ lưu lượng
rất thấp (DNS). Một proxy đứng trước lưu lượng UDP không nên forward không giới
hạn. Lý do giống như việc buffer không giới hạn là bug.

### Giới hạn kích thước và fragmentation
Nếu datagram lớn hơn path MTU, IP sẽ fragment nó. Chỉ cần mất một fragment là mất
cả datagram. Vì vậy ứng dụng UDP thường giữ payload dưới khoảng 1200 byte để vừa
trong một packet trên gần như mọi đường đi. Kích thước packet tối thiểu của QUIC là
1200 cũng vì lý do này. Giới hạn cứng là 65507 byte (65535 trừ header IP và UDP).
DNS answer quá lớn so với giới hạn này là lý do DNS chuyển sang TCP
([`14-dns.md`](14-dns.md)).

### UDP và các middlebox có state
NAT và firewall vẫn theo dõi "flow" của UDP bằng 4-tuple, dù UDP không có khái niệm
flow. Vì UDP không có FIN để báo kết thúc, mapping này bị xóa bằng **timer**,
thường sau 30–120 giây. Nếu một QUIC connection hoặc UDP session idle quá lâu, NAT
mapping có thể bị xóa và connection ngừng hoạt động mà không có thông báo nào.

Vì thế các protocol gửi keepalive định kỳ. QUIC còn hỗ trợ **connection migration**:
nó nhận diện connection bằng connection ID thay vì dựa vào 4-tuple
([`18-http3.md`](18-http3.md)).

### Proxy UDP và gotcha reflection attack
Giả mạo source address của UDP rất dễ, vì không có handshake nào để kiểm tra. Kẻ
tấn công gửi một query nhỏ, với source address là địa chỉ của nạn nhân, tới một
server sẽ trả lời bằng dữ liệu lớn hơn nhiều. Kết quả là nạn nhân bị dội một lượng
traffic lớn. Đây gọi là **amplification** (ví dụ: open DNS resolver, memcached qua
UDP, NTP).

Mọi UDP service bạn expose ra ngoài phải làm một trong hai việc: xác minh source
address, hoặc không bao giờ reply nhiều hơn lượng dữ liệu đã nhận
([`07-security/09-ddos.md`](../07-security/09-ddos.md)). QUIC chọn cách thứ nhất:
server không được gửi quá 3 lần số byte đã nhận từ một địa chỉ chưa được xác minh,
và dùng retry token để xác minh địa chỉ đó. Về bản thân QUIC, xem
[`18-http3.md`](18-http3.md).

Load balancing cho UDP cũng cần affinity. Vì không có connection để làm điểm neo,
balancer phải gắn một 4-tuple (hoặc một QUIC connection ID) vào một backend cố
định.

## Practice

1. Chạy UDP listener bằng `nc -u -l 9999`, rồi gửi datagram bằng
   `echo -n hello | nc -u -w1 127.0.0.1 9999`. Gửi hai datagram liên tiếp và xác
   nhận mỗi cái tới thành một message riêng. Đối chiếu với TCP, nơi `nc -l` có thể
   gộp chúng lại.
2. Viết một tokio UDP echo nhỏ (scratch project, không nằm trong workspace) dựa trên
   đoạn code ở trên. Thu nhỏ receive buffer còn 4 byte, gửi 10 byte, và quan sát
   phần bị cắt.
3. Chạy `sudo tcpdump -n -i lo udp port 9999` trong lúc làm mục 1. Dùng
   `tcpdump -n -X` để xem hex dump, và chỉ ra từng field của UDP header 8 byte.
4. Gửi tới một UDP port không có listener, dùng một socket *đã connect* (scratch
   program: `UdpSocket::connect` rồi `send` hai lần). Quan sát `ECONNREFUSED` ở lần
   gọi thứ hai. Đối chiếu nó với ICMP message bạn bắt được bằng `tcpdump icmp`.
5. Chạy `tc qdisc add dev lo root netem loss 30%` (xem
   [`12-testing/03-chaos.md`](../12-testing/03-chaos.md)) và gửi 100 datagram có
   đánh số. Đếm xem bao nhiêu cái tới nơi. Thêm `delay 20ms 10ms` để xem có cái nào
   tới sai thứ tự không. Giải thích vì sao bên gửi không biết gì về việc đó.
6. So sánh hình dạng của [`labs/00-tcp-server`](../../labs/00-tcp-server) (một vòng
   lặp `accept`, spawn một task cho mỗi connection) với thứ một UDP service cần
   (một socket, một vòng lặp `recv_from`, và state của từng peer lưu trong một map
   với key là source address). Ghi vào notes những gì phải đổi. Đây chính là bước
   chuyển cấu trúc mà [`labs/09-http3`](../../labs/09-http3) yêu cầu.
