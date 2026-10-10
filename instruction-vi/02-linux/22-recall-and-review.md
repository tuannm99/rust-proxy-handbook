# Recall và Review: Làm cho Kiến thức OS Nhớ Lâu

Đọc một file một lần tạo ra *cảm giác* hiểu, không phải *trí nhớ* về nó. Các chủ đề
OS đặc biệt dễ quên vì chúng trông như những sự thật không liên quan (fork, inode,
cgroup, futex...) — trí nhớ chỉ bám khi chúng treo vào một cấu trúc. File này là bộ
retrieval cho `02-linux/`: một bộ khung, câu hỏi cho từng file, hình vẽ để tái tạo từ
trí nhớ, và các thí nghiệm dự đoán-rồi-chạy. Nó không dạy gì mới; nó bắt bạn dùng
những gì các file khác đã dạy. Phương pháp (che-trả lời-kiểm tra, suy ra trước khi đọc
lại, ôn giãn cách ở ngày 1/3/7/21) giống như trong
[`01-network/22-recall-and-review.md`](../01-network/22-recall-and-review.md) và [`00-introduction/03-study-loop.md`](../00-introduction/03-study-loop.md).

## What to learn

### Một câu hỏi tổ chức mọi thứ
*Làm sao nhiều chương trình dùng chung một máy một cách an toàn?* Mỗi chủ đề là một câu
trả lời cho một tài nguyên: CPU -> process và scheduler; RAM -> virtual memory; đĩa ->
file; network card -> socket; chương trình khác -> IPC và signal; "ai được làm gì" ->
user và capability; "được dùng bao nhiêu" -> rlimit và cgroup; "thấy được gì" ->
namespace. Khi một sự thật cảm thấy lỏng lẻo, hãy hỏi: *tài nguyên nào, và OS đang
multiplex, trừu tượng hóa hay bảo vệ cái gì ở đây?* ([`01-fundamentals.md`](01-fundamentals.md))

### Bộ khung: mười lăm sự thật để giữ trong đầu
1. **Kernel** là code duy nhất chạm vào phần cứng; mọi thứ khác chạy ở **user space** và
   chỉ cross kernel qua **syscall**. [`01-fundamentals.md`](01-fundamentals.md), [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)
2. Một syscall tốn một lần đổi mode (và thường là một context switch), nên **số lượng
   syscall quan trọng**; một **context switch** còn tốn cả CPU cache đã ấm.
   [`02-hardware-basics.md`](02-hardware-basics.md)
3. CPU thực thi đặc quyền và **MMU** thực thi cô lập bộ nhớ; một truy cập sai là một
   **page fault**. [`02-hardware-basics.md`](02-hardware-basics.md), [`16-memory.md`](16-memory.md)
4. Một **process** = address space riêng + fd; một **thread** chia sẻ chúng; **task** của
   tokio không phải cả hai. [`03-processes-and-threads.md`](03-processes-and-threads.md)
5. `fork` copy (copy-on-write), `exec` thay thế, `wait` reap; một child chưa được thu
   là **zombie**; trong container **PID 1** đặc biệt. [`04-process-lifecycle.md`](04-process-lifecycle.md)
6. Gần như mọi thứ là **file descriptor**: một index tới một kernel object; một **epoll**
   chờ được tất cả. [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md), [`14-epoll.md`](14-epoll.md)
7. Một file là một **inode**; tên là entry của directory; `write` rơi vào **page cache**,
   chỉ bền sau `fsync`; thay thế atomic bằng write-temp + `rename`.
   [`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)
8. Permission được kiểm tra theo **effective UID/GID**; root bỏ qua; **capability** xẻ nhỏ
   root (`CAP_NET_BIND_SERVICE` cho port 443). [`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)
9. Bộ nhớ là **ảo**, **lười** (allocate ở lần chạm đầu) và **reclaim được**; RSS ≠ VSZ;
   vượt limit của cgroup nghĩa là **OOM kill** (exit 137). [`08-memory-basics.md`](08-memory-basics.md), [`16-memory.md`](16-memory.md)
