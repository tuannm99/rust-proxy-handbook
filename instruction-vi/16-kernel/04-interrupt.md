# Interrupt và Softirq

Vì sao "cái hộp này xử lý được bao nhiêu packet mỗi giây" một phần là câu
hỏi về xử lý interrupt, chứ không chỉ là code ứng dụng — và vì sao câu hỏi
đó càng khó hơn, chứ không dễ hơn, khi số core tăng lên.

## What to learn

### Hard IRQ: tối thiểu, nhanh, tắt interrupt
Khi NIC có một packet sẵn sàng, nó phát ra một hardware interrupt. Handler
chạy cho nó (ngữ cảnh "hard IRQ") chạy với interrupt bị tắt trên core đó
và phải cực kỳ nhanh — nó chỉ làm mức tối thiểu (acknowledge thiết bị,
xếp hàng công việc tiếp theo) rồi trả về ngay. Bất cứ thứ gì chậm hơn sẽ
chặn *toàn bộ* việc xử lý interrupt trên core đó, kể cả timer interrupt mà
scheduler phụ thuộc vào.

### Softirq: nửa còn lại, hoãn lại và có thể lập lịch
Việc xử lý packet thực sự — đi ngược lên network stack, demux socket,
cuối cùng chạm tới đường wakeup của epoll
([`16-kernel/01-epoll-internals.md`](01-epoll-internals.md)) — xảy ra trong một **softirq**, được
lập lịch chạy ngay sau khi hard IRQ handler trả về, nhưng trong một ngữ
cảnh có thể bị ngắt và chịu áp lực lập lịch bình thường. `NET_RX` là
softirq chịu trách nhiệm cụ thể cho việc xử lý packet đến. Dưới tốc độ
packet cao kéo dài, chính công việc softirq `NET_RX` có thể tiêu tốn đủ
nhiều một core khiến việc lập lịch process bình thường trên core đó bị đói
— thể hiện qua `%si` (softirq time) cao trong `top`/`mpstat`, và qua
latency ở bất cứ thứ gì khác đang cố chạy trên core đó.

### Chỗ này thể hiện ra sao khi tải cao, và mối liên hệ với RSS
Nếu interrupt của mọi packet (và do đó việc xử lý softirq của nó) đều rơi
vào một core bất kể ứng dụng dùng bao nhiêu core, core đó trở thành trần
throughput tổng bất kể [`proxy`](../../proxy) spawn bao nhiêu worker thread. Đây chính
xác là vấn đề mà [`16-kernel/05-rss.md`](05-rss.md) (hardware) và [`16-kernel/06-rps.md`](06-rps.md)
(software) giải quyết — dàn tải interrupt/softirq của các luồng khác nhau
ra nhiều core để việc xử lý packet tự nó scale theo số core.

### Gotcha: `irqbalance` xung đột với tinh chỉnh thủ công
`irqbalance` tự động cân bằng lại IRQ affinity giữa các core dựa trên tải.
Nếu bạn đã tự tay tinh chỉnh IRQ affinity để khớp với nơi thread ứng dụng
của bạn được pin ([`16-kernel/07-scheduler.md`](07-scheduler.md)), một daemon `irqbalance`
đang chạy có thể âm thầm chuyển interrupt ra khỏi các core bạn đã chọn,
tạo ra các đợt latency thất thường khó tái hiện vì việc gán cứ liên tục
thay đổi bên dưới bạn. Tắt `irqbalance` nếu bạn đang set IRQ affinity thủ
công — hai cái không được thiết kế để cùng tồn tại.

## Practice
1. Theo dõi `/proc/interrupts` và `/proc/softirqs` (hoặc cột
   `%irq`/`%soft` của `mpstat -P ALL 1`) trong khi tạo tải liên tục vào
   [`labs/00-tcp-server`](../../labs/00-tcp-server) hoặc [`proxy`](../../proxy).
2. Xác định core nào đang xử lý phần lớn công việc softirq `NET_RX` và so
   sánh với core nào worker thread ứng dụng của bạn thực sự đang chạy.
3. Nếu bạn có quyền truy cập vào hardware/VM có thể set IRQ affinity
   (`/proc/irq/<n>/smp_affinity`), thử align core interrupt của NIC với
   core ứng dụng đã pin (từ bài tập [`16-kernel/07-scheduler.md`](07-scheduler.md)) và đo tác
   động lên throughput và latency.
4. Kiểm tra xem `irqbalance` có đang chạy trên hệ thống test của bạn
   không; dừng nó, chạy lại thí nghiệm affinity, và so sánh độ ổn định của
   phép đo khi nó chạy so với khi nó dừng.
