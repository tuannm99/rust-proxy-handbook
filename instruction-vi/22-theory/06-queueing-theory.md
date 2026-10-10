# Queueing Theory

## What to learn

### Little's Law: một công thức giải thích mọi thứ phía sau nó
`L = λW` — số lượng item trung bình trong một hệ thống (`L`) bằng tỷ lệ
đến trung bình (`λ`) nhân thời gian trung bình một item ở trong hệ thống
(`W`). Nó đúng cho *bất kỳ* hệ thống queue ổn định nào bất kể phân bố
đến, phân bố service, hay số server, chính xác là lý do nó là công thức
có ý nghĩa nhất trong capacity planning: biết tỷ lệ request và latency
mục tiêu cho bạn biết chính xác proxy của bạn phải giữ được bao nhiêu
request đang bay.

```text
L = λ * W
ví dụ: λ = 1000 req/s, mục tiêu W = 50ms = 0.05s  =>  L = 50 request đang bay
```
Đây là phiên bản chính xác, định lượng của lời khuyên "bound theo thời gian, không theo số lượng" của [`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md) — Little's
Law là thứ cho bạn biết số lượng *nào* tương ứng với giới hạn thời gian
thật của bạn, thay vì đoán một độ sâu queue.

### Queue M/M/1: mô hình đơn giản nhất đáng biết tên
"M/M/1" nghĩa là đến kiểu Markov (không nhớ, mũ), service time kiểu
Markov, 1 server. Các kết quả dạng đóng của nó — waiting time trung bình,
độ dài queue trung bình — đều biểu diễn theo `ρ = λ/μ` (utilization:
tỷ lệ đến trên tỷ lệ service), và mỗi kết quả đều bùng nổ khi `ρ → 1`.

```text
M/M/1 waiting time trung bình trong hàng: Wq = ρ / (μ(1 - ρ))
```
Đọc mẫu số đó: khi utilization `ρ` tiến tới 1 (server "gần như" ở full
capacity), waiting time phân kỳ về vô hạn — không tuyến tính, không nhẹ
nhàng, mà như một tiệm cận đứng.

### Vì sao "chúng ta mới 80% CPU, còn headroom" là sai nguy hiểm
Thay `ρ = 0.8` so với `ρ = 0.95` vào công thức M/M/1: waiting time không
tăng theo tỷ lệ tương đương với utilization tăng — nó tăng bùng nổ, vì
mẫu số của công thức là `(1-ρ)`, và `1-0.95 = 0.05` nhỏ hơn `1-0.8 = 0.2`
bốn lần trong khi tử số cũng tăng — `Wq` đi từ `4/μ` lên `19/μ`, gần gấp 5
lần thời gian chờ chỉ với chưa tới 20% load thêm. Đây
là căn cứ chặt chẽ cho toàn bộ tiền đề của
[`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md) (queue gần saturation là một cái bẫy
latency) và vì sao capacity planning nhắm utilization thấp hơn hẳn 100% —
"thấp hơn hẳn," định lượng, nghĩa là ở đủ xa tiệm cận để các biến động
nhỏ trong tỷ lệ đến không tạo ra dao động latency lớn.

### Biến động làm nó tệ hơn: trực giác Pollaczek-Khinchine
Service time thật không phải hàm mũ — chúng thường biến động hơn (một
cache hit tốn 1ms, một cache miss tốn 200ms, theo ví dụ bimodal của
[`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)). Kết quả M/G/1 (công thức Pollaczek-Khinchine)
chỉ ra waiting time tăng theo *phương sai* của service time, không chỉ
trung bình của nó — hai hệ thống có service time trung bình giống nhau
nhưng phương sai khác nhau có hành vi queue khác nhau, đó là lý do
hình thức vì sao [`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md) khuyên áp adaptive
concurrency limit theo từng lớp công việc có chi phí tương tự thay vì
toàn cục.

### Nhiều server thay đổi hình dạng, không thay đổi vực
Một queue M/M/c (c server song song, ví dụ c worker thread) trì hoãn
cùng sự bùng nổ đó tới utilization cao hơn và làm nó mượt hơn phần nào
(nền tảng của công thức Erlang-C dùng trong staffing call-center), nhưng
hình dạng định tính — waiting time phân kỳ khi utilization tổng tiến tới
1 — không biến mất. Thêm worker/thread mua headroom, không mua sự miễn
nhiễm.

## Practice
1. Dùng Little's Law, tính capacity request đang bay [`proxy`](../../proxy) cần giữ cho
   một p99 latency mục tiêu và một tỷ lệ request kỳ vọng bạn chọn — viết
   con số đó ra như một input capacity-planning thật.
2. Implement một mô phỏng M/M/1 nhỏ (đến kiểu Poisson, service time mũ)
   và đo thực nghiệm waiting time trung bình ở `ρ = 0.5, 0.8, 0.9, 0.95, 0.99`
   — vẽ nó và xác nhận hình dạng bùng nổ khớp công thức.
3. Lặp lại mô phỏng với một service time cố định (không mũ) thay vào đó,
   giữ cùng trung bình, và so sánh waiting time với trường hợp M/M/1 ở
   cùng `ρ` — nối sự khác biệt với trực giác Pollaczek-Khinchine về
   phương sai.
4. Load-test [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) ở các tỷ lệ request tăng dần và vẽ
   latency p50/p99 quan sát được so với utilization đo được; xác định
   nơi đường cong bắt đầu bẻ về tiệm cận lý thuyết.
5. Xem lại phần adaptive concurrency limiting của
   [`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md) và giải thích, dùng công thức của
   file này, vì sao nhắm một set-point utilization cố định (thay vì một
   số lượng request cố định) là lựa chọn có nền tảng lý thuyết.
