# Hardware Basics: CPU, Interrupt, DMA, và Một Packet Tới Được Code Của Bạn Như Thế Nào

Cỗ máy mà kernel quản lý. Không phải kiến trúc máy tính như một ngành — chỉ là
những phần bạn cần để "syscall," "interrupt," "context switch," và "zero-copy"
thôi là những từ thần chú. Thuộc series fundamentals từ con số 0; xem
[`01-fundamentals.md`](01-fundamentals.md) để có index.

## What to learn

### CPU: fetch, execute, và một chiếc thang tốc độ
Một core CPU lặp đi lặp lại: fetch một instruction, decode và execute nó, giữ các
giá trị đang làm việc trong vài chục **register**. Mọi thứ khác đều chậm hơn và xa
hơn: L1 cache (~1 ns), L2/L3 (~4–40 ns), RAM (~100 ns), SSD (~100 µs), network
(~100 µs–100 ms) — xem [`08-memory-basics.md`](08-memory-basics.md) cho hệ phân cấp và
[`17-performance/`](../17-performance) cho việc nó làm gì với cấu trúc dữ liệu của bạn. Một CPU hiện đại
có nhiều **core** (đơn vị thực thi độc lập) và thường có **hyperthread** (hai hardware
thread dùng chung tài nguyên của một core). Kernel thấy mỗi hardware thread là một
"CPU" (`nproc` đếm chúng), và số worker mặc định của tokio chính là con số đó
([`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md)). Machine code gắn với một **instruction set**
(x86-64, aarch64), vì thế bạn mới phải cross-compile và code SIMD mới theo từng kiến
trúc.

### Privilege level: vì sao kernel làm được điều bạn không làm được
Bản thân CPU có các mode. Code kernel chạy ở mode đặc quyền (ring 0 trên x86) nơi nó
chạy được các instruction đặc biệt (nói chuyện với device, sửa page table, mask
interrupt) và đụng được vào mọi vùng nhớ; code user chạy không đặc quyền (ring 3) và
*bất kỳ* nỗ lực nào chạy instruction đặc quyền hay đụng vùng nhớ không được map cho nó
đều khiến CPU **trap** vào kernel. **Syscall** là một trap có chủ đích, được kiểm soát:
một instruction đặc biệt (`syscall`) chuyển CPU sang kernel mode tại một entry point cố
định ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)). Sự cô lập process của
[`03-processes-and-threads.md`](03-processes-and-threads.md) được thực thi bởi phần cứng (**MMU**, bên dưới), không phải bằng sự
tử tế.

### MMU: địa chỉ ảo là một tính năng phần cứng
Mọi địa chỉ chương trình dùng đều là **ảo**. **MMU** dịch từng cái sang địa chỉ vật lý ở
mỗi lần truy cập, dùng page table do kernel duy trì, theo các **page** kích thước cố định
(thường 4 KiB). **TLB** cache các bản dịch gần đây; miss tốn một lần duyệt bảng. Nếu một
page không được map hoặc không được phép (ghi vào read-only, user đụng vùng nhớ kernel),
CPU nâng một **page fault** và kernel quyết định: load nó, allocate nó, hay giết process
bằng `SIGSEGV` ([`16-memory.md`](16-memory.md)). Cô lập, `mmap`, `fork` copy-on-write, allocate lười
và memory-mapped file đều là một cơ chế này.

### Interrupt: device vỗ vai CPU
CPU không thể poll mọi device. Thay vào đó device nâng một **interrupt**: CPU dừng việc
đang làm, lưu state, nhảy tới một **interrupt handler** của kernel, rồi tiếp tục. Handler
được giữ cực nhỏ — ack device, lên lịch công việc thật như một **softirq** — vì interrupt
vô hiệu hóa các interrupt khác ([`16-kernel/04-interrupt.md`](../16-kernel/04-interrupt.md)). Ngoài device, một
**timer interrupt** nổ (hàng trăm hoặc hàng nghìn lần mỗi giây) cho kernel cơ hội định kỳ
để preempt task đang chạy ([`12-cpu-scheduling.md`](12-cpu-scheduling.md)); không có nó một process lặp vô
hạn sẽ chiếm core của nó mãi mãi. Interrupt và trap là cách thế giới bên ngoài và lỗi của
chính bạn thu hút sự chú ý của kernel; **signal** ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)) là kernel chuyển
tiếp những event như vậy *lên* process của bạn.

### DMA và network card: byte tới nơi mà CPU không phải copy
Một **NIC** (network interface card) có các **ring buffer** phần cứng trong RAM. Khi một
frame tới, NIC ghi nó thẳng vào một buffer bộ nhớ đã cấp sẵn bằng **DMA** (direct memory
access — device ghi vào RAM mà CPU không phải di chuyển từng byte), đẩy ring tiến lên, và
nâng một interrupt (hoặc kernel poll, khi tải cao — **NAPI**). Network stack của kernel
sau đó xử lý frame ([`01-network/07-link-layer.md`](../01-network/07-link-layer.md) -> IP -> TCP), đặt payload lên receive
queue của socket, và **wake up process** đang chờ trong `epoll_wait`/`read`. Chỉ khi đó
code của bạn mới chạy, và `read()` copy byte từ buffer của kernel sang buffer của bạn. Khi
gửi thì ngược lại: `write` của bạn copy vào socket buffer, TCP cắt thành segment, và NIC
DMA frame ra ngoài.

```text
dây -> NIC -> DMA vào ring buffer -> IRQ/NAPI -> softirq: xử lý IP+TCP
    -> socket receive queue -> wake up epoll_wait -> read() của bạn copy sang user buffer
```

Hai hệ quả: mỗi byte bạn proxy bị copy ít nhất hai lần (kernel->user khi read,
user->kernel khi write), đó là thứ [`18-zerocopy.md`](18-zerocopy.md) cố loại bỏ; và công việc
interrupt/softirq diễn ra trên core mà queue của NIC được gắn vào, đó là thứ RSS/RPS
([`16-kernel/05-rss.md`](../16-kernel/05-rss.md), [`16-kernel/06-rps.md`](../16-kernel/06-rps.md)) tinh chỉnh.

### Storage, ngắn gọn
Ổ đĩa và SSD là **block device**: đọc/ghi theo block kích thước cố định, chậm hơn RAM nhiều
bậc, và có khoảng cách lớn giữa truy cập tuần tự và ngẫu nhiên (lớn hơn nhiều trên đĩa
quay). Kernel che giấu điều này bằng **page cache** (RAM giữ dữ liệu file dùng gần đây) và
ghi xuống **lười** ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)). Proxy chủ yếu tránh đĩa — trừ log, cache
file và static asset — vì thế một `write()` bất ngờ vào file log chậm có thể làm đứng một
request path.

### Context switch là gì
Chuyển một core từ thread này sang thread khác nghĩa là lưu register và instruction pointer
của thread cũ, đổi page table sang process của thread mới (nếu khác), và khôi phục register
của thread mới — cộng với chi phí vô hình sau đó: dữ liệu của thread mới không có trong CPU
cache hay TLB, nên nó chạy chậm cho tới khi chúng ấm lên. Một syscall tự nó chuyển *mode*,
không phải thread; một **context switch** đổi *thread nào đang chạy*. Chi phí trực tiếp là
vài micro giây; phạt do cache thường lớn hơn. Đó là lý do thật vì sao thread-per-connection
thua event loop ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)) ở mức hàng chục nghìn connection.

### Gotcha: "CPU đang bận" là bốn thứ khác nhau
Dòng CPU của `top` chia thời gian thành `us` (code user), `sy` (kernel: syscall, network
stack), `si`/`hi` (soft/hard interrupt), `wa` (chờ disk I/O), và `st` (bị hypervisor lấy
trong VM). Một proxy ở 100% của một core với `si` cao đang bị nghẽn ở xử lý packet
(RSS/RPS, [`16-kernel/`](../16-kernel)), không phải code Rust của bạn; `sy` cao chỉ ra overhead syscall
([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)); `st` cao nghĩa là hàng xóm ồn ào. Hãy nhìn phép chia này
trước khi tối ưu bất cứ thứ gì.

## Practice

1. Chạy `lscpu` và `nproc`; xác định socket, số core mỗi socket, số thread mỗi core,
   kích thước cache và NUMA node, rồi so sánh `nproc` với số worker thread mà tokio khởi
   động trong [`labs/00-tcp-server`](../../labs/00-tcp-server).
2. Đọc `/proc/interrupts` hai lần cách nhau mười giây, trong khi chạy một vòng `curl` vào
   server local (hoặc `iperf3` giữa các host); tìm các hàng của NIC (hay softirq của `lo`)
   và xem counter của CPU nào nhúc nhích. Sau đó đọc `/proc/softirqs` và tìm `NET_RX` và
   `TIMER`.
3. Chạy `vmstat 1` và đọc `cs` (context switch/giây) và `in` (interrupt/giây) khi idle, rồi
   dưới một load test của [`labs/00-tcp-server`](../../labs/00-tcp-server); so sánh một biến thể
   thread-per-connection (một chương trình scratch dùng `std::thread::spawn`) với bản tokio
   của bạn ở 1.000 connection đồng thời.
4. Chạy `mpstat -P ALL 1` (hoặc `top` rồi `1`) dưới tải và đọc phép chia `%usr`, `%sys`,
   `%soft` theo từng core; ghi lại core nào xử lý network softirq.
5. Dùng `perf stat -e context-switches,cpu-migrations,page-faults ./target/release/<bin>`
   trên một chương trình nhỏ để thấy context switch và page fault dưới dạng counter
   ([`12-testing/05-debugging.md`](../12-testing/05-debugging.md)).
6. Đo thời gian truy cập tuần tự vs ngẫu nhiên trên một mảng 1 GiB trong một chương trình
   Rust scratch và giải thích khoảng cách theo hệ phân cấp cache/TLB ở trên.
