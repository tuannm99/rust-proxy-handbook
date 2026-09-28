# CPU Scheduling Algorithm

## What to learn

### Các metric scheduling thực sự được đánh giá bằng
Turnaround time (hoàn thành − đến), waiting time (turnaround − service
time), và response time (phản hồi đầu tiên − đến) đo những thứ khác
nhau, và một scheduler tối ưu cái này có thể làm tệ cái khác. Phần thảo
luận thực dụng về CFS của [`16-kernel/07-scheduler.md`](../16-kernel/07-scheduler.md) giả định các metric
này; file này định nghĩa chúng đủ chính xác để tính tay.

### FCFS và convoy effect
First-come-first-served là chính sách đơn giản nhất và tệ nhất dưới
workload hỗn hợp: một job dài đứng trước nhiều job ngắn làm mọi job ngắn
phải chờ sau nó — convoy effect. Đây là mô hình trực tiếp cho một bug
proxy rất thật: một request upstream chậm chiếm một worker thread trước
nhiều request nhanh (nguy cơ blocking-thread của [`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md)
là cùng hình dạng, ở một tầng cao hơn trong stack).

### SJF/SRTF: chứng minh được tối ưu cho waiting time trung bình, và vì sao không ai chạy nó nguyên bản
Shortest-Job-First (non-preemptive) hoặc Shortest-Remaining-Time-First
(preemptive) tối thiểu hóa waiting time trung bình — chứng minh được, đây
là tốt nhất có thể nếu bạn tối ưu thuần túy metric đó. Cái bẫy: nó cần
biết trước độ dài job, không có sẵn cho hầu hết workload thật, và nó
starve job dài vô hạn dưới sự đến liên tục của job ngắn.

### Round-robin và trade-off kích thước quantum
Mỗi process nhận một time slice cố định (quantum) trước khi bị preempt
lại vào hàng đợi. Quantum quá lớn suy biến về gần FCFS, với convoy effect
của nó; quantum quá nhỏ lãng phí thời gian vào overhead context-switch so
với công việc thật được làm. Không có quantum đúng phổ quát — nó là một
trade-off trực tiếp giữa responsiveness và throughput, tune theo từng
workload.

```text
quantum quá lớn  -> hành xử như FCFS, job dài chặn job ngắn
quantum quá nhỏ  -> hầu hết thời gian CPU tốn vào context-switch, không tính toán
```

### Multi-Level Feedback Queue (MLFQ): xấp xỉ SJF mà không cần biết độ dài job
MLFQ chạy nhiều hàng đợi ở các mức priority khác nhau với quantum khác
nhau; một process dùng hết quantum của nó (hành xử như job dài) bị giảm
xuống một hàng đợi priority thấp hơn, quantum dài hơn, còn một process
yield sớm (hành xử như job ngắn I/O-bound) giữ priority cao. Cái này xấp
xỉ lợi ích của SJF chỉ dùng hành vi *quan sát được* thay vì cần biết
trước — nguyên lý thiết kế đằng sau nhiều scheduler OS đa dụng (Windows,
BSD/Solaris đời cũ). Linux đi đường khác: CFS (fair share theo virtual
runtime) và, từ kernel 6.6, EEVDF (earliest eligible virtual deadline
first) — cả hai ưu ái task ngắn, I/O-bound qua cơ chế accounting chứ không
qua hàng đợi priority tường minh, nên *mục tiêu* giống MLFQ dù cơ chế thì
không.

### Nơi cái này nối với bài toán scheduling của chính proxy
Scheduler work-stealing của một tokio runtime ([`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md))
và budget cooperative-yield (phần coop của [`03-rust/05-async.md`](../03-rust/05-async.md)) đang
giải một phiên bản của chính xác bài toán này ở một tầng trên kernel:
nhiều task chia sẻ một số nhỏ worker thread, cần fairness mà không biết
trước độ dài task. Lý thuyết tầng kernel ở đây là tổ tiên trực tiếp của
chính thiết kế scheduling của tokio.

## Practice
1. Tính tay turnaround/waiting/response time cho một tập job cố định nhỏ
   (cho arrival time và burst time) dưới FCFS, rồi dưới SJF — xác nhận
   waiting time trung bình của SJF thấp hơn.
2. Mô phỏng convoy effect: một job dài theo sau bởi năm job ngắn dưới
   FCFS so với SJF, và tính sự khác biệt tổng waiting time.
3. Implement round-robin trên cùng tập job ở hai kích thước quantum khác
   nhau (rất nhỏ, rất lớn) và vẽ số context-switch so với thời gian hoàn
   thành cho mỗi cái.
4. Implement một MLFQ 3-mức tối thiểu (quy tắc: dùng hết quantum -> giảm
   mức; yield sớm -> giữ mức) trên một workload tổng hợp CPU-bound/IO-bound
   trộn lẫn, và so sánh waiting time trung bình với round-robin thường.
5. Đọc [`16-kernel/07-scheduler.md`](../16-kernel/07-scheduler.md) và [`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md) liền nhau
   và viết một đoạn ánh xạ ý tưởng cốt lõi của MLFQ (priority dựa trên
   hành vi, không cần biết trước) lên budget scheduling cooperative của
   tokio.
