# TCP Reliability: Retransmission, Flow Control, Congestion Control

TCP biến một mạng packet hay mất dữ liệu thành một stream đáng tin cậy bằng cách nào?
File này cũng nói về các tham số có thể chỉnh và các kiểu lỗi mà người vận hành proxy
thực sự gặp. Kiến thức cần có trước: [`12-tcp.md`](12-tcp.md). Phần toán đằng sau
congestion control nằm ở
[`22-theory/05-congestion-control-math.md`](../22-theory/05-congestion-control-math.md).
File này chỉ nói về cơ chế và hệ quả khi vận hành.

## What to learn

### Ba vấn đề riêng, ba cơ chế riêng
TCP giải ba vấn đề độc lập nhau:

- **Loss** (một packet biến mất): gửi lại packet đó (retransmit).
- **Bên nhận chậm hơn bên gửi**: flow control, dùng receive window.
- **Mạng chậm hơn bên gửi**: congestion control, dùng congestion window.

Bên gửi được phép có tối đa `min(receive window, congestion window)` byte đang bay mà
chưa được ack. Nhầm hai window này với nhau là hiểu lầm phổ biến nhất về TCP. Hãy giữ
chúng tách bạch.

### Retransmission: timer và duplicate ACK
Bên gửi giữ lại mọi segment chưa được ack. Có hai điều kiện khiến nó gửi lại:

- **Retransmission timeout (RTO).** Mỗi lần nhận ACK, bên gửi đo round-trip time (RTT
  trung bình có làm mượt, cộng độ lệch), rồi đặt RTO cao hơn một chút. Linux có mức
  sàn 200 ms. Nếu ACK không tới kịp, bên gửi gửi lại và **nhân đôi** RTO (exponential
  backoff). Nó thử tối đa khoảng 15 lần (`tcp_retries2`, cỡ 15 phút) rồi mới làm
  connection fail với `ETIMEDOUT`. Nếu packet bị mất mà phía sau không còn packet nào
  khác (ví dụ đuôi của một lần truyền), bên gửi có thể phải chờ nguyên một RTO, tức
  hàng trăm mili giây không có gì xảy ra.
- **Fast retransmit.** Giả sử segment N bị mất nhưng N+1, N+2, N+3 vẫn tới nơi. Bên
  nhận cứ ACK lặp lại "tôi vẫn cần N". Đây là **duplicate ACK**. Khi gặp ba duplicate
  ACK, bên gửi hiểu rằng N đã mất chứ không chỉ chậm, và gửi lại ngay mà không chờ
  timer.

**SACK** (selective acknowledgment, được thương lượng trong SYN) cho phép bên nhận báo:
"tôi đã có byte 5000–8000 và 9000–12000, chỗ trống là 8000–9000". Nhờ vậy bên gửi chỉ
gửi lại đúng chỗ trống, không gửi lại mọi thứ phía sau nó. Trên Linux, SACK và
**timestamp** (để đo RTT chính xác và chống packet trùng cũ) được bật mặc định.

```text
$ ss -ti dst 10.0.0.7
... rto:204 rtt:3.2/1.1 mss:1448 cwnd:10 ssthresh:7 bytes_retrans:2896 retrans:0/2
```

`retrans` và `bytes_retrans` là chỉ báo loss đầu tiên bạn có, theo từng connection, mà
không cần bắt packet.

### Head-of-line blocking
TCP giao byte **đúng thứ tự**. Nếu segment N bị mất, các byte sau N dù đã tới vẫn nằm
trong receive buffer của kernel và **bị giữ lại, chưa đưa cho application**, cho đến
khi N được gửi lại. Mọi logical stream được multiplex trên connection đó đều bị đứng
theo.

HTTP/2 thừa hưởng vấn đề này (nhiều stream, chung một TCP connection,
[`17-http2.md`](17-http2.md)). Đó cũng là lý do HTTP/3 chuyển sang QUIC, nơi mỗi stream
tự hồi phục độc lập ([`18-http3.md`](18-http3.md)).

### Flow control: receive window
Mỗi ACK mang theo một **window**: bên nhận còn chỗ trong buffer cho bao nhiêu byte nữa.
Khi application đọc không đủ nhanh, buffer trong kernel của bên nhận đầy dần. Window mà
nó quảng bá thu nhỏ lại. Khi về **zero window**, bên gửi dừng hẳn và gửi định kỳ các
**zero-window probe** để hỏi xem đã có chỗ chưa.

