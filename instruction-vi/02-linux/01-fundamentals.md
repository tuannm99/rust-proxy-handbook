# Linux/OS Fundamentals

Bài học đầu tiên của lộ trình OS, và là bài mà mọi file sau đều dựa vào. Nó
không giả định gì: nếu bạn đã biết hệ điều hành làm gì, lướt qua trong hai mươi
phút; nếu chưa, hãy đọc hai lần và làm Practice trước khi đi tiếp. Một OS trông
như một trăm chủ đề không liên quan (process, memory, file, signal, container)
cho tới khi bạn thấy chúng đều là câu trả lời cho một câu hỏi — *làm sao nhiều
chương trình dùng chung một máy một cách an toàn?* — nên file này cho bạn câu
hỏi đó và tấm bản đồ trước.

## What to learn

### Hệ điều hành để làm gì
Một máy tính có vài tài nguyên vật lý: core CPU, RAM, đĩa, network card, một
chiếc đồng hồ. Nhiều chương trình muốn chúng cùng lúc, và không cái nào được phép
phá các cái khác. **Hệ điều hành** làm ba việc:

1. **Multiplex** từng tài nguyên giữa nhiều chương trình (cho mỗi cái ảo giác có
   CPU riêng và bộ nhớ riêng).
2. **Trừu tượng hóa** phần cứng sau những ý tưởng đơn giản, đồng nhất, để bạn
   không bao giờ phải lập trình trực tiếp disk controller hay network card.
3. **Bảo vệ**: ngăn một chương trình đọc hay làm hỏng bộ nhớ của chương trình khác
   hay của chính kernel.

Mỗi chủ đề trong thư mục này là một hàng của bảng này:

| Phần cứng | OS biến nó thành | Bạn dùng qua | Đọc |
|---|---|---|---|
| CPU | **process / thread**, được xếp lịch luân phiên | `fork`, `clone`, `exec` | [`03-processes-and-threads.md`](03-processes-and-threads.md), [`04-process-lifecycle.md`](04-process-lifecycle.md), [`12-cpu-scheduling.md`](12-cpu-scheduling.md) |
| RAM | **virtual memory**: address space riêng | `mmap`, `brk`, pointer thường | [`08-memory-basics.md`](08-memory-basics.md), [`16-memory.md`](16-memory.md) |
| Đĩa | **file** và directory | `open`, `read`, `write` | [`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md) |
| Network card | **socket** | `socket`, `bind`, `connect` | [`01-network/11-socket.md`](../01-network/11-socket.md) |
| Đồng hồ | thời gian và timer | `clock_gettime`, `timerfd` | [`11-time-and-timers.md`](11-time-and-timers.md) |
| Chương trình khác | pipe, signal, shared memory | `pipe`, `kill`, `mmap` | [`10-ipc.md`](10-ipc.md), [`17-signals.md`](17-signals.md) |
| Ai được làm gì | user, permission | `chmod`, capability | [`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md) |
| Giới hạn và cô lập | rlimit, **namespace, cgroup** | `setrlimit`, container | [`20-limits-and-proc.md`](20-limits-and-proc.md), [`13-containers.md`](13-containers.md) |

### Stack phần mềm, và "Linux" nghĩa là gì
Từ dưới lên: **phần cứng**, **kernel**, **system library** (`libc` bọc syscall thành
các hàm như `read()`), và **chương trình** (shell, `curl`, proxy của bạn). Nói
chặt chẽ, **Linux chỉ là kernel**; một *distribution* (Ubuntu, Debian, Alpine) gói
kernel đó với `libc`, một package manager, và hàng nghìn chương trình. Điều này
giải thích các khác biệt thực tế: Alpine dùng `musl` thay `glibc` (nên DNS và một
số hành vi khác, [`01-network/14-dns.md`](../01-network/14-dns.md)), một container mang theo userland riêng nhưng dùng chung kernel
của host ([`13-containers.md`](13-containers.md)), và `uname -r` hiện phiên bản *kernel* còn
`cat /etc/os-release` hiện *distro*.

