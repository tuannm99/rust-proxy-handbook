# Đọc mạng: tcpdump, ss, ip, curl, dig, openssl

Phần Practice của mọi file sau đều bảo bạn "bắt packet" hoặc "kiểm tra bằng `ss`". File
này dạy cách *đọc* output của các tool đó, để một packet trace trở thành bằng chứng
thay vì một đống nhiễu. Ghi chú cài đặt cho những tool ít phổ biến nằm ở
[`12-testing/06-lab-environment.md`](../12-testing/06-lab-environment.md). Phương pháp
dùng chúng khi có gì đó hỏng nằm ở
[`12-testing/05-debugging.md`](../12-testing/05-debugging.md).

## What to learn

### tcpdump: bắt và lọc
`tcpdump` ghi lại các packet mà một interface nhìn thấy. Nó cần quyền root (hoặc
`CAP_NET_RAW`). Các flag đáng nhớ:

```text
sudo tcpdump -i lo -n -nn port 8080          # -n: không DNS lookup, -nn: không đổi port thành tên
sudo tcpdump -i any -n host 10.0.0.7 and tcp # biểu thức filter BPF
sudo tcpdump -i lo -n -X -s0 port 8080       # -X: payload hex+ASCII, -s0: lấy cả packet
sudo tcpdump -i lo -n -w cap.pcap port 8080  # lưu cho Wireshark; dùng -r cap.pcap để đọc lại
sudo tcpdump -i any -n 'tcp[tcpflags] & (tcp-syn|tcp-fin|tcp-rst) != 0'
```

Luôn dùng `-n`. Nếu không, tcpdump bị chặn lại để làm reverse DNS và làm sai lệch thứ
bạn đang đo. Trên một host bận, luôn có filter. Dùng `-i lo` để test local, `-i any`
để thấy mọi interface (nhưng sẽ mất link-layer header).

