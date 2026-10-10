# TCP Reliability: Retransmission, Flow Control, Congestion Control

TCP biến một mạng packet hay mất dữ liệu thành một stream đáng tin cậy như thế
nào — cùng các nút vặn và failure mode mà người vận hành proxy thực sự gặp.
Kiến thức tiên quyết: [`12-tcp.md`](12-tcp.md). Phần toán đằng sau congestion control nằm ở
[`22-theory/05-congestion-control-math.md`](../22-theory/05-congestion-control-math.md); file này là cơ chế và hệ quả khi vận hành.

## What to learn

### Ba vấn đề riêng biệt, ba cơ chế riêng biệt
TCP giải độc lập: **loss** (một packet biến mất — retransmit nó), **bên nhận
chậm hơn bên gửi** (flow control: receive window) và **mạng chậm hơn bên gửi**
(congestion control: congestion window). Bên gửi được phép có tối đa
`min(receive window, congestion window)` byte chưa được ack đang bay. Nhầm lẫn
hai window này là hiểu lầm phổ biến nhất về TCP; hãy giữ chúng tách bạch.

### Retransmission: timer và duplicate ACK
Bên gửi giữ mọi segment chưa được ack. Hai trigger khiến nó gửi lại:

- **Retransmission timeout (RTO).** Bên gửi đo round-trip time ở mỗi ACK
  (smoothed RTT cộng độ lệch) và đặt RTO cao hơn một chút (sàn 200 ms trên
  Linux). Nếu không có ACK kịp thời nó gửi lại và **nhân đôi** RTO (exponential
  backoff), tối đa ~15 lần retry (`tcp_retries2`, cỡ 15 phút) rồi mới làm fail
  connection với `ETIMEDOUT`. Một packet mất mà phía sau không còn gì (đuôi của
  một lần truyền) có thể tốn nguyên một RTO — hàng trăm mili giây chết lặng.
- **Fast retransmit.** Nếu segment N mất nhưng N+1, N+2, N+3 tới, bên nhận cứ
  ACK "vẫn cần N" (**duplicate ACK**). Ba lần duplicate báo cho bên gửi rằng N
  đã mất, không chỉ là trễ, và nó gửi lại ngay mà không chờ timer.

**SACK** (selective acknowledgment, thương lượng trong SYN) cho bên nhận nói
"tôi có byte 5000–8000 và 9000–12000, lỗ hổng là 8000–9000," để bên gửi chỉ gửi
lại cái lỗ thay vì mọi thứ phía sau nó. Trên Linux, SACK và **timestamp** (cho
mẫu RTT chính xác và chống packet trùng cũ) được bật mặc định.

```text
$ ss -ti dst 10.0.0.7
... rto:204 rtt:3.2/1.1 mss:1448 cwnd:10 ssthresh:7 bytes_retrans:2896 retrans:0/2
```

`retrans` và `bytes_retrans` là chỉ báo loss đầu tiên của bạn, cho từng
connection, không cần bắt packet.

### Head-of-line blocking
TCP giao byte **đúng thứ tự**. Nếu segment N mất, các byte sau N đã tới nằm
trong receive buffer của kernel, **bị giữ lại không đưa cho application** cho
tới khi N được retransmit. Mọi logical stream được multiplex trên connection đó
cùng khựng lại. Đây là vấn đề HTTP/2 thừa hưởng (nhiều stream, một TCP
connection, [`17-http2.md`](17-http2.md)) và là lý do HTTP/3 chuyển sang QUIC, nơi mỗi stream
tự hồi phục độc lập ([`18-http3.md`](18-http3.md)).