### Cả vòng đời một chương trình, từ `./proxy` tới exit
Đây là câu chuyện mà phần còn lại của thư mục phóng to vào:

1. Bạn gõ `./proxy`. **Shell** (một chương trình bình thường) `fork` một bản sao của
   chính nó, và child `exec` `proxy` ([`04-process-lifecycle.md`](04-process-lifecycle.md)).
2. **Kernel** đọc file thực thi, dựng một **address space** riêng mới, map code của
   chương trình vào ([`08-memory-basics.md`](08-memory-basics.md)), và làm một thread trở nên runnable.
3. **Dynamic linker** nạp các shared library (`ldd ./proxy` liệt kê chúng); rồi
   `main` chạy. Runtime của Rust và tokio khởi động các worker **thread**.
4. Chương trình gọi **syscall** để làm bất cứ việc thật nào — `socket`, `bind`,
   `epoll_wait`, `read`, `write` — mỗi cái băng vào kernel rồi quay lại
   ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)); kết quả là các **file descriptor**.
5. Khi chờ I/O, nó **block** và scheduler chạy thứ khác
   ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)); khi dữ liệu tới, một **interrupt** đánh thức nó
   ([`02-hardware-basics.md`](02-hardware-basics.md)).
6. Một **signal** (`SIGTERM`) yêu cầu nó dừng; nó drain và gọi `exit`
   ([`17-signals.md`](17-signals.md)). Kernel giải phóng bộ nhớ và fd của nó; parent
   `wait` lấy exit status ([`04-process-lifecycle.md`](04-process-lifecycle.md)).

Ba ý tưởng giải thích phần lớn những gì theo sau, nên hãy thuộc lòng: **(1)** user
space và kernel space tách biệt, và cánh cửa duy nhất giữa chúng là syscall;
**(2)** gần như mọi thứ — file, socket, pipe, timer — là một **file descriptor**,
nên một cơ chế (`epoll`) chờ được tất cả; **(3)** kernel *multiplex* — mỗi process
thấy CPU và bộ nhớ riêng, đó là một ảo giác mà OS duy trì và có chi phí (context
switch, page fault) mà bạn đo được.

### Kernel tồn tại để làm gì
**Kernel** là chương trình duy nhất trên máy được phép nói chuyện trực tiếp
với hardware, quản lý memory cho mọi process, và enforce cách ly giữa
chúng. Mọi thứ khác — proxy của bạn, shell của bạn, mọi process khác —
chạy trong **user space**, không có quyền truy cập hardware trực tiếp và
không thể đụng vào memory của process khác. Gần như mọi chủ đề trong
[`02-linux/`](.) là hệ quả của đúng một ranh giới đó: cái giá phải trả khi băng
qua nó ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)), nó cách ly cái gì
([`03-processes-and-threads.md`](03-processes-and-threads.md), [`13-containers.md`](13-containers.md)), và chuyện gì xảy ra
khi kernel cần ngắt bạn thay vì chờ được hỏi
([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)).

### Mười hai mảnh, và mỗi mảnh nằm ở đâu
Đọc theo thứ tự này; mỗi mảnh xây trên các mảnh phía trên.

- **[`02-hardware-basics.md`](02-hardware-basics.md)** — chính cỗ máy: privilege level của CPU,
  MMU, interrupt, DMA, và một packet từ network card kết thúc trong `read()` của bạn thế nào.
- **[`03-processes-and-threads.md`](03-processes-and-threads.md)** — process và thread thật ra là gì, vì
  sao thread rẻ hơn, và vì sao chia sẻ memory giữa chúng là lý do
  [`03-rust/04-sync.md`](../03-rust/04-sync.md) tồn tại.
- **[`04-process-lifecycle.md`](04-process-lifecycle.md)** — `fork`, `exec`, `wait`, zombie, orphan
  và vì sao PID 1 trong container đặc biệt.