Đây là **backpressure** đầu-cuối. Với proxy, một client chậm chỉ làm chậm việc đọc từ
*upstream* nếu code của bạn ngừng đọc upstream khi việc write về phía client bị block
([`04-runtime/`](../04-runtime)). Nếu thay vào đó bạn buffer vô hạn, áp lực dồn vào bộ
nhớ của process. Đó là cách kinh điển để một client chậm làm sập proxy.

Field window dài 16 bit, nên bị giới hạn ở 64 KB. Mức này quá nhỏ cho link nhanh và
dài. **Window scaling** (thương lượng trong SYN, dịch tối đa 14 bit) nâng trần lên
khoảng 1 GB.

**Bandwidth-delay product** (BDP, [`04-latency-throughput.md`](04-latency-throughput.md))
cho biết window phải lớn cỡ nào để lấp đầy đường truyền. Ví dụ: link 1 Gbit/s với RTT
100 ms cần khoảng 12,5 MB dữ liệu đang bay. Linux tự điều chỉnh (autotune) buffer, tối
đa đến giá trị của `tcp_rmem`/`tcp_wmem`. Nếu bạn tự đặt `SO_RCVBUF`/`SO_SNDBUF` thì
autotuning của socket đó bị *tắt*, và thường đó là sai lầm.

### Congestion control: dò xem mạng chở được bao nhiêu
Bên gửi duy trì một **congestion window** (`cwnd`, đơn vị là segment) và dò dung lượng
của mạng bằng thực nghiệm:

1. **Slow start**: bắt đầu từ initial window (10 segment trên Linux hiện đại, khoảng
   14 KB) và **nhân đôi `cwnd` mỗi RTT** (mỗi ACK nhận được thì thêm một segment). Cứ
   thế cho đến khi mất packet hoặc `cwnd` chạm `ssthresh`.
2. **Congestion avoidance**: vượt qua `ssthresh`, `cwnd` chỉ tăng khoảng 1 segment mỗi
   RTT.
3. **Khi mất packet**, `cwnd` bị cắt. Reno cổ điển giảm một nửa. Cách làm này gọi là
   **AIMD** (additive increase, multiplicative decrease). Nó khiến các flow cạnh tranh
   nhau dần hội tụ về phần chia công bằng.

Linux mặc định dùng **Cubic**. Tốc độ tăng của nó là hàm bậc ba theo thời gian kể từ
lần mất packet gần nhất, và hoạt động tốt hơn trên link có BDP cao. **BBR** làm khác:
nó *mô hình hóa* bandwidth và RTT, và không dùng loss làm tín hiệu chính. BBR làm tốt
hơn trên đường hay mất packet hoặc bị bufferbloat. Dùng
`sysctl net.ipv4.tcp_congestion_control` để xem và đặt thuật toán.

Vì sao proxy cần quan tâm:

- **Connection mới bắt đầu rất nhỏ.** Một connection upstream mới chỉ gửi được khoảng
  14 KB trong RTT đầu tiên, dù link nhanh đến đâu. Một response lớn cần vài RTT để
  tăng tốc. Đây là lý do về throughput để reuse connection, bên cạnh lý do về latency
  ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)).
- **Slow-start after idle.** Mặc định, Linux thu nhỏ `cwnd` lại sau một khoảng idle
  (`net.ipv4.tcp_slow_start_after_idle=1`). Một pooled connection nằm yên vài giây sẽ
  *phải tăng tốc lại từ đầu*. Proxy có throughput cao thường tắt tùy chọn này.
- **Bufferbloat.** Queue quá lớn trong router có thể giữ hàng giây dữ liệu. Các thuật
  toán dựa trên loss làm đầy những queue đó, và cộng thêm latency rất lớn mà không có
  loss nào. Dấu hiệu nhận biết là: khi tải cao, RTT tăng vọt trong lúc throughput đi
  ngang.

### Nagle's algorithm, delayed ACK và TCP_NODELAY
Đây là hai tối ưu hóa đều hợp lý, nhưng khi kết hợp lại thì gây rắc rối.

- **Nagle**: nếu còn dữ liệu chưa được ack đang bay, giữ các write nhỏ lại và gộp
  chúng, cho đến khi ACK tới hoặc gom đủ một segment. Mục đích là giảm số packet nhỏ.