10. Một blocking syscall đỗ thread lại; **event loop** tồn tại để một thread phục vụ nhiều
    connection; **signal** interrupt từ bên ngoài. [`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md), [`17-signals.md`](17-signals.md)
11. Lựa chọn IPC: pipe, **Unix socket** (truyền được fd bằng `SCM_RIGHTS` -> hot restart),
    shared memory (nhanh, bạn tự đồng bộ), **futex** dưới mọi `Mutex`. [`10-ipc.md`](10-ipc.md)
12. Thời gian **monotonic** cho duration, **wall-clock** cho timestamp của con người; mọi
    lần chờ cần một **timeout** và lý tưởng là một deadline. [`11-time-and-timers.md`](11-time-and-timers.md)
13. Scheduler giữ các **run queue**; task thiên về I/O có latency tốt; **CPU limit** của
    cgroup throttle cả process. [`12-cpu-scheduling.md`](12-cpu-scheduling.md)
14. Một **container** = namespace (thấy gì) + cgroup (được dùng gì) trên một **kernel dùng
    chung**, không phải VM. [`13-containers.md`](13-containers.md)
15. Một service có **hợp đồng** với supervisor: ở foreground, log ra stdout, xử lý `SIGTERM`,
    ở dưới `RLIMIT_NOFILE`, báo readiness. [`20-limits-and-proc.md`](20-limits-and-proc.md), [`21-systemd-and-services.md`](21-systemd-and-services.md)

### Ngân hàng câu hỏi, theo từng file
Trả lời mỗi câu không ghi chú, rồi kiểm tra theo mũi tên. Câu có dấu sao (*) là những
câu mọi người hay sai nhất.

**01 fundamentals** ([`01-fundamentals.md`](01-fundamentals.md))
- OS làm ba việc nào? Linux là một OS hay một kernel?
- Kể `./proxy` từ lúc nhấn phím tới exit trong sáu bước. *

**02 hardware** ([`02-hardware-basics.md`](02-hardware-basics.md))
- Ở mức CPU, một syscall khác một lời gọi hàm bình thường ở chỗ nào?
- Lần theo một network frame từ dây tới khi `read()` của bạn trả về. Các lần copy ở đâu? *
- `us`, `sy`, `si`, `wa`, `st` trong `top` là gì, và bạn kiểm tra cái nào với một proxy ở 100% CPU?

**03 processes and threads** ([`03-processes-and-threads.md`](03-processes-and-threads.md))
- Thread chia sẻ gì với các thread anh em, và cái gì là riêng của nó?
- Vì sao một task tokio rẻ hơn một thread? "Hai scheduler chồng lên nhau" là gì?

**04 process lifecycle** ([`04-process-lifecycle.md`](04-process-lifecycle.md))
- `fork` và `exec` mỗi cái làm gì, và vì sao chúng là hai lời gọi riêng?
- Zombie là gì, ai tạo ra nó, và exit code 137 nghĩa là gì? *
- Vì sao một proxy chạy làm PID 1 bỏ qua `SIGTERM`? Sửa bằng hai cách.

**05 kernel and syscalls** ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md))
- Vì sao ba lần `write` tốn hơn một `writev`?
- Nếu hai process cùng giữ "fd 5", chúng có là cùng một thứ không? Khi nào một object thực sự đóng? *
- Root vs kernel mode: khác nhau thế nào?

**06 filesystem** ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md))
- Inode lưu gì và không lưu gì? Vì sao `df` có thể báo đầy trong khi `du` báo không? *
- Một `write` thành công có nghĩa dữ liệu đã nằm trên đĩa không? `fsync` thêm gì?
- Viết các bước cập nhật một config file một cách atomic. Vì sao file tạm phải ở cùng directory?
- Vì sao open-rồi-`fstat` an toàn hơn `stat`-rồi-open?

**07 users and capabilities** ([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md))
- Kiểm tra permission dùng UID nào? Bit `x` nghĩa gì trên một directory?
- Một proxy bind port 443 mà không chạy bằng root bằng cách nào? Kể ba cách. *
- Hạ đặc quyền theo thứ tự nào, và vì sao phải kiểm tra mọi giá trị trả về?

**08 memory basics** ([`08-memory-basics.md`](08-memory-basics.md))
- Vì sao hai process cùng dùng địa chỉ `0x1000` mà không xung đột?
- Sắp xếp register, L1, L3, RAM, SSD, network theo latency, kèm con số xấp xỉ.

**09 blocking I/O and signals** ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md))
- Chuyện gì xảy ra với một thread bên trong một `read` blocking? Vì sao thread-per-connection ngừng scale?
- Vì sao làm việc thật bên trong một signal handler là nguy hiểm?

**10 IPC** ([`10-ipc.md`](10-ipc.md))
- Pipe vs Unix socket vs shared memory: khi nào dùng cái nào?
- Một hot restart giữ listening port mở xuyên qua việc đổi binary bằng cách nào? *
- Một `Mutex` không tranh chấp tốn gì so với một cái bị tranh chấp, và vì sao?

**11 time** ([`11-time-and-timers.md`](11-time-and-timers.md))
- Clock nào cho timeout, clock nào cho kiểm tra expire certificate, và cái gì hỏng nếu đổi chỗ? *
- Vì sao tokio giữ timer wheel riêng? Drop một future bị `timeout` thì xảy ra gì?
- Idle timeout vs total deadline: mỗi cái ngăn tấn công nào?

**12 CPU scheduling** ([`12-cpu-scheduling.md`](12-cpu-scheduling.md))
- Run-queue wait là gì và vì sao code hiệu quả không loại bỏ được nó?
- Một container `cpus=2` trên host 64 core chạy 64 tokio worker. Chuyện gì xảy ra? *
- Load average cao nhưng CPU thấp: nguyên nhân khả dĩ là gì?

**13 containers** ([`13-containers.md`](13-containers.md))
- Namespace giới hạn gì và cgroup giới hạn gì? Container có kernel riêng không?
- Vì sao `127.0.0.1` bên trong container không tới được service của host?

**14 epoll** ([`14-epoll.md`](14-epoll.md))
- Level- vs edge-triggered: với edge-triggered bạn phải làm khác điều gì? *
- Vì sao một file thường không thể làm non-blocking bằng epoll?

**15 io_uring** ([`15-io_uring.md`](15-io_uring.md))
- Mô hình readiness vs completion: khác biệt thực tế là gì?

**16 memory** ([`16-memory.md`](16-memory.md))
- Minor vs major page fault. Vì sao các request đầu tiên sau khi khởi động chậm hơn?
- `free -h` hiện ít bộ nhớ "free". Đó có phải vấn đề không? Con số nào mới quan trọng? *
- Một cgroup OOM kill nhìn từ bên ngoài ra sao, và cái gì trong proxy nhân bộ nhớ lên?

**17 signals** ([`17-signals.md`](17-signals.md))
- Một proxy nên làm gì với `SIGTERM` vs `SIGHUP`? Vì sao `SIGKILL` không thể xử lý?

**18 zero-copy** ([`18-zerocopy.md`](18-zerocopy.md))
- `sendfile` loại bỏ những lần copy nào? Vì sao zero-copy khó với TLS?

**19 netfilter** ([`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md))
- Kể tên năm hook. DNAT xảy ra ở đâu, và SNAT ở đâu?
- `DROP` vs `REJECT`: client thấy gì ở mỗi cái? *
- Chuyện gì xảy ra khi conntrack đầy, và triệu chứng gì hiện ra?