Ngôn ngữ filter là **BPF**, chính là nền tảng mà eBPF phát triển lên từ đó
([`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md)). Việc bắt packet xảy ra trong
kernel, trước khi application thấy dữ liệu và sau khi nó đã gửi. Vì vậy tcpdump cho
thấy thứ *thực sự có trên dây*, kể cả những packet mà code của bạn không hề biết.

### Đọc một dòng TCP
```text
12:00:01.000100 IP 127.0.0.1.52000 > 127.0.0.1.8080: Flags [S], seq 1000, win 65495, options [mss 65495,sackOK,TS val 1 ecr 0,nop,wscale 7], length 0
12:00:01.000120 IP 127.0.0.1.8080 > 127.0.0.1.52000: Flags [S.], seq 2000, ack 1001, win 65483, ..., length 0
12:00:01.000130 IP 127.0.0.1.52000 > 127.0.0.1.8080: Flags [.], ack 2001, win 512, length 0
12:00:01.000300 IP 127.0.0.1.52000 > 127.0.0.1.8080: Flags [P.], seq 1:79, ack 1, win 512, length 78
```

Cách đọc:

- `src.port > dst.port` là hướng đi của packet.
- **Flags**: `S` = SYN, `.` = chỉ ACK, `S.` = SYN+ACK, `P.` = PSH+ACK (có data), `F.` =
  FIN+ACK, `R` = RST.
- `seq 1:79` nghĩa là byte 1 đến byte 78 của stream. Sau handshake, tcpdump in sequence
  number **tương đối**. Dùng `-S` để in giá trị tuyệt đối.
- `ack` là byte kế tiếp mà bên gửi mong đợi.
- `length` là số byte payload.
- `win` là receive window mà bên gửi quảng bá.
- `options` hiện MSS, SACK, window scaling, timestamp. Tất cả đều được thương lượng
  trong SYN ([`13-tcp-reliability.md`](13-tcp-reliability.md)).

Với vốn từ này, bạn có thể đọc lướt một handshake, một request, một retransmission (cùng
một `seq` xuất hiện hai lần) và một reset.

### Wireshark và tshark
Wireshark mở một file `.pcap`, giải mã mọi layer thành một cây, và có thể **follow a TCP
stream** (ghép hai chiều lại thành các byte của application). `tshark` là cùng engine đó
trên command line: `tshark -r cap.pcap -Y 'http.request' -T fields -e http.host`.

Workflow thường dùng: bắt packet trên server bằng tcpdump, rồi phân tích trên laptop
bằng Wireshark.

Payload TLS không đọc được, trừ khi bạn đưa session key cho Wireshark. Đặt
`SSLKEYLOGFILE=keys.log` cho client có hỗ trợ (curl và trình duyệt có, rustls có helper
`KeyLogFile`), rồi trỏ TLS preferences của Wireshark vào file đó
([`19-tls.md`](19-tls.md)).

### ss: bảng socket của kernel
`ss` (thay thế `netstat`) in thông tin socket thẳng từ kernel.

```text
ss -tlnp            # TCP, Listening, Numeric, Process: ai đang listen ở đâu
ss -tn state established '( dport = :8080 or sport = :8080 )'
ss -tn state time-wait | wc -l
ss -tni             # -i: thông tin nội bộ của từng socket (rtt, cwnd, retrans, mss)
ss -s               # tổng hợp số lượng theo state
```

Với một listening socket:
- `Recv-Q` là **độ dài hiện tại của accept queue**.
- `Send-Q` là giá trị tối đa của nó (backlog).

Đây là con số cần theo dõi khi connection bị drop ([`11-socket.md`](11-socket.md)).

Với một established socket:
- `Recv-Q` là số byte kernel đã nhận nhưng process của bạn *chưa đọc*.
- `Send-Q` là số byte đã gửi nhưng chưa được ack.

`Recv-Q` tăng liên tục nghĩa là app đọc quá chậm (backpressure,
[`12-tcp.md`](12-tcp.md)). `Send-Q` tăng nghĩa là peer hoặc mạng đang chậm. `ss -tni`
còn hiện `rtt`, `cwnd`, `retrans` và `bytes_acked`, tức là TCP state mà kernel giữ giúp
bạn.

### ip và nc: địa chỉ, route, và một client làm bằng tay
`ip addr`, `ip route`, `ip neigh`, `ip -s link` (counter) bao quát interface, routing và
ARP ([`07-link-layer.md`](07-link-layer.md), [`08-ip-and-icmp.md`](08-ip-and-icmp.md)).
`ip route get <dst>` hỏi kernel xem nó sẽ chọn route nào và source address nào.

`nc` (netcat) mở một connection TCP hoặc UDP thô và cho bạn tự gõ byte:

- `printf 'GET / HTTP/1.1\r\nHost: x\r\n\r\n' | nc 127.0.0.1 8080` gửi một request
  bằng tay.
- `nc -l 9999` mở một listener.
- `nc -z host 1-1024` quét port.

`nc` giúp bạn chứng minh một bug nằm ở các byte của protocol, chứ không phải do curl đã
"xử lý hộ" giúp bạn.

### curl -v và --resolve
`curl -v` in toàn bộ cuộc trao đổi: kết quả DNS, TCP connect, TLS handshake (version,
cipher, certificate), request header (dòng `>`), response header (dòng `<`). Vài biến
thể hữu ích:

```text
curl -sS -o /dev/null -w 'dns=%{time_namelookup} tcp=%{time_connect} tls=%{time_appconnect} ttfb=%{time_starttransfer} total=%{time_total}\n' https://example.com/
curl --resolve example.com:443:127.0.0.1 https://example.com/   # bỏ qua DNS, vẫn giữ SNI/Host
curl --http1.1 / --http2 / --http3 URL      # chọn protocol
curl --path-as-is 'http://h/a/../b'         # không normalize path
curl -H 'Transfer-Encoding: chunked' -d @file URL
```

Dòng timing của `-w` cho bạn bảng phân rã latency trong
[`04-latency-throughput.md`](04-latency-throughput.md) chỉ với một lệnh. Mỗi giá trị là
thời gian cộng dồn từ đầu, nên:
- `tcp - dns` là chi phí của TCP handshake.
- `tls - tcp` là chi phí của TLS handshake.
- `ttfb - tls` là thời gian server xử lý ("suy nghĩ").

### dig và openssl s_client
```text
dig example.com A +noall +answer            # record và TTL
dig @1.1.1.1 example.com AAAA               # hỏi một resolver cụ thể
dig +trace example.com                      # đi từ root -> TLD -> authoritative
openssl s_client -connect host:443 -servername host -alpn h2 </dev/null
openssl s_client -connect host:443 -showcerts   # in cả chain
```

`dig` nói DNS trực tiếp, bỏ qua cấu hình resolver của OS. Vì vậy nếu `dig` và chương
trình của bạn cho kết quả khác nhau, nguyên nhân nằm ở `/etc/resolv.conf`, `nsswitch`
hoặc caching ([`14-dns.md`](14-dns.md)).

`openssl s_client` thực hiện một TLS client handshake và in ra version, cipher, ALPN đã
chọn, và certificate chain đã thương lượng. Cờ `-servername` đặt SNI. Nếu bỏ nó đi khi
gọi một server virtual-host, bạn sẽ nhận sai certificate (bug SNI mismatch,
[`19-tls.md`](19-tls.md)).

### Gotcha: bắt ở đâu quyết định bạn thấy gì
- Bắt ở phía **client**, bạn thấy packet đúng lúc gửi đi, kể cả những packet sau đó bị
  drop. Bắt ở **server**, bạn chỉ thấy những gì đã tới nơi.
- Traffic loopback bỏ qua hoàn toàn NIC: không có checksum thật, không có giới hạn MTU,
  không mất packet. Vì thế test local không tái hiện được vấn đề PMTUD hay offload.
- Với TCP segmentation offload, tcpdump có thể hiện một "packet" lớn hơn MTU, vì NIC mới
  là nơi cắt nhỏ, và nó làm việc đó sau điểm bắt.
- Bắt trên một proxy cho thấy hai cuộc hội thoại riêng biệt (phía client và phía
  upstream). Bạn phải tự đối chiếu chúng bằng timestamp và port. Đây chính là việc mà
  trace ID ([`08-observability/`](../08-observability)) làm tự động.

## Practice

1. Khởi động [`labs/00-tcp-server`](../../labs/00-tcp-server) và chạy
   `sudo tcpdump -i lo -n port <p>`. Ở terminal khác, chạy
   `printf 'hello\n' | nc 127.0.0.1 <p>`. Chú thích từng dòng của trace: dòng nào là
   SYN, SYN-ACK, ACK, data segment, ACK của nó, FIN, và ACK cuối.
2. Lưu trace đó bằng `-w`, mở trong Wireshark (hoặc `tshark -r`), và dùng "Follow TCP
   stream" để xem các byte của application. Sau đó chạy `tcpdump -r cap.pcap -n -X` và
   tìm đúng các byte đó trong hex dump.
3. Khi một phiên `nc` dài đang mở, chạy `ss -tni dst 127.0.0.1`. Xác định state, `rtt`,
   `cwnd` và kích thước queue của nó. Sau đó cho phía server ngừng đọc (một `sleep` trong
   handler) trong khi client vẫn gửi, và xem `Recv-Q`/`Send-Q` tăng lên.
4. Chạy lệnh timing `curl -w` ở trên vào một site HTTPS từ xa và vào một server HTTP
   thường chạy local. Giải thích từng field, field nào bằng 0 trong trường hợp local, và
   vì sao.
5. Dùng `dig +trace example.com` để đi theo chuỗi delegation. Sau đó so sánh
   `dig example.com` với `getent hosts example.com` (thứ mà hầu hết chương trình dùng),
   và giải thích mọi khác biệt về output hoặc latency.
6. Chạy `openssl s_client -connect example.com:443 -servername example.com -alpn h2 </dev/null`.
   Rồi chạy lại không có `-servername` vào một host multi-tenant, và so sánh các
   certificate trả về. Cuối cùng, đặt `SSLKEYLOGFILE=$PWD/keys.log`, chạy
   `curl https://example.com/` dưới `tcpdump -w`, và đọc HTTP đã giải mã trong
   Wireshark.

---

<!-- nav -->
← Trước: [`01-network/09-udp.md`](09-udp.md) · [Mục lục](00-README.md) · Tiếp theo: [`01-network/11-socket.md`](11-socket.md) →
