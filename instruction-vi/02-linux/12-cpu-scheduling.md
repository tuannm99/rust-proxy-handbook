# CPU Scheduling trong thực tế: Run Queue, Priority, Affinity và CFS Throttling

File này trả lời: kernel quyết định thread nào chạy trên core nào như thế nào, và điều đó ảnh hưởng
gì đến tail latency, giới hạn CPU trong container và số lượng thread. Đây là phần thực hành. Các
thuật toán lý thuyết (FCFS, round-robin, MLFQ, chứng minh tính công bằng) nằm ở
[`22-theory/04-cpu-scheduling.md`](../22-theory/04-cpu-scheduling.md), còn phần bên trong của Linux nằm
ở [`16-kernel/07-scheduler.md`](../16-kernel/07-scheduler.md).

## What to learn

### Thread xếp hàng chờ, scheduler chọn
Tại mọi thời điểm, số thread runnable nhiều hơn số core rất nhiều. Mỗi core có một **run queue** chứa
các thread đã sẵn sàng. Kernel scheduler chọn một thread, cho nó chạy trong một **timeslice** (hoặc cho
đến khi nó bị block, hoặc bị timer interrupt preempt, [`02-hardware-basics.md`](02-hardware-basics.md)),
rồi chọn lại.

Một thread ở một trong các state sau:
- **running**: đang chạy trên một core.
- **runnable**: đang chờ một core (`R` trong `ps`).
- **sleeping**: bị block ở I/O, một lock hoặc một timer (`S`), hoặc `D` (uninterruptible) cho một số
  lần chờ disk.
- stopped hoặc zombie ([`04-process-lifecycle.md`](04-process-lifecycle.md)).

Một `read` blocking ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)) chuyển thread từ
runnable sang sleeping và nhả core ra. Khi dữ liệu tới, nó quay lại runnable.

**Thời gian từ lúc "sẵn sàng" đến lúc "thực sự chạy" là thời gian chờ trong run queue.** Đây là một phần
của tail latency mà code hiệu quả đến mấy cũng không sửa được.

### CFS: công bằng dựa trên virtual runtime
Scheduler mặc định của Linux cho thread thường là CFS (và EEVDF, người kế nhiệm của nó trong các kernel
gần đây). Mục tiêu là **chia CPU công bằng**. Nó theo dõi mỗi thread đã nhận bao nhiêu CPU (gọi là
**virtual runtime**, `vruntime`) và luôn chạy thread đã nhận ít nhất. Nhờ vậy mọi thread runnable nhận
được phần CPU gần bằng nhau theo thời gian.

Một thread sleep nhiều (ví dụ proxy worker thiên về I/O) tích lũy ít vruntime. Khi I/O của nó xong, nó được
ưu tiên và chạy sớm. Vì vậy **các task tương tác và thiên về I/O tự nhiên có latency tốt** so với các
thread ngốn CPU.

Một thread **CPU-bound** dùng hết slice và bị preempt. Timeslice co giãn theo số thread đang tranh nhau.
Càng nhiều thread runnable trên mỗi core, mỗi thread có slice càng ngắn và phải chờ lâu hơn giữa các lượt
chạy.