**20 limits and /proc** ([`20-limits-and-proc.md`](20-limits-and-proc.md))
- Một proxy chạm `EMFILE`. Accept loop sẽ ra sao nếu bạn không làm gì? Bạn kiểm tra limit
  *thực sự* có hiệu lực ở đâu? *
- File `/proc` nào trả lời: bao nhiêu fd? bao nhiêu bộ nhớ thật? cgroup nào?

**21 systemd** ([`21-systemd-and-services.md`](21-systemd-and-services.md))
- Mô tả hợp đồng dừng (signal và timeout). Vì sao `Type=notify` tồn tại?
- Socket activation cho bạn gì về restart và về đặc quyền?

### Vẽ từ trí nhớ
1. Sự phân chia user/kernel với cánh cửa syscall, và nơi epoll, socket, file nằm ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)).
2. Đường đi của một packet: NIC, DMA, IRQ/softirq, TCP, socket queue, wake up, `read` ([`02-hardware-basics.md`](02-hardware-basics.md)).
3. fd table -> open file description -> inode, với `dup`/`fork` chia sẻ ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)).
4. Các state của process và chuyển đổi (running, runnable, sleeping, zombie) ([`12-cpu-scheduling.md`](12-cpu-scheduling.md), [`04-process-lifecycle.md`](04-process-lifecycle.md)).
5. Một virtual address space (code, heap, stack, mmap) ánh xạ tới RAM, page cache và swap ([`16-memory.md`](16-memory.md)).
6. Các netfilter hook với routing decision ([`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md)).
7. Namespace và cgroup quanh một process ([`13-containers.md`](13-containers.md)).

### Dự đoán, rồi chạy
Viết dự đoán trước; chạy; ghi lại điều bất ngờ.
1. `ulimit -n 64`, rồi mở 100 connection tới server của bạn — accept loop làm gì?
2. `rm` một file log mà một process vẫn đang mở — `df` có đổi không? Khi nào?
3. `kill -TERM 1` trong một container có và không có handler — chuyện gì xảy ra, và exit code là gì?
4. `strace -c` một HTTP server hello-world — syscall nào chiếm đa số?
5. Ghi 1 GiB có và không có `fsync` — dự đoán tỉ lệ.
6. Chạy một chương trình tokio nặng CPU với `--cpus=1` và 8 worker — `cpu.stat` hiện gì?

### Gotcha: OS chỉ là một đống sự thật cho tới khi bạn kể lại được nó
Phép thử của việc hiểu không phải là nhớ một định nghĩa mà là kể một *câu chuyện* —
"chuyện gì xảy ra khi một request tới và một worker đang block trên đĩa?" — bằng bộ khung.
Nếu bạn kể được ba câu chuyện như vậy không ghi chú, các sự thật sẽ ở lại.

## Practice
1. Gập file này lại và viết mười lăm sự thật của bộ khung từ trí nhớ; tự chấm điểm, rồi lặp
   lại sau 1, 3 và 7 ngày và ghi điểm vào learning log của bạn.
2. Biến mỗi câu có dấu sao bạn đã sai thành một flashcard kèm link tới section.
3. Không ghi chú, kể thành tiếng câu chuyện "một request tới [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) trong khi
   một worker thread đang block đọc một file", gọi tên syscall, cơ chế kernel và hành động của
   scheduler ở mỗi bước, trong khi vẽ các mục 1, 2 và 4 ở trên.
4. Chạy ba thí nghiệm "Dự đoán, rồi chạy" và ghi lại mỗi cái dạy bạn điều gì mà việc đọc không dạy.