- **Delayed ACK**: bên nhận chờ (tối đa khoảng 40 ms) với hy vọng ACK có thể đi kèm một
  reply, hoặc ACK được hai segment cùng lúc.

Kết hợp hai thứ này: client write một request thành hai mảnh nhỏ. Mảnh thứ hai phải
chờ ACK của mảnh thứ nhất. Nhưng server đang trì hoãn ACK đó vì còn chờ nốt phần còn
lại của request. Kết quả là mỗi request bị đứng khoảng 40 ms.

`TCP_NODELAY` tắt Nagle. Proxy và RPC server thường bật nó, vì chúng đã tự đóng gói
(framing) các write. Công cụ ngược lại là `TCP_CORK`/`MSG_MORE`, nghĩa là "giữ lại cho
đến khi tôi bảo gửi". Chúng dùng để ghép header và body của `sendfile` thành packet đầy
([`02-linux/18-zerocopy.md`](../02-linux/18-zerocopy.md)). Cách sửa tốt hơn vẫn là dùng
*một* `write` (hoặc một vectored write) cho mỗi message, thay vì nhiều write nhỏ.

```rust
stream.set_nodelay(true)?;
```

### Gotcha: nghĩa địa sysctl
Trên internet có rất nhiều danh sách sysctl "TCP tuning" được copy từ server này sang
server khác. Hầu hết đã lỗi thời hoặc có hại: `tcp_tw_recycle`, `tcp_sack=0`, hay đặt
`tcp_rmem` max rất lớn trên máy có hàng nghìn connection (bộ nhớ mỗi connection nhân
lên theo số connection).

Chỉ tune khi có số đo:
- `ss -ti` cho `retrans`/`cwnd`/`rtt`.
- `nstat -az | grep -i retrans` cho counter toàn hệ thống.
- Một load test trước và sau khi chỉnh
  ([`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)).

Cơ chế bên trong kernel (autotuning, queue) nằm ở
[`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md).

## Practice

1. Thêm delay vào loopback: `sudo tc qdisc add dev lo root netem delay 50ms` (50 ms mỗi
   chiều, tức RTT khoảng 100 ms). Tải một response 1 MB từ một server local bằng
   `curl -sS -o /dev/null -w '%{time_total}\n'` và ghi lại thời gian. Trong lúc truyền,
   dùng `ss -ti` để xem `cwnd` tăng. Gỡ delay bằng `sudo tc qdisc del dev lo root`.
2. Thêm loss: `sudo tc qdisc add dev lo root netem loss 2%`. Tải lại như trên. Quan sát
   `retrans` trong `ss -ti`, và tìm một fast retransmit trong `tcpdump -n -S` (cùng một
   `seq` được gửi hai lần sau ba duplicate ACK). So sánh thời gian truyền với trường hợp
   0% loss.
3. Đọc `wscale` và `sackOK` từ một gói SYN trong `tcpdump -n`. Tính window tối đa mà nó
   cho phép (65535 << wscale). Tính BDP của đường 1 Gbit/s, 50 ms, rồi so với mức tối
   đa của `net.ipv4.tcp_rmem`.
4. Tái hiện stall do Nagle + delayed ACK. Viết một client scratch gửi `"GET /"` và
   `" HTTP/1.1\r\n\r\n"` bằng hai lần `write` riêng, tới một server chỉ reply sau khi
   nhận đủ request. Chạy hai lần, có và không có `set_nodelay(true)`, và đo round trip.
   Sau đó sửa theo cách thứ hai: ghép request thành một lần write duy nhất.
5. Tạo backpressure có chủ đích. Cho một client connect tới
   [`labs/00-tcp-server`](../../labs/00-tcp-server) và gửi liên tục, trong khi handler
   phía server sleep trước mỗi lần read. Cho thấy window rơi về 0 trong `tcpdump -n`
   (`win 0`) và `write` của client bị block. Sau đó xác nhận bộ nhớ của server không
   tăng.
6. So sánh Cubic với BBR (`sudo sysctl -w net.ipv4.tcp_congestion_control=bbr`, nếu
   `modprobe tcp_bbr` chạy được trên kernel của bạn) trên loopback `netem` có delay và
   loss. Ghi lại throughput của từng thuật toán và giải thích sự khác biệt dựa trên tín
   hiệu congestion mà mỗi thuật toán sử dụng.
