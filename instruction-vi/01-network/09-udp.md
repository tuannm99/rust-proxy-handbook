# UDP

Transport protocol còn lại: không connection, không thứ tự, không
retransmission — và là nền mà QUIC/HTTP/3 và DNS được xây trên đó. Biết chính
xác UDP *không* cho bạn những gì là điều khiến bộ máy của TCP
([`12-tcp.md`](12-tcp.md), [`13-tcp-reliability.md`](13-tcp-reliability.md)) trở nên dễ hiểu.

## What to learn

### UDP là gì: port cộng với một checksum
UDP header dài 8 byte: source port, destination port, length, checksum. Đó là
toàn bộ protocol. IP giao packet tới một *máy* ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)); đóng
góp duy nhất của UDP là **port**, chọn ra *process* trên máy đó
([`02-addressing.md`](02-addressing.md)). Mọi thứ khác mà một transport có thể làm — thiết lập,
thứ tự, đảm bảo giao hàng, thích ứng tốc độ — đơn giản là không có. Checksum
phát hiện hỏng dữ liệu; packet fail checksum bị bỏ âm thầm.

### Datagram giữ ranh giới message; stream thì không
Một lần `sendto` 300 byte tạo ra một datagram, và một lần `recvfrom` ở phía
bên kia trả về đúng 300 byte đó, hoặc không gì cả. Ranh giới message được bảo
toàn — ngược với byte stream của TCP, nơi hai lần write có thể tới thành một
lần read và một write có thể thành hai ([`03-byte-streams.md`](03-byte-streams.md)). Cái giá:
nếu buffer của bạn nhỏ hơn datagram, phần dư bị **cắt và mất**, không để lại
cho lần read sau.

```rust
let sock = tokio::net::UdpSocket::bind("0.0.0.0:5353").await?;
let mut buf = [0u8; 1500];
let (n, peer) = sock.recv_from(&mut buf).await?; // một datagram nguyên vẹn
sock.send_to(&buf[..n], peer).await?;
```

Không có `accept` và không có socket riêng cho từng peer: một socket nhận từ
tất cả mọi người, và mỗi `recv_from` cho biết ai gửi. State "connection", nếu
có, là thứ ứng dụng của bạn tự xây.

### Cái gì có thể sai, và ai xử lý
UDP datagram có thể bị **mất**, **trùng**, **đảo thứ tự**, và **trễ**; bên gửi
không bao giờ được báo. Phản hồi duy nhất là gián tiếp: một ICMP
port-unreachable nếu không có gì listen ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)), mà một
connected UDP socket (`connect()` trên UDP socket chỉ ghi lại peer mặc định)
sẽ lộ ra thành `ECONNREFUSED` ở lần gọi kế tiếp. Bất cứ thứ gì cần độ tin cậy
phải tự thêm: DNS tự retry query sau timeout; QUIC dựng lại thứ tự, loss
recovery, congestion control và mã hóa ở user space ([`18-http3.md`](18-http3.md)).

### Không có congestion control nghĩa là bên gửi phải lịch sự
TCP chậm lại khi mạng congested; UDP sẵn sàng gửi ở tốc độ line rate vào một
link đã bão hòa và làm mọi thứ tệ hơn cho tất cả, kể cả chính nó. Protocol xây
trên UDP phải tự implement congestion control (QUIC có) hoặc giữ lưu lượng
thấp (DNS). Một proxy đứng trước lưu lượng UDP nên coi forward không giới hạn
là bug, vì cùng lý do nó coi buffer không giới hạn là bug.

### Giới hạn kích thước và fragmentation
Datagram lớn hơn path MTU sẽ bị IP fragment, và mất bất kỳ fragment nào là mất
datagram — nên ứng dụng UDP giữ payload dưới ~1200 byte để vừa một packet trên
hầu hết mọi đường đi (kích thước packet tối thiểu của QUIC là 1200 cũng vì lý
do này). Trần cứng là 65507 byte (65535 trừ header IP và UDP). DNS answer lớn
vượt giới hạn là lý do DNS fallback sang TCP ([`14-dns.md`](14-dns.md)).

### UDP và middlebox có state
NAT và firewall theo dõi "flow" UDP bằng 4-tuple dù UDP không có flow, và cho
expire theo **timer** (thường 30–120 giây), vì không có FIN báo hiệu kết thúc.
Một QUIC connection hoặc UDP session idle lâu có thể mất NAT mapping và ngừng
hoạt động âm thầm, vì vậy protocol gửi keepalive định kỳ và QUIC hỗ trợ
**connection migration** bằng connection ID thay vì dựa vào 4-tuple
([`18-http3.md`](18-http3.md)).

### Proxy UDP và gotcha reflection attack
Source address của UDP giả mạo dễ như trở bàn tay vì không có handshake để
kiểm chứng. Kẻ tấn công gửi một query nhỏ với địa chỉ nạn nhân làm source tới
một server reply bằng thứ lớn hơn nhiều — **amplification** (open DNS
resolver, memcached trên UDP, NTP). Mọi UDP service bạn expose phải hoặc xác
minh source (QUIC làm vậy: server không được gửi quá 3x số byte đã nhận từ một
địa chỉ chưa được validate, và dùng retry token để validate — xem
[`18-http3.md`](18-http3.md) về bản thân QUIC) hoặc không bao giờ reply nhiều hơn
những gì nhận ([`07-security/09-ddos.md`](../07-security/09-ddos.md)). Load balancing UDP cũng cần
affinity: balancer phải pin một 4-tuple (hoặc QUIC connection ID) vào một
backend vì không có connection để neo.

## Practice

1. Khởi động UDP listener bằng `nc -u -l 9999` và gửi datagram bằng
   `echo -n hello | nc -u -w1 127.0.0.1 9999`; gửi hai cái liên tiếp và xác
   nhận mỗi cái đến thành một message riêng (đối chiếu với TCP, nơi `nc -l`
   có thể gộp chúng).
2. Viết một tokio UDP echo nhỏ (scratch project, không thuộc workspace) bằng
   đoạn code ở trên, thu nhỏ receive buffer còn 4 byte, gửi 10 byte, và quan
   sát việc bị cắt.
3. Chạy `sudo tcpdump -n -i lo udp port 9999` trong lúc làm mục 1 và xác định
   các field của UDP header 8 byte trong hex dump `-X` (`tcpdump -n -X`).
4. Gửi tới một UDP port không có listener trên một socket *đã connect* (scratch
   program: `UdpSocket::connect` rồi `send` hai lần) và quan sát
   `ECONNREFUSED` ở lần gọi thứ hai; gắn nó với ICMP message bạn bắt được bằng
   `tcpdump icmp`.
5. Dùng `tc qdisc add dev lo root netem loss 30%` (xem
   [`12-testing/03-chaos.md`](../12-testing/03-chaos.md)) và gửi 100 datagram có đánh số; đếm bao nhiêu
   cái đến nơi, có cái nào đảo thứ tự không (thêm `delay 20ms 10ms`), và giải
   thích vì sao bên gửi không biết được.
6. So sánh hình dạng của [`labs/00-tcp-server`](../../labs/00-tcp-server) (một vòng lặp `accept` spawn một
   task cho mỗi connection) với thứ một UDP service cần (một socket, một vòng
   `recv_from`, state theo từng peer trong một map keyed bằng source address),
   và ghi vào notes của bạn cái gì sẽ phải đổi — đây chính là bước nhảy cấu
   trúc mà [`labs/09-http3`](../../labs/09-http3) thực hiện.
