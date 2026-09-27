# Congestion Control: Phần toán đằng sau tham số

## What to learn

### Congestion window như biến state thật
Congestion window (`cwnd`) của TCP chặn số byte chưa được ack có thể đang
bay; throughput xấp xỉ `cwnd / RTT`. Mỗi giải thuật congestion control,
về mặt cơ chế, là một quy tắc tăng và giảm `cwnd` phản ứng với các tín
hiệu — loss, delay, explicit congestion notification. Phần xử lý thực
dụng của [`01-network/08-tcp.md`](../01-network/08-tcp.md) gọi tên cái này; file này suy ra phần
toán tăng/giảm thật.

### Slow start: theo cấp số, có chủ đích
`cwnd` bắt đầu ở một giá trị nhỏ (trước đây 1-2 segment; RFC 6928 nâng
mặc định thường gặp lên khoảng 10) và gấp đôi mỗi RTT tới khi loss đầu
tiên hoặc chạm một threshold — tăng theo cấp số, vì tăng tuyến tính từ một
window khởi đầu nhỏ sẽ tốn quá nhiều round trip để đạt capacity thật của
một đường truyền.

```text
RTT 1: cwnd = 10 segment
RTT 2: cwnd = 20
RTT 3: cwnd = 40
... tiếp tục gấp đôi tới khi loss hoặc ssthresh
```
Cú tăng theo cấp số này là lý do cụ thể, định lượng được, vì sao một kết
nối TCP hoàn toàn mới chậm hơn một kết nối được tái sử dụng dù mạng
không hề tắc nghẽn — luận điểm tái sử dụng kết nối của
[`01-network/08-tcp.md`](../01-network/08-tcp.md) có hình dạng chính xác ở đây.

### Congestion avoidance: AIMD (additive increase, multiplicative decrease)
Sau slow start, `cwnd` tăng khoảng một segment mỗi RTT (additive
increase) và giảm một nửa khi có loss (multiplicative decrease) — AIMD
kiểu Reno kinh điển. AIMD chứng minh được là công bằng giữa các flow cạnh
tranh ở trạng thái ổn định (nó converge về phần chia bằng nhau của
bandwidth bottleneck), chính xác là lý do nó được chọn hơn các lựa chọn
converge nhanh hơn nhưng kém công bằng hơn.

```text
additive increase:      cwnd += 1 segment mỗi RTT (không loss)
multiplicative decrease: cwnd = cwnd / 2 (khi loss)
```

### Công thức throughput, và nó nói gì về long fat network
Với AIMD dựa trên loss, throughput ở trạng thái ổn định xấp xỉ tỷ lệ với
`MSS / (RTT * sqrt(loss_rate))` — "công thức căn bậc hai." Hệ quả trực
tiếp, khó chịu: trên một đường truyền RTT cao, loss thấp, throughput tỷ
lệ với `1/RTT`, nên cùng một tỷ lệ loss không đáng kể trên một kết nối
nội bộ datacenter có thể chặn cứng throughput trên một kết nối xuyên lục
địa — một lý do định lượng vì sao latency upstream-tới-origin của một
proxy quan trọng cho throughput, không chỉ cho tail latency.

### Vì sao Cubic và BBR tồn tại: hai phê bình khác nhau về toán của AIMD
**Cubic** (mặc định của Linux) tăng `cwnd` như một hàm bậc ba theo thời
gian từ lần loss cuối thay vì tuyến tính, dành nhiều thời gian gần
ceiling trước đó hơn trước khi dò lên cao hơn — phù hợp hơn với mạng
bandwidth cao, RTT cao ("long fat") nơi cú leo additive chậm của AIMD
lãng phí capacity. **BBR** từ bỏ hoàn toàn loss như tín hiệu chính và
thay vào đó mô hình hóa trực tiếp bandwidth bottleneck thật và RTT tối
thiểu của đường truyền, phản ứng với delay đo được thay vì chờ một router
drop một packet — phản ứng trực tiếp với quan sát rằng control dựa trên
loss lấp đầy buffer (bufferbloat) rất lâu trước khi nó báo hiệu congestion.

### Vì sao một proxy không tự implement cái này — nhưng vẫn nên biết nó
Đây hoàn toàn là việc của TCP stack của kernel ([`01-network/08-tcp.md`](../01-network/08-tcp.md) đã
nói vậy); giá trị của phần toán ở đây là khả năng suy luận định lượng về
một câu hỏi production thật — "vì sao throughput tới vùng này giảm mà
không có gì phía chúng ta thay đổi" — thay vì chỉ suy luận định tính.

## Practice
1. Tính tay `cwnd` sau mỗi 6 RTT đầu của slow start bắt đầu từ `cwnd = 10`,
   rồi tính nó cho 6 RTT tiếp theo của additive increase AIMD sau một lần
   loss giảm nó một nửa.
2. Dùng công thức throughput căn bậc hai, tính sự khác biệt throughput
   giữa một đường truyền RTT 1ms/loss 0.01% và một đường truyền RTT
   150ms/loss 0.01% cho cùng MSS — định lượng riêng RTT tốn của bạn bao
   nhiêu.
3. Capture một lần transfer file thật bằng `tcpdump` hoặc `ss -i` và xác
   định phase slow-start (tăng `cwnd` theo cấp số) chuyển sang congestion
   avoidance (tăng tuyến tính) từ đường cong throughput quan sát được.
4. Check `sysctl net.ipv4.tcp_congestion_control` trên một máy Linux bạn
   kiểm soát, đổi giữa `cubic` và `bbr` (nếu có), và so sánh
   throughput/latency trên một đường truyền giả lập cố tình lossy hoặc
   RTT cao (`tc netem`).
5. Viết một đoạn nối phần toán của file này với tuyên bố của
   [`01-network/08-tcp.md`](../01-network/08-tcp.md) rằng tái sử dụng kết nối quan trọng cho
   throughput "không chỉ latency" — chỉ ra lý do định lượng vì sao slow
   start của một kết nối mới thực sự tốn của bạn.
