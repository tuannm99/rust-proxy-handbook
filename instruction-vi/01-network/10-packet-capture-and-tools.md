# Đọc mạng: tcpdump, ss, ip, curl, dig, openssl

Phần Practice của mọi file sau đều nói "bắt packet" hoặc "kiểm tra bằng `ss`".
File này dạy cách *đọc* output của các tool đó, để một packet trace trở thành
bằng chứng thay vì nhiễu. Ghi chú cài đặt cho các tool ít phổ biến nằm ở
[`12-testing/06-lab-environment.md`](../12-testing/06-lab-environment.md); phương pháp dùng chúng khi có gì đó hỏng là
[`12-testing/05-debugging.md`](../12-testing/05-debugging.md).

## What to learn

### tcpdump: bắt và lọc
`tcpdump` ghi lại các packet mà một interface thấy. Nó cần root (hoặc
`CAP_NET_RAW`). Các flag đáng thuộc:

```text
sudo tcpdump -i lo -n -nn port 8080          # -n: không DNS lookup, -nn: không tên port
sudo tcpdump -i any -n host 10.0.0.7 and tcp # biểu thức filter BPF
sudo tcpdump -i lo -n -X -s0 port 8080       # -X: payload hex+ASCII, -s0: cả packet
sudo tcpdump -i lo -n -w cap.pcap port 8080  # lưu cho Wireshark; -r cap.pcap để đọc
sudo tcpdump -i any -n 'tcp[tcpflags] & (tcp-syn|tcp-fin|tcp-rst) != 0'
```

