# CPU Scheduling trong thực tế: Run Queue, Priority, Affinity, và CFS Throttling

Kernel quyết định thread nào chạy trên core nào ra sao, và điều đó có nghĩa gì cho tail latency, giới
hạn CPU trong container và số lượng thread. Đây là layer thực hành; các thuật toán học thuật (FCFS,
round-robin, MLFQ, chứng minh fairness) nằm ở [`22-theory/04-cpu-scheduling.md`](../22-theory/04-cpu-scheduling.md) và phần bên trong của Linux ở
[`16-kernel/07-scheduler.md`](../16-kernel/07-scheduler.md).

## What to learn

### Thread chờ trong queue; scheduler chọn
Tại mọi thời điểm số thread runnable nhiều hơn số core rất nhiều. Mỗi core có một **run queue** các thread
đã sẵn sàng; kernel scheduler chọn một cái, cho nó chạy một **timeslice** (hoặc cho tới khi nó block hay bị
timer interrupt preempt, [`02-hardware-basics.md`](02-hardware-basics.md)), rồi chọn lại. Một thread ở một trong vài state: **running**,
**runnable** (đang chờ một core — `R` trong `ps`), **sleeping** (block ở I/O, một lock, hay một timer — `S`, hoặc
`D` uninterruptible cho một số lần chờ disk), hoặc stopped/zombie ([`04-process-lifecycle.md`](04-process-lifecycle.md)). Một `read` blocking
([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)) chuyển thread runnable -> sleeping, giải phóng core; dữ liệu tới chuyển nó về runnable.
**Latency từ "sẵn sàng" đến "thực sự chạy" là thời gian chờ run-queue**, và nó là phần tail latency mà không
code hiệu quả nào sửa được.

### CFS: công bằng bằng virtual runtime
Scheduler mặc định của Linux cho thread thường (CFS, và người kế nhiệm EEVDF trong các kernel gần đây) nhắm
tới **chia công bằng**: nó theo dõi mỗi thread đã nhận bao nhiêu CPU (**virtual runtime** của nó, `vruntime`) và
luôn chạy thread đã nhận ít nhất, để mọi thread runnable nhận phần lát gần như bằng nhau theo thời gian. Một
thread sleep nhiều (một proxy worker thiên về I/O) tích ít vruntime, nên khi I/O của nó xong nó được ưu tiên và
chạy sớm — **các task tương tác và thiên về I/O tự nhiên có latency tốt** so với các thread ngốn CPU. Một
thread **CPU-bound** dùng hết slice và bị preempt. Timeslice co giãn theo số thread tranh nhau: nhiều thread
runnable hơn trên mỗi core nghĩa là mỗi cái có slice ngắn hơn và chờ lâu hơn giữa các lượt.

### Priority: nice, real-time, và điều không nên làm
Trọng số cho sự công bằng đến từ giá trị **nice** (-20..+19; nice cao hơn = phần ít hơn). `nice -n 10 ./batch` và
`renice` điều chỉnh nó; hạ xuống dưới 0 cần đặc quyền (`CAP_SYS_NICE`, [`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)). Tách biệt là các policy
**real-time** (`SCHED_FIFO`, `SCHED_RR`) chạy nghiêm ngặt trước mọi thread thường với priority cố định; một
thread real-time quay vòng sẽ starve mọi thứ khác, kể cả việc dọn dẹp của chính kernel, và có thể làm treo máy.
Với proxy đây hầu như không bao giờ là công cụ đúng — hãy sửa tranh chấp ([`17-performance/`](../17-performance)). `chrt -p <pid>`
hiện policy của một thread.

### Affinity và migration: core nào?
Mặc định một thread có thể chạy trên bất kỳ core nào, và scheduler **migrate** thread để load balancing. Mỗi lần
migrate mất cache đã ấm: dữ liệu của thread nằm trong L1/L2 của core *cũ* ([`17-performance/`](../17-performance)). **CPU affinity**
pin một thread/process vào các core chọn trước (`taskset -c 0-3 ./proxy`, `sched_setaffinity`). Nó có ích trong
thiết kế thread-per-core (mỗi core một worker, không bao giờ bị migrate, không chia sẻ gì — kiến trúc của nginx
worker, và là một lý do để xem `glommio` thay vì work-stealing của tokio khi cần throughput cực đại), và để giữ proxy
trên cùng các core xử lý NIC queue của nó ([`16-kernel/05-rss.md`](../16-kernel/05-rss.md)) hoặc trên một **NUMA** node duy nhất
([`16-memory.md`](16-memory.md)). Nó gây hại nếu pin cẩu thả: hai worker bận trên một core trong khi các core khác rảnh.
`isolcpus`/cgroup `cpuset` dành riêng core cho công việc nhạy latency.

### Load average và "bận" nghĩa là gì
Ba **load average** của `uptime` (1/5/15 phút) đếm các thread *runnable hoặc đang uninterruptible sleep* — không phải
phần trăm CPU. Load 8 trên 8 core nghĩa là bão hòa; 8 trên 32 core là phần lớn rảnh; nhưng load cao với CPU *thấp* chỉ ra
các thread kẹt ở state `D` (đĩa hoặc NFS chậm) thay vì đang tính toán. Ghép với `top`/`vmstat` (`r` = runnable, `b` =
blocked), và phép chia CPU `us`/`sy`/`si`/`wa`/`st` ([`02-hardware-basics.md`](02-hardware-basics.md)). **Pressure Stall Information**
(`/proc/pressure/cpu`, `cpu.pressure` theo cgroup) báo trực tiếp tỉ lệ thời gian các task runnable mà không chạy — tín hiệu
"tôi đang bị đói CPU" rõ nhất.

### Giới hạn CPU của cgroup và throttling: cú bất ngờ của container
Trong container ([`13-containers.md`](13-containers.md)) một **limit** CPU (`cpu.max`, `limits.cpu` của Kubernetes) là một *quota trên mỗi
chu kỳ*: ví dụ 200 ms CPU time cho mỗi chu kỳ 100 ms = tương đương 2 core. Một process multi-threaded có thể đốt cả quota ở phần đầu một
chu kỳ bằng cách dùng nhiều core song song, rồi **mọi thread của nó bị đóng băng (throttle) cho tới chu kỳ kế tiếp** — thêm hàng
chục mili giây latency cho mọi request trong cửa sổ đó dù mức dùng CPU trung bình trông thấp. `cpu.stat` hiện `nr_throttled` và
`throttled_usec`; một số khác 0 và tăng dần là chẩn đoán. Hai cái bẫy làm nó tệ hơn:

- Một runtime định cỡ số worker thread từ số core của host (`nproc`) — thường là 64 — không phải từ quota của container. Sáu mươi
  bốn tokio worker chia nhau một quota 2 core sẽ *tự throttle chính mình*. Hãy đặt số worker thread tường minh khớp với limit
  ([`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md)).