### Flow control: receive window
Mỗi ACK mang **window**: bên nhận còn chỗ buffer cho bao nhiêu byte nữa. Kernel
buffer của bên nhận đầy lên khi application không `read` đủ nhanh; window được
quảng bá co lại; tới **zero window** thì bên gửi dừng hẳn và gửi định kỳ
**zero-window probe**. Đây là **backpressure** đầu-cuối: một client chậm chỉ làm
chậm việc đọc *upstream* của proxy nếu code của bạn ngừng đọc từ upstream khi
việc write phía client sẽ bị block ([`04-runtime/`](../04-runtime)). Nếu thay vào đó bạn
buffer vô hạn, áp lực bị hấp thụ trong bộ nhớ của process — cách kinh điển để
một client chậm làm sập proxy.

Field window 16-bit bị chặn ở 64 KB, quá nhỏ cho link nhanh và dài. **Window
scaling** (thương lượng trong SYN, dịch tối đa 14 bit) nâng trần lên ~1 GB.
**Bandwidth-delay product** ([`04-latency-throughput.md`](04-latency-throughput.md)) là window cần lớn cỡ nào để
lấp đầy một đường truyền: link 1 Gbit/s với RTT 100 ms cần ~12,5 MB đang bay.
Linux tự autotune buffer tới mức tối đa của `tcp_rmem`/`tcp_wmem`; set tường
minh `SO_RCVBUF`/`SO_SNDBUF` sẽ *tắt* autotuning cho socket đó, thường là sai
lầm.

### Congestion control: dò tìm dung lượng
Bên gửi duy trì một **congestion window** (`cwnd`, tính bằng segment), khám phá
dung lượng của mạng bằng thực nghiệm:

1. **Slow start**: bắt đầu từ initial window (10 segment trên Linux hiện đại,
   ~14 KB) và **nhân đôi `cwnd` mỗi RTT** (mỗi ACK thêm một segment) cho tới khi
   mất packet, hoặc tới `ssthresh`.
2. **Congestion avoidance**: qua `ssthresh`, tăng ~1 segment mỗi RTT.
3. **Khi mất packet**, cắt: Reno cổ điển giảm một nửa `cwnd`. Đây là **AIMD**
   (additive increase, multiplicative decrease), làm cho các flow cạnh tranh hội
   tụ về phần chia công bằng.

Linux mặc định dùng **Cubic** (tăng trưởng là hàm bậc ba của thời gian kể từ lần
mất gần nhất, tốt hơn trên link BDP cao); **BBR** thay vào đó *mô hình hóa*
bandwidth và RTT và không coi loss là tín hiệu chính, làm tốt hơn trên đường
hay mất packet hoặc bufferbloat. `sysctl net.ipv4.tcp_congestion_control` xem
và đặt nó.

Vì sao proxy cần quan tâm:

- **Connection mới bắt đầu nhỏ.** Một connection upstream mới chỉ gửi ~14 KB
  trong RTT đầu, dù link nhanh cỡ nào — một response lớn cần vài RTT để lên tốc.
  Đây là luận điểm của throughput cho việc tái sử dụng connection, bên cạnh luận
  điểm của latency ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)).
- **Slow-start after idle.** Mặc định Linux làm sụp `cwnd` sau một khoảng idle
  (`net.ipv4.tcp_slow_start_after_idle=1`), nên một pooled connection nằm yên vài
  giây sẽ *lên tốc lại từ đầu*; proxy throughput cao thường tắt nó.
- **Bufferbloat.** Queue quá lớn trong router giữ hàng giây dữ liệu; thuật toán
  dựa trên loss làm đầy chúng và thêm latency khổng lồ mà không hề có loss. RTT
  tăng vọt khi tải cao trong khi throughput phẳng là dấu hiệu nhận biết.

### Nagle's algorithm, delayed ACK, và TCP_NODELAY
Hai tối ưu hợp lý nhưng tương tác tệ. **Nagle**: nếu còn data chưa ack đang bay,
giữ các write nhỏ và gộp chúng lại cho tới khi ACK tới (hoặc gom đủ một
segment) — ít packet tí hon hơn. **Delayed ACK**: bên nhận chờ (tới ~40 ms) hy
vọng ACK có thể đi kèm một reply, hoặc ACK hai segment một lúc. Kết hợp lại:
client write một request thành hai mảnh nhỏ; mảnh thứ hai chờ ACK của mảnh thứ
nhất, mà server đang trì hoãn vì đang chờ phần còn lại của request. Kết quả: khựng
~40 ms ở mỗi request.

