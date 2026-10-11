# Linux/OS Fundamentals

Đây là bài đầu tiên của lộ trình OS, và mọi file sau đều dựa vào nó. Bài không giả
định bạn biết gì trước. Nếu bạn đã biết hệ điều hành làm gì, chỉ cần lướt qua trong
khoảng hai mươi phút. Nếu chưa, hãy đọc hai lần rồi làm Practice trước khi đi tiếp.

Một OS trông như một trăm chủ đề rời rạc (process, memory, file, signal, container).
Chúng chỉ trở nên liên quan với nhau khi bạn thấy tất cả đều là câu trả lời cho cùng
một câu hỏi: *làm sao để nhiều chương trình dùng chung một máy mà vẫn an toàn?* Vì vậy
file này đưa cho bạn câu hỏi đó và tấm bản đồ trước.

## What to learn

### Hệ điều hành để làm gì
Một máy tính có vài tài nguyên vật lý: core CPU, RAM, đĩa, network card, và một chiếc
đồng hồ. Nhiều chương trình muốn dùng chúng cùng lúc, và không chương trình nào được
phá chương trình khác. **Hệ điều hành** làm ba việc:

1. **Multiplex** từng tài nguyên giữa nhiều chương trình. Mỗi chương trình có cảm giác
   như đang có CPU riêng và bộ nhớ riêng.
2. **Trừu tượng hóa** phần cứng thành những khái niệm đơn giản và thống nhất. Nhờ vậy
   bạn không bao giờ phải lập trình trực tiếp disk controller hay network card.
3. **Bảo vệ**: không cho một chương trình đọc hoặc làm hỏng bộ nhớ của chương trình
   khác, hay của chính kernel.

Mỗi chủ đề trong thư mục này tương ứng với một hàng của bảng sau:

| Phần cứng | OS biến nó thành | Bạn dùng qua | Đọc |
|---|---|---|---|
| CPU | **process / thread**, được schedule chạy luân phiên | `fork`, `clone`, `exec` | [`03-processes-and-threads.md`](03-processes-and-threads.md), [`04-process-lifecycle.md`](04-process-lifecycle.md), [`12-cpu-scheduling.md`](12-cpu-scheduling.md) |
| RAM | **virtual memory**: mỗi process có address space riêng | `mmap`, `brk`, pointer thông thường | [`08-memory-basics.md`](08-memory-basics.md), [`16-memory.md`](16-memory.md) |
| Đĩa | **file** và directory | `open`, `read`, `write` | [`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md) |
| Network card | **socket** | `socket`, `bind`, `connect` | [`01-network/11-socket.md`](../01-network/11-socket.md) |
| Đồng hồ | thời gian và timer | `clock_gettime`, `timerfd` | [`11-time-and-timers.md`](11-time-and-timers.md) |
| Các chương trình khác | pipe, signal, shared memory | `pipe`, `kill`, `mmap` | [`10-ipc.md`](10-ipc.md), [`17-signals.md`](17-signals.md) |
| Ai được làm gì | user, permission | `chmod`, capability | [`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md) |
| Giới hạn và cách ly | rlimit, **namespace, cgroup** | `setrlimit`, container | [`20-limits-and-proc.md`](20-limits-and-proc.md), [`13-containers.md`](13-containers.md) |

### Stack phần mềm, và "Linux" nghĩa là gì
Từ dưới lên, stack gồm: **phần cứng**, **kernel**, **system library** (`libc` bọc các
syscall thành những hàm như `read()`), và **chương trình** (shell, `curl`, proxy của
bạn).

Nói cho chính xác thì **Linux chỉ là kernel**. Một *distribution* (Ubuntu, Debian,
Alpine) gói kernel đó cùng `libc`, một package manager và hàng nghìn chương trình khác.
Điều này giải thích vài khác biệt thực tế:

- Alpine dùng `musl` thay vì `glibc`, nên DNS và một số hành vi khác không giống nhau
  ([`01-network/14-dns.md`](../01-network/14-dns.md)).
- Một container mang theo userland riêng, nhưng dùng chung kernel với host
  ([`13-containers.md`](13-containers.md)).