Luôn dùng `-n` (nếu không tcpdump bị chặn lại để reverse DNS và làm sai lệch
thứ bạn đang đo), luôn có filter trên một host bận. `-i lo` cho test local,
`-i any` để thấy mọi interface (nhưng mất link-layer header). Ngôn ngữ filter
là **BPF**, cùng bộ máy mà eBPF phát triển lên từ đó
([`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md)). Việc bắt xảy ra trong kernel, trước khi
application thấy dữ liệu và sau khi nó đã gửi, nên nó cho thấy thứ *thực sự
trên dây* — gồm cả những packet mà code của bạn không hề biết.

### Đọc một dòng TCP
```text
12:00:01.000100 IP 127.0.0.1.52000 > 127.0.0.1.8080: Flags [S], seq 1000, win 65495, options [mss 65495,sackOK,TS val 1 ecr 0,nop,wscale 7], length 0
12:00:01.000120 IP 127.0.0.1.8080 > 127.0.0.1.52000: Flags [S.], seq 2000, ack 1001, win 65483, ..., length 0
12:00:01.000130 IP 127.0.0.1.52000 > 127.0.0.1.8080: Flags [.], ack 2001, win 512, length 0
12:00:01.000300 IP 127.0.0.1.52000 > 127.0.0.1.8080: Flags [P.], seq 1:79, ack 1, win 512, length 78
```

Giải mã: `src.port > dst.port`; **Flags** `S` = SYN, `.` = chỉ ACK, `S.` =
SYN+ACK, `P.` = PSH+ACK (data), `F.` = FIN+ACK, `R` = RST. `seq 1:79` = byte
1 đến 78 của stream (tcpdump in sequence number **tương đối** sau handshake;
`-S` hiện giá trị tuyệt đối). `ack` = byte kế tiếp mà bên gửi mong đợi.
`length` = số byte payload. `win` = receive window được quảng bá. `options`
hiện MSS, SACK, window scaling, timestamp — tất cả được thương lượng trong SYN
([`13-tcp-reliability.md`](13-tcp-reliability.md)). Có vốn từ này bạn có thể đọc lướt một handshake,
một request, một retransmission (cùng `seq` xuất hiện hai lần) và một reset.

### Wireshark và tshark
Wireshark mở một file `.pcap`, giải mã mọi layer thành một cây, và có thể
**follow a TCP stream** (ghép hai chiều lại thành các byte ứng dụng). `tshark`
là cùng engine trên command line:
`tshark -r cap.pcap -Y 'http.request' -T fields -e http.host`. Bắt trên server
bằng tcpdump, phân tích trên laptop bằng Wireshark — workflow thường dùng.
Payload TLS là mờ đục trừ khi bạn đưa session key cho Wireshark: set
`SSLKEYLOGFILE=keys.log` cho client có hỗ trợ (curl và trình duyệt có, rustls
có helper `KeyLogFile`), rồi trỏ TLS preferences của Wireshark vào file đó
([`19-tls.md`](19-tls.md)).

### ss: bảng socket của kernel
`ss` (thay `netstat`) in socket thẳng từ kernel.

```text
ss -tlnp            # TCP, Listening, Numeric, Process: ai listen ở đâu
ss -tn state established '( dport = :8080 or sport = :8080 )'
ss -tn state time-wait | wc -l
ss -tni             # -i: nội bộ từng socket (rtt, cwnd, retrans, mss)
ss -s               # tổng hợp số lượng theo state
```

Với một listening socket, `Recv-Q` là **độ dài accept queue** hiện tại và
`Send-Q` là giá trị tối đa của nó (backlog) — con số cần theo dõi khi
connection bị drop ([`11-socket.md`](11-socket.md)). Với một established socket, `Recv-Q` là số
byte kernel đã nhận mà process của bạn *chưa đọc* và `Send-Q` là số byte đã
gửi nhưng chưa được ack. `Recv-Q` tăng liên tục nghĩa là app đọc chậm
(backpressure, [`12-tcp.md`](12-tcp.md)); `Send-Q` tăng nghĩa là peer hoặc mạng chậm.
`ss -tni` lộ `rtt`, `cwnd`, `retrans` và `bytes_acked`, state TCP mà kernel
giữ giùm bạn.

### ip và nc: địa chỉ, route, và một client làm tay
`ip addr`, `ip route`, `ip neigh`, `ip -s link` (counter) bao quát interface,
routing và ARP ([`07-link-layer.md`](07-link-layer.md), [`08-ip-and-icmp.md`](08-ip-and-icmp.md)). `ip route get <dst>`
hỏi kernel nó sẽ chọn route và source address nào. `nc` (netcat) mở một kết
nối TCP hoặc UDP thô và cho bạn gõ byte —
`printf 'GET / HTTP/1.1\r\nHost: x\r\n\r\n' | nc 127.0.0.1 8080` gửi một
request làm tay; `nc -l 9999` listen; `nc -z host 1-1024` quét port. `nc` là
cách chứng minh một bug nằm ở byte của protocol chứ không phải ở sự "tử tế"
của curl.

### curl -v và --resolve
`curl -v` in toàn bộ cuộc trao đổi: kết quả DNS, TCP connect, TLS handshake
(version, cipher, certificate), request header (`>`), response header (`<`).
Vài biến thể hữu ích:

```text
curl -sS -o /dev/null -w 'dns=%{time_namelookup} tcp=%{time_connect} tls=%{time_appconnect} ttfb=%{time_starttransfer} total=%{time_total}\n' https://example.com/
curl --resolve example.com:443:127.0.0.1 https://example.com/   # bỏ qua DNS, giữ SNI/Host
curl --http1.1 / --http2 / --http3 URL      # chọn protocol
curl --path-as-is 'http://h/a/../b'         # không normalize path
curl -H 'Transfer-Encoding: chunked' -d @file URL
```

Dòng timing `-w` cho bạn bảng phân rã latency của
[`04-latency-throughput.md`](04-latency-throughput.md) chỉ bằng một lệnh: mỗi field là cộng dồn, nên
`tcp - dns` là chi phí TCP handshake, `tls - tcp` là TLS handshake,
`ttfb - tls` là thời gian server "suy nghĩ".

### dig và openssl s_client
```text
dig example.com A +noall +answer            # record và TTL
dig @1.1.1.1 example.com AAAA               # hỏi một resolver cụ thể
dig +trace example.com                      # đi từ root -> TLD -> authoritative
openssl s_client -connect host:443 -servername host -alpn h2 </dev/null
openssl s_client -connect host:443 -showcerts   # in chain
```

`dig` nói DNS trực tiếp, bỏ qua cấu hình resolver của OS, nên khác biệt giữa
`dig` và chương trình của bạn chỉ về `/etc/resolv.conf`, `nsswitch`, hoặc
caching ([`14-dns.md`](14-dns.md)). `openssl s_client` thực hiện một TLS client handshake
và in ra version, cipher, ALPN đã chọn và certificate chain được thương
lượng; `-servername` set SNI — bỏ nó đi khi gọi một server virtual-host và bạn
nhận sai certificate (bug SNI mismatch, [`19-tls.md`](19-tls.md)).

### Gotcha: bắt ở đâu quyết định bạn thấy gì
Bắt ở phía **client** bạn thấy packet như lúc gửi (gồm cả những cái sau đó bị
drop); bắt ở **server** bạn chỉ thấy cái đã tới. Traffic loopback bỏ qua hẳn
NIC — không checksum thật, không giới hạn MTU, không mất packet — nên test
local không thể tái hiện vấn đề PMTUD hay offload. Với TCP segmentation
offload, tcpdump có thể hiện "packet" lớn hơn MTU, vì NIC cắt sau điểm bắt.
Và bắt trên một proxy cho thấy hai cuộc hội thoại riêng biệt (phía client và
phía upstream) mà bạn phải tự đối chiếu bằng timestamp và port — đúng thứ mà
trace ID ([`08-observability/`](../08-observability)) tự động hóa.

## Practice

1. Khởi động [`labs/00-tcp-server`](../../labs/00-tcp-server) và chạy `sudo tcpdump -i lo -n port <p>`;
   ở terminal khác `printf 'hello\n' | nc 127.0.0.1 <p>`. Chú thích từng dòng
   của trace: dòng nào là SYN, SYN-ACK, ACK, data segment, ACK của nó, FIN, và
   ACK cuối.
2. Lưu trace đó bằng `-w`, mở trong Wireshark (hoặc `tshark -r`), và dùng
   "Follow TCP stream" để xem các byte ứng dụng. Sau đó chạy
   `tcpdump -r cap.pcap -n -X` và tìm cùng các byte đó trong hex dump.
3. Khi một phiên `nc` dài đang mở, chạy `ss -tni dst 127.0.0.1` và xác định
   state, `rtt`, `cwnd`, và kích thước queue của nó; ngừng đọc ở phía server
   (một `sleep` trong handler) trong khi client vẫn gửi và xem
   `Recv-Q`/`Send-Q` tăng.
4. Chạy lệnh timing `curl -w` ở trên vào một site HTTPS từ xa và vào một
   server HTTP thường local; giải thích từng field, field nào bằng 0 ở trường
   hợp local, và vì sao.
5. Dùng `dig +trace example.com` để đi theo chuỗi delegation, rồi so sánh
   `dig example.com` với `getent hosts example.com` (thứ mà hầu hết chương
   trình dùng) và giải thích mọi khác biệt về output hoặc latency.
6. Chạy `openssl s_client -connect example.com:443 -servername example.com
   -alpn h2 </dev/null`, rồi chạy lại không có `-servername` vào một host
   multi-tenant, và so sánh các certificate trả về. Cuối cùng set
   `SSLKEYLOGFILE=$PWD/keys.log`, chạy `curl https://example.com/` dưới
   `tcpdump -w`, và đọc HTTP đã giải mã trong Wireshark.