### Priority: nice, real-time, và điều không nên làm
Trọng số cho sự công bằng đến từ giá trị **nice** (-20 đến +19; nice càng cao thì phần CPU càng ít).
`nice -n 10 ./batch` và `renice` dùng để điều chỉnh. Hạ nice xuống dưới 0 cần đặc quyền (`CAP_SYS_NICE`,
[`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)).

Tách biệt với nice là các policy **real-time** (`SCHED_FIFO`, `SCHED_RR`). Chúng luôn chạy trước mọi thread
thường, với priority cố định. Nếu một thread real-time chạy vòng lặp mãi, nó làm đói mọi thứ khác, kể cả
công việc dọn dẹp của chính kernel, và có thể làm treo cả máy. Với proxy, đây hầu như không bao giờ là
công cụ đúng. Hãy giải quyết sự tranh chấp thay vì dùng nó ([`17-performance/`](../17-performance)).
`chrt -p <pid>` hiện policy của một thread.

### Affinity và migration: chạy trên core nào?
Mặc định một thread có thể chạy trên bất kỳ core nào, và scheduler **migrate** (chuyển) thread giữa các
core để cân bằng tải. Mỗi lần migrate làm mất cache đã nóng: dữ liệu của thread đang nằm trong L1/L2 của
core *cũ* ([`17-performance/`](../17-performance)).

**CPU affinity** gắn cố định một thread/process vào các core được chọn trước (`taskset -c 0-3 ./proxy`,
`sched_setaffinity`). Nó có ích trong:
- Thiết kế thread-per-core: mỗi core một worker, không bao giờ bị migrate, không chia sẻ gì. Đây là kiến
  trúc của nginx worker, và là một lý do để cân nhắc `glommio` thay cho work-stealing của tokio khi cần
  throughput cực đại.
- Giữ proxy trên cùng các core đang xử lý NIC queue của nó
  ([`16-kernel/05-rss.md`](../16-kernel/05-rss.md)), hoặc trên một **NUMA** node duy nhất
  ([`16-memory.md`](16-memory.md)).

Nếu pin cẩu thả, nó gây hại: hai worker bận phải chia nhau một core trong khi các core khác nằm rảnh.
`isolcpus` hoặc cgroup `cpuset` dành riêng một số core cho công việc nhạy latency.

### Load average và "bận" nghĩa là gì
Ba con số **load average** của `uptime` (1/5/15 phút) đếm các thread *runnable hoặc đang uninterruptible
sleep*. Đó không phải phần trăm CPU.

- Load 8 trên máy 8 core nghĩa là bão hòa.
- Load 8 trên máy 32 core nghĩa là phần lớn còn rảnh.
- Load cao nhưng CPU *thấp* nghĩa là các thread đang kẹt ở state `D` (đĩa hoặc NFS chậm), chứ không phải
  đang tính toán.

Hãy đối chiếu với `top`/`vmstat` (`r` = runnable, `b` = blocked) và phép chia CPU `us`/`sy`/`si`/`wa`/`st`
([`02-hardware-basics.md`](02-hardware-basics.md)).

**Pressure Stall Information** (`/proc/pressure/cpu`, hoặc `cpu.pressure` theo từng cgroup) cho biết trực
tiếp tỷ lệ thời gian mà các task runnable không được chạy. Đây là tín hiệu rõ nhất cho biết "tôi đang bị
thiếu CPU".

### Giới hạn CPU của cgroup và throttling: điều bất ngờ trong container
Trong container ([`13-containers.md`](13-containers.md)), một **limit** CPU (`cpu.max`, hoặc `limits.cpu`
của Kubernetes) là một *quota cho mỗi chu kỳ*. Ví dụ: 200 ms CPU time cho mỗi chu kỳ 100 ms, tương đương
2 core.

Một process multi-threaded có thể đốt hết quota ngay đầu chu kỳ bằng cách dùng nhiều core song song. Sau đó
**mọi thread của nó bị đóng băng (throttle) cho đến chu kỳ kế tiếp**. Mỗi request trong khoảng thời gian đó
bị thêm hàng chục mili giây latency, dù mức dùng CPU trung bình trông thấp.

`cpu.stat` hiện `nr_throttled` và `throttled_usec`. Nếu hai số này khác 0 và tăng dần, đó là chẩn đoán. Có
hai cái bẫy làm tình hình tệ hơn:

- Runtime tính số worker thread từ số core của host (`nproc`, thường là 64), không phải từ quota của
  container. Sáu mươi bốn tokio worker chia nhau quota 2 core sẽ *tự throttle chính mình*. Hãy đặt số
  worker thread tường minh cho khớp với limit
  ([`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md)).