- **[`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)** — ranh giới user space/kernel space,
  syscall tốn bao nhiêu, và file descriptor — cái handle dạng số nguyên mà
  mọi thứ trong [`02-linux/14-epoll.md`](14-epoll.md) được xây quanh nó.
- **[`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)** — inode, path, page cache, `write`
  đảm bảo và không đảm bảo gì, thay thế file một cách atomic.
- **[`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)** — một process *là ai* trong mắt
  kernel, permission bit, capability, và bind port 443 mà không cần root.
- **[`08-memory-basics.md`](08-memory-basics.md)** — mức tối thiểu cần để đoạn mở đầu của
  [`02-linux/16-memory.md`](16-memory.md) cảm giác quen thuộc thay vì thông tin mới, cộng
  thêm RAM nằm ở đâu so với cache và disk.
- **[`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)** — vì sao một syscall có thể block
  một thread, vì sao event loop tồn tại như một giải pháp thay thế, và
  signal là gì (một sự gián đoạn từ bên ngoài luồng điều khiển bình
  thường của bạn, không phải một giá trị trả về).
- **[`10-ipc.md`](10-ipc.md)** — pipe, Unix socket, truyền file descriptor
  giữa các process, shared memory, và một `Mutex` thực sự chờ ra sao (`futex`).
- **[`11-time-and-timers.md`](11-time-and-timers.md)** — monotonic vs wall-clock time, timer
  và timeout hoạt động thế nào, và vì sao mọi lần chờ trong proxy đều cần deadline.
- **[`12-cpu-scheduling.md`](12-cpu-scheduling.md)** — run queue, priority, affinity, load
  average, và CPU-limit throttling trong container.
- **[`13-containers.md`](13-containers.md)** — container thật ra là gì (các process bị cách ly
  chạy trên cùng một kernel dùng chung, không phải một VM tí hon), vì
  Kubernetes/pod/cgroup được nhắc tới liên tục từ [`09-architecture/`](../09-architecture) trở
  đi mà chưa có chỗ nào định nghĩa chúng.

Đọc theo thứ tự đó một lần; sau đó, coi mỗi file như một chỗ tra cứu độc
lập. Các file Kernel mechanisms phía sau
([`14-epoll.md`](14-epoll.md) trở đi) cũng gồm tầng vận hành — Linux networking
và netfilter ([`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md)), giới hạn tài nguyên và
`/proc` ([`20-limits-and-proc.md`](20-limits-and-proc.md)), và chạy như một service
([`21-systemd-and-services.md`](21-systemd-and-services.md)).

## Practice
1. Chạy `uname -a` và đọc phiên bản kernel của bạn; chạy `ps aux` và chọn
   ba process — với mỗi process, đoán (rồi kiểm chứng bằng `man`/docs) nó
   làm gì.
2. Đọc mười hai file anh em theo thứ tự, rồi quay lại đây và giải thích, mỗi ý
   một câu: vì sao thread rẻ hơn process, syscall thật ra băng qua cái gì,
   vì sao `read()` có thể block, và container cách ly cái gì mà một
   process bình thường không có.
3. Chạy `cat /proc/version` và `cat /proc/cpuinfo | grep -c processor` —
   xác nhận bạn tìm được phiên bản kernel và số core mà không cần công cụ
   GUI.
4. Chạy `strace -f ls 2>&1 | head -30` và, dùng danh sách "vòng đời một chương trình" ở trên, tìm `execve`, việc nạp library (`openat` các file `.so`), và công việc thật đầu tiên (`write`); rồi `ldd $(which ls)` và `cat /proc/self/maps | head` để thấy các library và address space bạn vừa đọc.
5. Không nhìn ghi chú, kể câu chuyện của `./proxy` từ lúc nhấn phím tới exit trong sáu bước, gọi tên syscall hoặc cơ chế kernel trong mỗi bước; đối chiếu với file này.