- `uname -r` hiện phiên bản *kernel*. `cat /etc/os-release` hiện *distro*.

### Vòng đời một chương trình, từ `./proxy` đến lúc exit
Đây là câu chuyện mà phần còn lại của thư mục sẽ phóng to vào từng đoạn:

1. Bạn gõ `./proxy`. **Shell** (cũng chỉ là một chương trình bình thường) `fork` một
   bản sao của chính nó, và process con `exec` chương trình `proxy`
   ([`04-process-lifecycle.md`](04-process-lifecycle.md)).
2. **Kernel** đọc file thực thi, dựng một **address space** mới riêng cho nó, và map
   code của chương trình vào ([`08-memory-basics.md`](08-memory-basics.md)). Sau đó nó
   cho một thread vào trạng thái runnable.
3. **Dynamic linker** load các shared library (`ldd ./proxy` liệt kê chúng). Rồi `main`
   chạy. Runtime của Rust và tokio khởi động các worker **thread**.
4. Mỗi khi cần làm việc thật, chương trình gọi **syscall**: `socket`, `bind`,
   `epoll_wait`, `read`, `write`. Mỗi syscall đi vào kernel rồi quay lại
   ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)). Kết quả bạn nhận là các
   **file descriptor**.
5. Khi phải chờ I/O, thread **block** và scheduler chạy thứ khác
   ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)). Khi dữ liệu tới,
   một **interrupt** wake up nó ([`02-hardware-basics.md`](02-hardware-basics.md)).
6. Một **signal** (`SIGTERM`) yêu cầu nó dừng. Nó drain các việc đang làm rồi gọi
   `exit` ([`17-signals.md`](17-signals.md)). Kernel thu hồi bộ nhớ và fd của nó, và
   parent gọi `wait` để lấy exit status
   ([`04-process-lifecycle.md`](04-process-lifecycle.md)).

Có ba ý tưởng giải thích phần lớn những gì còn lại, nên hãy nhớ kỹ:

1. User space và kernel space tách biệt nhau, và cánh cửa duy nhất giữa chúng là
   syscall.
2. Gần như mọi thứ (file, socket, pipe, timer) đều là một **file descriptor**. Nhờ vậy
   chỉ cần một cơ chế (`epoll`) là chờ được tất cả.
3. Kernel *multiplex* tài nguyên. Mỗi process có cảm giác như đang có CPU và bộ nhớ
   riêng. Đó là một ảo giác do OS duy trì, và nó có chi phí (context switch, page
   fault) mà bạn đo được.

### Kernel tồn tại để làm gì
**Kernel** là chương trình duy nhất trên máy được nói chuyện trực tiếp với hardware,
quản lý memory cho mọi process, và đảm bảo các process bị cách ly với nhau. Mọi thứ
khác, kể cả proxy và shell của bạn, chạy trong **user space**. Chúng không được truy
cập hardware trực tiếp và không thể động vào memory của process khác.

Gần như mọi chủ đề trong [`02-linux/`](.) là hệ quả của đúng ranh giới này:
- chi phí mỗi lần vượt qua ranh giới ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)),
- ranh giới này cách ly những gì
  ([`03-processes-and-threads.md`](03-processes-and-threads.md),
  [`13-containers.md`](13-containers.md)),
- và chuyện gì xảy ra khi kernel cần ngắt chương trình của bạn, thay vì chờ được hỏi
  ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)).

### Mười hai mảnh ghép, và mỗi mảnh nằm ở đâu
Đọc theo thứ tự dưới đây. Mỗi mảnh dựa vào các mảnh trước nó.

- **[`02-hardware-basics.md`](02-hardware-basics.md)**: chính cỗ máy. Gồm privilege
  level của CPU, MMU, interrupt, DMA, và đường đi của một packet từ network card đến
  lệnh `read()` của bạn.
- **[`03-processes-and-threads.md`](03-processes-and-threads.md)**: process và thread
  thực chất là gì, vì sao thread rẻ hơn, và vì sao việc chia sẻ memory giữa các thread
  là lý do [`03-rust/04-sync.md`](../03-rust/04-sync.md) tồn tại.