`TCP_NODELAY` tắt Nagle; proxy và RPC server set nó, vì chúng đã tự framing các
write. Công cụ ngược lại là `TCP_CORK`/`MSG_MORE` — "giữ lại cho tới khi tôi bảo"
— để ghép header cộng body `sendfile` thành packet đầy
([`02-linux/18-zerocopy.md`](../02-linux/18-zerocopy.md)). Cách sửa tốt hơn vẫn là *một* `write` (hoặc một
vectored write) cho mỗi message thay vì nhiều write nhỏ.

```rust
stream.set_nodelay(true)?;
```

### Gotcha: nghĩa địa sysctl
Internet đầy các danh sách sysctl "TCP tuning" được copy qua lại giữa các
server. Hầu hết đã lỗi thời hoặc có hại (`tcp_tw_recycle`, `tcp_sack=0`, `tcp_rmem`
max khổng lồ trên máy có hàng nghìn connection — bộ nhớ mỗi connection nhân lên).
Chỉ tune dựa trên số đo: `ss -ti` cho `retrans`/`cwnd`/`rtt`,
`nstat -az | grep -i retrans` cho counter toàn hệ thống, một load test
trước/sau ([`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)). Cơ chế trong kernel (autotuning,
queue) nằm ở [`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md).

## Practice

1. Thêm delay vào loopback: `sudo tc qdisc add dev lo root netem delay 50ms`
   (50 ms mỗi chiều, tức ~100 ms RTT). Tải một response 1 MB từ một server local
   bằng `curl -sS -o /dev/null -w '%{time_total}\n'` và ghi lại thời gian; dùng
   `ss -ti` trong lúc truyền để xem `cwnd` tăng. Gỡ bằng
   `sudo tc qdisc del dev lo root`.
2. Thêm loss: `sudo tc qdisc add dev lo root netem loss 2%`. Lặp lại việc tải;
   quan sát `retrans` trong `ss -ti`, và tìm một fast retransmit trong
   `tcpdump -n -S` (cùng `seq` được gửi hai lần sau ba duplicate ACK). So sánh
   thời gian truyền với 0% loss.
3. Đọc `wscale` và `sackOK` từ một SYN trong `tcpdump -n`, tính window tối đa
   nó cho phép (65535 << wscale), rồi tính BDP của đường 1 Gbit/s, 50 ms và so
   với mức tối đa của `net.ipv4.tcp_rmem`.
4. Tái hiện cú khựng Nagle/delayed-ACK: viết một client scratch gửi `"GET /"` và
   `" HTTP/1.1\r\n\r\n"` thành hai `write` riêng tới một server chỉ reply sau
   khi có request đầy đủ, có và không có `set_nodelay(true)`, và đo round trip;
   sửa theo cách thứ hai bằng cách ghép request thành một write duy nhất.
5. Tạo backpressure có chủ đích: làm một client kết nối tới
   [`labs/00-tcp-server`](../../labs/00-tcp-server) và gửi liên tục, trong khi handler phía server
   sleep trước mỗi lần read; cho thấy window rơi về 0 trong `tcpdump -n`
   (`win 0`) và `write` của client bị block, rồi xác nhận server giữ bộ nhớ
   không đổi.
6. So sánh Cubic với BBR (`sudo sysctl -w net.ipv4.tcp_congestion_control=bbr`
   nếu `modprobe tcp_bbr` chạy được trên kernel của bạn) trên một loopback
   `netem` có delay và loss; ghi lại throughput của mỗi cái và giải thích sự khác
   biệt theo thứ mà mỗi thuật toán dùng làm tín hiệu congestion.