- Một `request` (trọng số tương đối) khác với một `limit` (trần cứng). Request quyết định ai thắng khi có
  tranh chấp. Limit thì throttle ngay cả khi máy đang rảnh. Nhiều team nhạy latency đặt request mà không
  đặt CPU limit.

### Thread và core: chọn kích thước runtime
Với code async không bao giờ block, dùng khoảng một worker cho mỗi core. Nhiều hơn chỉ thêm context switch
và làm cache bị xáo trộn ([`02-hardware-basics.md`](02-hardware-basics.md)).

Với công việc *blocking* (file I/O qua `tokio::fs`, DNS qua `getaddrinfo`, nén nặng CPU), hãy chuyển sang
`spawn_blocking` (một pool riêng, lớn hơn). Nhờ vậy nó không làm đói các async worker. Nếu không, chỉ vài
task chậm là giữ hết mọi worker, và **mọi connection bị đứng cùng một lúc**. Triệu chứng là "mọi latency
tăng vọt cùng một lúc" ([`01-network/14-dns.md`](../01-network/14-dns.md)).

Việc lấy mẫu xem thread nào đang runnable, thread nào đang chờ là việc của `perf` và `tokio-console`
([`12-testing/05-debugging.md`](../12-testing/05-debugging.md)).

### Gotcha: hàng xóm ồn ào và steal time
Trên một VM, `st` (steal) là thời gian hypervisor chạy *người khác* trong khi vCPU của bạn đang runnable.
Trên host dùng chung, khi tenant khác tăng tải đột ngột, p99 của bạn nhảy vọt dù code của bạn không đổi.
Bạn không sửa được điều này từ bên trong máy. Bạn chỉ phát hiện được nó (xem `st` trong `top`/`vmstat`) để
khỏi mất công đuổi theo một lỗi không có thật trong code.

## Practice

1. Chạy `top -H -p <pid>` (xem thread) và `ps -eLo pid,tid,psr,stat,pcpu,comm` lên proxy tokio của bạn khi
   đang có tải. Xác định mỗi worker thread gần đây chạy trên core nào (`psr`), nó di chuyển thường xuyên
   đến đâu, và thread nào đang `R`, thread nào đang `S`.
2. Chạy một CPU hog (`yes > /dev/null`) được pin bằng `taskset -c 0`. Chạy
   [`labs/00-tcp-server`](../../labs/00-tcp-server) có và không có affinity vào core 0. Đo latency request
   (`wrk`/`hey`, [`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)) trong mỗi trường hợp.
   Sau đó `nice -n 19` cho cái hog và so sánh lại.
3. Đọc `/proc/loadavg`, `vmstat 1` (`r`, `b`) và `cat /proc/pressure/cpu` khi hog và một load test chạy
   cùng nhau. Liên hệ từng con số với số lượng thread "runnable".
4. Trong một container với `--cpus=2` trên một host nhiều core, chạy một chương trình tokio dùng số worker
   mặc định cùng một request handler CPU-bound. Xem `cat /sys/fs/cgroup/cpu.stat` (`nr_throttled`) và
   histogram latency. Sau đó đặt `worker_threads(2)` tường minh và so sánh p99.
5. Chạy [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) với `taskset -c 0-1`, lần lượt dùng 4 worker
   rồi 2 worker. Load test và giải thích sự khác biệt bằng thời gian chờ trong run queue.
6. Chuyển một bước nặng CPU (ví dụ gzip một body lớn) từ chạy trực tiếp trong handler sang
   `spawn_blocking`. Dùng một probe latency gửi các request nhỏ chạy đồng thời để cho thấy chúng không còn
   bị đứng.

---

<!-- nav -->
← Trước: [`02-linux/11-time-and-timers.md`](11-time-and-timers.md) · [Mục lục](00-README.md) · Tiếp theo: [`02-linux/13-containers.md`](13-containers.md) →
