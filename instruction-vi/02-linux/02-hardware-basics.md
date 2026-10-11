# Hardware Basics: CPU, Interrupt, DMA, và Đường Đi Của Một Packet Đến Code Của Bạn

File này nói về cỗ máy mà kernel quản lý. Đây không phải là môn kiến trúc máy tính. Nó
chỉ gồm những phần bạn cần để các từ "syscall", "interrupt", "context switch" và
"zero-copy" không còn là những câu thần chú. File thuộc nhóm fundamentals. Xem
[`01-fundamentals.md`](01-fundamentals.md) để có mục lục.

## What to learn

### CPU: fetch, execute, và chiếc thang tốc độ
Một core CPU lặp đi lặp lại một việc: fetch một instruction, decode rồi execute nó. Các giá
trị đang xử lý được giữ trong vài chục **register**. Mọi thứ khác đều chậm hơn và ở xa hơn:

| Nơi lưu trữ | Thời gian truy cập |
|---|---|
| L1 cache | ~1 ns |
| L2/L3 cache | ~4–40 ns |
| RAM | ~100 ns |
| SSD | ~100 µs |
| Network | ~100 µs – 100 ms |

Xem [`08-memory-basics.md`](08-memory-basics.md) cho hệ thống phân cấp này, và
[`17-performance/`](../17-performance) để biết nó ảnh hưởng thế nào đến cấu trúc dữ liệu
của bạn.