- **[`04-process-lifecycle.md`](04-process-lifecycle.md)**: `fork`, `exec`, `wait`,
  zombie, orphan, và vì sao PID 1 trong container là trường hợp đặc biệt.
- **[`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)**: ranh giới giữa user
  space và kernel space, một syscall tốn bao nhiêu, và file descriptor, tức cái handle
  dạng số nguyên mà mọi thứ trong [`14-epoll.md`](14-epoll.md) được xây quanh.
- **[`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)**: inode, path, page cache,
  `write` đảm bảo gì và không đảm bảo gì, và cách thay một file một cách atomic.
- **[`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)**:
  kernel xem một process là ai, permission bit, capability, và cách bind port 443 mà
  không cần root.
- **[`08-memory-basics.md`](08-memory-basics.md)**: kiến thức tối thiểu để phần mở đầu
  của [`16-memory.md`](16-memory.md) nghe quen thay vì mới hoàn toàn, cộng thêm vị trí
  của RAM so với cache và disk.
- **[`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)**: vì sao một
  syscall có thể block cả thread, vì sao event loop ra đời để thay thế, và signal là gì
  (một sự ngắt từ bên ngoài vào luồng thực thi bình thường, không phải một giá trị trả
  về).
- **[`10-ipc.md`](10-ipc.md)**: pipe, Unix socket, truyền file descriptor giữa các
  process, shared memory, và cách một `Mutex` thực sự chờ (`futex`).
- **[`11-time-and-timers.md`](11-time-and-timers.md)**: monotonic time so với wall-clock
  time, timer và timeout hoạt động thế nào, và vì sao mọi chỗ chờ trong proxy đều cần
  deadline.
- **[`12-cpu-scheduling.md`](12-cpu-scheduling.md)**: run queue, priority, affinity,
  load average, và việc CPU limit trong container bị throttle.
- **[`13-containers.md`](13-containers.md)**: container thực chất là gì. Đó là các
  process bị cách ly, chạy trên cùng một kernel dùng chung, không phải một máy ảo thu
  nhỏ. Từ [`09-architecture/`](../09-architecture) trở đi, Kubernetes, pod và cgroup
  được nhắc liên tục nhưng chưa được định nghĩa ở đâu.

Hãy đọc theo đúng thứ tự trên một lần. Sau đó, coi mỗi file là một chỗ tra cứu độc
lập. Các file về cơ chế kernel ở phía sau ([`14-epoll.md`](14-epoll.md) trở đi) còn có
phần vận hành: Linux networking và netfilter
([`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md)), giới
hạn tài nguyên và `/proc` ([`20-limits-and-proc.md`](20-limits-and-proc.md)), và chạy
như một service ([`21-systemd-and-services.md`](21-systemd-and-services.md)).

## Practice
1. Chạy `uname -a` và đọc phiên bản kernel của bạn. Chạy `ps aux` và chọn ba process.
   Với mỗi process, đoán xem nó làm gì, rồi kiểm tra lại bằng `man` hoặc docs.
2. Đọc mười hai file ở trên theo thứ tự, rồi quay lại đây. Giải thích mỗi ý bằng một
   câu: vì sao thread rẻ hơn process, syscall thực sự vượt qua ranh giới nào, vì sao
   `read()` có thể block, và container cách ly những gì mà một process bình thường
   không có.
3. Chạy `cat /proc/version` và `cat /proc/cpuinfo | grep -c processor`. Xác nhận bạn tìm
   được phiên bản kernel và số core mà không cần công cụ GUI.
4. Chạy `strace -f ls 2>&1 | head -30`. Dựa vào danh sách "vòng đời một chương trình"
   ở trên, tìm: `execve`, bước load library (các lệnh `openat` mở file `.so`), và việc
   thật đầu tiên (`write`). Sau đó chạy `ldd $(which ls)` và
   `cat /proc/self/maps | head` để thấy các library và address space bạn vừa đọc.
5. Không nhìn ghi chú, kể lại câu chuyện của `./proxy` từ lúc bấm phím đến lúc exit
   trong sáu bước. Mỗi bước gọi tên syscall hoặc cơ chế kernel liên quan. Sau đó đối
   chiếu với file này.