- Một `request` (trọng số tương đối) khác một `limit` (trần cứng): request quyết định ai thắng khi tranh chấp; limit throttle cả khi
  máy đang rảnh. Nhiều team nhạy latency set request mà không set CPU limit.

### Thread vs core: định cỡ runtime
Với code async không bao giờ block, dùng khoảng một worker mỗi core: nhiều hơn chỉ thêm context switch và cache churn
([`02-hardware-basics.md`](02-hardware-basics.md)). Với công việc *blocking* (file I/O qua `tokio::fs`, DNS qua `getaddrinfo`, nén nặng CPU) hãy chuyển nó sang
`spawn_blocking` (một pool riêng, lớn hơn) để nó không starve các async worker; nếu không vài task chậm giữ mọi worker và **mọi
connection stall cùng lúc** — dấu hiệu "mọi latency tăng vọt cùng một lúc" ([`01-network/14-dns.md`](../01-network/14-dns.md)). Lấy mẫu xem thread nào runnable vs đang chờ là việc của `perf`
và `tokio-console` ([`12-testing/05-debugging.md`](../12-testing/05-debugging.md)).

### Gotcha: hàng xóm ồn ào và steal time
Trên một VM, `st` (steal) là thời gian hypervisor chạy *người khác* khi vCPU của bạn runnable. Trên các host chia sẻ, đợt bùng của
tenant khác hiện ra thành p99 của bạn nhảy vọt mà code của bạn không đổi. Bạn không sửa được từ bên trong máy; bạn chỉ phát hiện được
nó (`st` trong `top`/`vmstat`) để thôi đuổi theo một bóng ma.

## Practice

1. Chạy `top -H -p <pid>` (thread) và `ps -eLo pid,tid,psr,stat,pcpu,comm` lên proxy tokio của bạn dưới tải; xác định core (`psr`)
   nào mỗi worker thread chạy gần nhất và nó di chuyển thường xuyên ra sao, và thread nào `R` vs `S`.
2. Khởi động một CPU hog (`yes > /dev/null`) pin bằng `taskset -c 0` và chạy [`labs/00-tcp-server`](../../labs/00-tcp-server) có và không có affinity
   vào core 0; đo latency request (`wrk`/`hey`, [`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)) trong mỗi trường hợp, rồi `nice -n 19`
   cái hog và so sánh.
3. Đọc `/proc/loadavg`, `vmstat 1` (`r`, `b`), và `cat /proc/pressure/cpu` khi hog và một load test chạy cùng nhau; liên hệ từng con
   số với số lượng "runnable".
4. Trong một container với `--cpus=2` trên một host nhiều core, chạy một chương trình tokio dùng worker mặc định và một request
   handler CPU-bound; xem `cat /sys/fs/cgroup/cpu.stat` (`nr_throttled`) và một histogram latency, rồi đặt `worker_threads(2)` tường
   minh và so sánh p99.
5. Chạy [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) với `taskset -c 0-1`, 4 worker, rồi 2 worker; load test và giải thích sự khác biệt bằng
   thời gian chờ run-queue.
6. Chuyển một bước nặng CPU (ví dụ gzip một body lớn) từ inline trong handler sang `spawn_blocking`, và cho thấy bằng một probe latency
   của request nhỏ chạy đồng thời rằng các request nhỏ không còn bị stall.