Một CPU hiện đại có nhiều **core** (đơn vị thực thi độc lập). Nó thường có thêm
**hyperthread**: hai hardware thread dùng chung tài nguyên của một core. Kernel coi mỗi
hardware thread là một "CPU" (`nproc` đếm chúng). Số worker mặc định của tokio cũng bằng
đúng con số này ([`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md)).

Machine code gắn với một **instruction set** (x86-64, aarch64). Vì thế bạn phải
cross-compile, và code SIMD phải viết riêng cho từng kiến trúc.

### Privilege level: vì sao kernel làm được việc mà bạn không làm được
Bản thân CPU có các mode (chế độ chạy).

- Code kernel chạy ở mode đặc quyền (ring 0 trên x86). Ở đó nó chạy được các instruction đặc
  biệt (nói chuyện với device, sửa page table, tắt interrupt) và truy cập được mọi vùng nhớ.
- Code user chạy ở mode không đặc quyền (ring 3). Nếu nó cố chạy instruction đặc quyền, hoặc
  chạm vào vùng nhớ không được map cho nó, CPU sẽ **trap** vào kernel.

**Syscall** là một trap có chủ đích và được kiểm soát. Một instruction đặc biệt (`syscall`)
chuyển CPU sang kernel mode tại một điểm vào cố định
([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)).

Việc cách ly giữa các process ([`03-processes-and-threads.md`](03-processes-and-threads.md))
do phần cứng bảo đảm (**MMU**, xem bên dưới). Nó không dựa vào việc các process tự giác.

### MMU: địa chỉ ảo là một tính năng của phần cứng
Mọi địa chỉ mà chương trình dùng đều là địa chỉ **ảo**. **MMU** dịch từng địa chỉ sang địa
chỉ vật lý ở mỗi lần truy cập. Nó dùng page table do kernel duy trì, chia bộ nhớ thành các
**page** có kích thước cố định (thường 4 KiB). **TLB** cache các bản dịch gần đây. Nếu miss,
CPU phải duyệt page table, tốn thêm thời gian.

Nếu một page chưa được map, hoặc truy cập không được phép (ghi vào vùng read-only, user chạm
vùng nhớ của kernel), CPU gây ra một **page fault** và kernel quyết định phải làm gì: load
page đó, allocate nó, hay giết process bằng `SIGSEGV` ([`16-memory.md`](16-memory.md)).

Cách ly process, `mmap`, `fork` kiểu copy-on-write, allocate lười, và memory-mapped file đều
dựa vào cùng một cơ chế này.

### Interrupt: device gọi CPU
CPU không thể liên tục hỏi (poll) từng device. Thay vào đó, device phát ra một **interrupt**.
CPU dừng việc đang làm, lưu state, nhảy tới **interrupt handler** của kernel, rồi quay lại
chạy tiếp.

Handler được giữ cực nhỏ: nó chỉ ack device và xếp công việc thật vào hàng đợi dưới dạng
**softirq**, vì khi đang xử lý interrupt thì các interrupt khác bị tắt
([`16-kernel/04-interrupt.md`](../16-kernel/04-interrupt.md)).

Ngoài device, còn có **timer interrupt** nổ hàng trăm hoặc hàng nghìn lần mỗi giây. Nó cho
kernel cơ hội định kỳ để preempt (ngắt) task đang chạy
([`12-cpu-scheduling.md`](12-cpu-scheduling.md)). Nếu không có timer interrupt, một process
lặp vô hạn sẽ chiếm core của nó mãi mãi.

Interrupt và trap là cách thế giới bên ngoài, và cả lỗi của chính bạn, thu hút sự chú ý của
kernel. **Signal** ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)) là cách
kernel chuyển tiếp những sự kiện đó *lên* process của bạn.

### DMA và network card: byte tới nơi mà CPU không phải copy
Một **NIC** (network interface card) có các **ring buffer** nằm trong RAM. Khi một frame tới:

1. NIC ghi nó thẳng vào một buffer bộ nhớ đã được cấp sẵn, bằng **DMA** (direct memory
   access: device ghi vào RAM mà CPU không phải chuyển từng byte).
2. NIC đẩy ring tiến lên, rồi phát ra một interrupt. (Khi tải cao, kernel chuyển sang tự
   poll, gọi là **NAPI**.)
3. Network stack của kernel xử lý frame ([`01-network/07-link-layer.md`](../01-network/07-link-layer.md)
   → IP → TCP), đặt payload vào receive queue của socket, và **wake up process** đang chờ
   trong `epoll_wait`/`read`.
4. Chỉ đến lúc này code của bạn mới chạy. `read()` copy byte từ buffer của kernel sang buffer
   của bạn.

Khi gửi thì ngược lại: `write` của bạn copy dữ liệu vào socket buffer, TCP cắt nó thành các
segment, và NIC dùng DMA để đưa frame ra ngoài.

```text
dây -> NIC -> DMA vào ring buffer -> IRQ/NAPI -> softirq: xử lý IP+TCP
    -> socket receive queue -> wake up epoll_wait -> read() của bạn copy sang user buffer
```

Có hai hệ quả:
- Mỗi byte mà bạn proxy bị copy ít nhất hai lần: kernel→user khi read, user→kernel khi write.
  [`18-zerocopy.md`](18-zerocopy.md) nhằm loại bỏ các lần copy này.
- Công việc interrupt/softirq chạy trên core mà queue của NIC được gắn vào. RSS/RPS
  ([`16-kernel/05-rss.md`](../16-kernel/05-rss.md), [`16-kernel/06-rps.md`](../16-kernel/06-rps.md))
  dùng để tinh chỉnh việc này.

### Storage, nói ngắn gọn
Ổ đĩa và SSD là **block device**: chúng đọc/ghi theo block có kích thước cố định, chậm hơn RAM
nhiều bậc. Truy cập tuần tự và truy cập ngẫu nhiên cũng cách nhau rất xa về tốc độ (trên đĩa
quay, khoảng cách này còn lớn hơn nhiều).

Kernel che giấu điều này bằng **page cache** (RAM giữ dữ liệu file vừa dùng) và bằng cách ghi
xuống đĩa **một cách lười** ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)).

Proxy phần lớn tránh đĩa, trừ log, cache file và static asset. Vì vậy một lần `write()` bất
ngờ vào file log chậm có thể làm đứng cả đường xử lý request.

### Context switch là gì
Chuyển một core từ thread này sang thread khác nghĩa là:
1. Lưu register và instruction pointer của thread cũ.
2. Đổi page table sang process của thread mới (nếu khác process).
3. Khôi phục register của thread mới.

Ngoài ra còn một chi phí vô hình đi theo sau: dữ liệu của thread mới chưa nằm trong CPU cache
hay TLB, nên nó chạy chậm cho đến khi chúng được nạp lại.

Một syscall chỉ đổi *mode*, không đổi thread. Một **context switch** mới đổi *thread nào đang
chạy*. Chi phí trực tiếp là vài micro giây. Chi phí do mất cache thường còn lớn hơn. Đó là lý
do thật sự khiến thread-per-connection thua event loop
([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)) khi có hàng chục nghìn
connection.

### Gotcha: "CPU đang bận" có thể là bốn thứ khác nhau
Dòng CPU của `top` chia thời gian thành:

- `us`: code user.
- `sy`: kernel (syscall, network stack).
- `si`/`hi`: soft/hard interrupt.
- `wa`: chờ disk I/O.
- `st`: thời gian bị hypervisor lấy đi (trong VM).

Cách đọc:
- Proxy chạy 100% một core mà `si` cao: nút thắt nằm ở xử lý packet (RSS/RPS,
  [`16-kernel/`](../16-kernel)), không phải ở code Rust của bạn.
- `sy` cao: overhead của syscall ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)).
- `st` cao: có "hàng xóm ồn ào" trên cùng máy vật lý.

Hãy xem phép chia này trước khi tối ưu bất cứ thứ gì.

## Practice

1. Chạy `lscpu` và `nproc`. Xác định số socket, số core mỗi socket, số thread mỗi core, kích
   thước cache và NUMA node. Sau đó so sánh `nproc` với số worker thread mà tokio khởi động
   trong [`labs/00-tcp-server`](../../labs/00-tcp-server).
2. Đọc `/proc/interrupts` hai lần, cách nhau mười giây, trong khi chạy một vòng lặp `curl` vào
   server local (hoặc `iperf3` giữa các host). Tìm các dòng của NIC (hoặc softirq của `lo`) và
   xem counter của CPU nào tăng. Sau đó đọc `/proc/softirqs` và tìm `NET_RX` và `TIMER`.
3. Chạy `vmstat 1` và đọc `cs` (context switch mỗi giây) và `in` (interrupt mỗi giây) khi máy
   idle, rồi trong lúc load test [`labs/00-tcp-server`](../../labs/00-tcp-server). So sánh một
   biến thể thread-per-connection (chương trình scratch dùng `std::thread::spawn`) với bản
   tokio của bạn ở 1.000 connection đồng thời.
4. Chạy `mpstat -P ALL 1` (hoặc `top` rồi bấm `1`) dưới tải và đọc phép chia `%usr`, `%sys`,
   `%soft` theo từng core. Ghi lại core nào xử lý network softirq.
5. Dùng `perf stat -e context-switches,cpu-migrations,page-faults ./target/release/<bin>` trên
   một chương trình nhỏ để thấy context switch và page fault dưới dạng counter
   ([`12-testing/05-debugging.md`](../12-testing/05-debugging.md)).
6. Trong một chương trình Rust scratch, đo thời gian truy cập tuần tự và truy cập ngẫu nhiên
   trên một mảng 1 GiB. Giải thích khoảng cách giữa hai kết quả bằng hệ thống phân cấp
   cache/TLB ở trên.

---

<!-- nav -->
← Trước: [`02-linux/01-fundamentals.md`](01-fundamentals.md) · [Mục lục](00-README.md) · Tiếp theo: [`02-linux/03-processes-and-threads.md`](03-processes-and-threads.md) →
