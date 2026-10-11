# Recall và Review: Làm cho Kiến thức OS Nhớ Lâu

Đọc một file một lần chỉ tạo ra *cảm giác* là mình hiểu, chưa tạo ra *trí nhớ*. Các chủ đề OS đặc biệt dễ quên, vì
chúng trông như những sự thật rời rạc (fork, inode, cgroup, futex...). Trí nhớ chỉ bám khi chúng được treo vào một
cấu trúc.

File này là bộ retrieval cho `02-linux/`. Nó gồm: một bộ khung, câu hỏi cho từng file, hình để vẽ lại từ trí nhớ, và
các thí nghiệm "dự đoán rồi chạy". File này không dạy gì mới. Nó buộc bạn phải dùng những gì các file khác đã dạy.

Phương pháp giống với
[`01-network/22-recall-and-review.md`](../01-network/22-recall-and-review.md) và
[`00-introduction/03-study-loop.md`](../00-introduction/03-study-loop.md): che lại rồi trả lời rồi kiểm tra, tự suy ra
trước khi đọc lại, và ôn giãn cách ở ngày 1/3/7/21.

## What to learn

### Một câu hỏi tổ chức mọi thứ
*Làm sao để nhiều chương trình dùng chung một máy mà vẫn an toàn?* Mỗi chủ đề là câu trả lời cho một loại tài
nguyên:

| Cần quản lý | Câu trả lời của OS |
|---|---|
| CPU | process và scheduler |
| RAM | virtual memory |
| đĩa | file |
| network card | socket |
| chương trình khác | IPC và signal |
| "ai được làm gì" | user và capability |
| "được dùng bao nhiêu" | rlimit và cgroup |
| "thấy được gì" | namespace |

Khi một sự thật cảm thấy lỏng lẻo, hãy hỏi: *tài nguyên nào đang được nói đến, và ở đây OS đang multiplex, trừu tượng
hóa hay bảo vệ cái gì?* ([`01-fundamentals.md`](01-fundamentals.md))

### Bộ khung: mười lăm sự thật cần giữ trong đầu
1. **Kernel** là code duy nhất chạm vào phần cứng. Mọi thứ khác chạy ở **user space** và chỉ vào được kernel qua
   **syscall**. [`01-fundamentals.md`](01-fundamentals.md), [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)
2. Một syscall tốn một lần đổi mode (và thường kèm một context switch), nên **số lượng syscall quan trọng**. Một
   **context switch** còn làm mất cả CPU cache đã nóng. [`02-hardware-basics.md`](02-hardware-basics.md)
3. CPU thực thi quyền đặc quyền, **MMU** thực thi cách ly bộ nhớ. Một truy cập sai gây ra **page fault**.
   [`02-hardware-basics.md`](02-hardware-basics.md), [`16-memory.md`](16-memory.md)
4. Một **process** = address space riêng + các fd. Một **thread** chia sẻ chúng. **Task** của tokio không phải cả hai.
   [`03-processes-and-threads.md`](03-processes-and-threads.md)
5. `fork` copy (copy-on-write), `exec` thay thế, `wait` reap. Child chưa được thu dọn là **zombie**. Trong container,
   **PID 1** là trường hợp đặc biệt. [`04-process-lifecycle.md`](04-process-lifecycle.md)
6. Gần như mọi thứ là **file descriptor**: một index trỏ tới một kernel object. Một **epoll** chờ được tất cả.
   [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md), [`14-epoll.md`](14-epoll.md)
7. Một file là một **inode**, tên chỉ là entry của directory. `write` chỉ rơi vào **page cache**, chỉ bền vững sau
   `fsync`. Thay thế atomic bằng cách ghi file tạm rồi `rename`.
   [`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)
8. Permission được kiểm tra theo **effective UID/GID**. root bỏ qua kiểm tra. **Capability** chia nhỏ quyền root
   (`CAP_NET_BIND_SERVICE` cho port 443).
   [`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)
9. Bộ nhớ là **ảo**, **lười** (chỉ được cấp khi chạm lần đầu) và **có thể bị thu hồi**. RSS ≠ VSZ. Vượt limit của
   cgroup nghĩa là **OOM kill** (exit 137). [`08-memory-basics.md`](08-memory-basics.md), [`16-memory.md`](16-memory.md)
10. Một blocking syscall làm thread phải đứng chờ. **Event loop** tồn tại để một thread phục vụ nhiều connection.
    **Signal** là sự ngắt từ bên ngoài. [`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md),
    [`17-signals.md`](17-signals.md)
11. Các lựa chọn IPC: pipe, **Unix socket** (truyền fd bằng `SCM_RIGHTS`, nền tảng của hot restart), shared memory
    (nhanh, nhưng bạn tự đồng bộ), và **futex** nằm dưới mọi `Mutex`. [`10-ipc.md`](10-ipc.md)
12. Thời gian **monotonic** dùng cho duration, **wall-clock** dùng cho timestamp của con người. Mọi lần chờ đều cần một
    **timeout**, tốt nhất là một deadline. [`11-time-and-timers.md`](11-time-and-timers.md)
13. Scheduler giữ các **run queue**. Task thiên về I/O có latency tốt. **CPU limit** của cgroup sẽ throttle cả process.
    [`12-cpu-scheduling.md`](12-cpu-scheduling.md)
14. Một **container** = namespace (thấy gì) + cgroup (được dùng bao nhiêu), chạy trên **kernel dùng chung**. Không phải
    là VM. [`13-containers.md`](13-containers.md)
15. Một service có **hợp đồng** với supervisor: chạy ở foreground, log ra stdout, xử lý `SIGTERM`, ở dưới
    `RLIMIT_NOFILE`, báo readiness. [`20-limits-and-proc.md`](20-limits-and-proc.md),
    [`21-systemd-and-services.md`](21-systemd-and-services.md)

### Ngân hàng câu hỏi, theo từng file
Trả lời mỗi câu mà không xem ghi chú, rồi kiểm tra theo link. Câu có dấu sao (*) là những câu mọi người hay sai nhất.

**01 fundamentals** ([`01-fundamentals.md`](01-fundamentals.md))
- OS làm ba việc nào? Linux là một OS hay chỉ là một kernel?
- Kể lại hành trình của `./proxy` từ lúc bấm phím đến lúc exit trong sáu bước. *

**02 hardware** ([`02-hardware-basics.md`](02-hardware-basics.md))
- Ở mức CPU, một syscall khác một lời gọi hàm bình thường ở điểm nào?
- Lần theo một network frame từ dây cho đến khi `read()` của bạn trả về. Các lần copy nằm ở đâu? *
- `us`, `sy`, `si`, `wa`, `st` trong `top` là gì, và bạn kiểm tra cái nào khi một proxy chạy ở 100% CPU?

**03 processes and threads** ([`03-processes-and-threads.md`](03-processes-and-threads.md))
- Thread chia sẻ gì với các thread cùng process, và cái gì là riêng của nó?
- Vì sao một task tokio rẻ hơn một thread? "Hai scheduler chồng lên nhau" nghĩa là gì?

**04 process lifecycle** ([`04-process-lifecycle.md`](04-process-lifecycle.md))
- `fork` và `exec` mỗi cái làm gì, và vì sao chúng là hai lời gọi riêng?
- Zombie là gì, ai tạo ra nó, và exit code 137 nghĩa là gì? *
- Vì sao một proxy chạy làm PID 1 lại bỏ qua `SIGTERM`? Nêu hai cách sửa.

**05 kernel and syscalls** ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md))
- Vì sao ba lần `write` tốn hơn một lần `writev`?
- Nếu hai process cùng có "fd 5", chúng có phải là cùng một thứ không? Khi nào một kernel object thực sự bị đóng? *
- root và kernel mode khác nhau thế nào?

**06 filesystem** ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md))
- Inode lưu gì và không lưu gì? Vì sao `df` có thể báo đầy trong khi `du` báo không? *
- Một `write` thành công có nghĩa là dữ liệu đã nằm trên đĩa không? `fsync` thêm gì?
- Viết các bước cập nhật một config file một cách atomic. Vì sao file tạm phải nằm cùng directory?
- Vì sao mở file rồi `fstat` an toàn hơn `stat` rồi mới mở?

**07 users and capabilities** ([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md))
- Kiểm tra permission dùng UID nào? Bit `x` nghĩa gì trên một directory?
- Làm sao một proxy bind được port 443 mà không chạy bằng root? Kể ba cách. *
- Hạ đặc quyền theo thứ tự nào, và vì sao phải kiểm tra giá trị trả về của mọi lời gọi?

**08 memory basics** ([`08-memory-basics.md`](08-memory-basics.md))
- Vì sao hai process cùng dùng địa chỉ `0x1000` mà không xung đột nhau?
- Sắp xếp register, L1, L3, RAM, SSD, network theo latency, kèm con số xấp xỉ.

**09 blocking I/O and signals** ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md))
- Chuyện gì xảy ra với một thread bên trong một `read` blocking? Vì sao thread-per-connection ngừng scale?
- Vì sao làm việc thật bên trong một signal handler là nguy hiểm?

**10 IPC** ([`10-ipc.md`](10-ipc.md))
- Pipe, Unix socket và shared memory: khi nào dùng cái nào?
- Hot restart giữ listening port luôn mở xuyên suốt việc đổi binary bằng cách nào? *
- Một `Mutex` không bị tranh chấp tốn gì so với một cái bị tranh chấp, và vì sao?

**11 time** ([`11-time-and-timers.md`](11-time-and-timers.md))
- Clock nào dùng cho timeout, clock nào dùng để kiểm tra certificate hết hạn, và nếu đổi chỗ thì cái gì hỏng? *
- Vì sao tokio giữ timer wheel riêng? Khi một future bị `timeout` thì chuyện gì xảy ra với nó?
- Idle timeout và total deadline: mỗi cái ngăn chặn loại tấn công nào?

**12 CPU scheduling** ([`12-cpu-scheduling.md`](12-cpu-scheduling.md))
- Thời gian chờ trong run queue là gì, và vì sao code hiệu quả không loại bỏ được nó?
- Một container `cpus=2` trên host 64 core chạy 64 tokio worker. Chuyện gì xảy ra? *
- Load average cao nhưng CPU thấp: các nguyên nhân khả dĩ là gì?

**13 containers** ([`13-containers.md`](13-containers.md))
- Namespace giới hạn cái gì, và cgroup giới hạn cái gì? Container có kernel riêng không?
- Vì sao `127.0.0.1` bên trong container không tới được service của host?

**14 epoll** ([`14-epoll.md`](14-epoll.md))
- Level-triggered và edge-triggered: với edge-triggered, bạn phải làm khác điều gì? *
- Vì sao không thể làm một file thường thành non-blocking bằng epoll?

**15 io_uring** ([`15-io_uring.md`](15-io_uring.md))
- Mô hình readiness và mô hình completion khác nhau thế nào trong thực tế?

**16 memory** ([`16-memory.md`](16-memory.md))
- Minor fault và major fault khác nhau thế nào? Vì sao các request đầu tiên sau khi khởi động thì chậm hơn?
- `free -h` hiện bộ nhớ "free" rất ít. Đó có phải vấn đề không? Con số nào mới quan trọng? *
- Một cgroup OOM kill trông như thế nào nhìn từ bên ngoài, và cái gì trong proxy làm bộ nhớ nhân lên?

**17 signals** ([`17-signals.md`](17-signals.md))
- Một proxy nên làm gì với `SIGTERM` và với `SIGHUP`? Vì sao không thể xử lý `SIGKILL`?

**18 zero-copy** ([`18-zerocopy.md`](18-zerocopy.md))
- `sendfile` loại bỏ những lần copy nào? Vì sao zero-copy khó áp dụng với TLS?

**19 netfilter** ([`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md))
- Kể tên năm hook. DNAT xảy ra ở đâu, SNAT xảy ra ở đâu?
- `DROP` và `REJECT`: client thấy gì ở mỗi trường hợp? *
- Chuyện gì xảy ra khi conntrack đầy, và triệu chứng nào xuất hiện?

**20 limits and /proc** ([`20-limits-and-proc.md`](20-limits-and-proc.md))
- Một proxy chạm `EMFILE`. Accept loop sẽ thế nào nếu bạn không xử lý gì? Bạn kiểm tra limit *thực sự* đang có hiệu lực
  ở đâu? *
- File `/proc` nào trả lời: có bao nhiêu fd? dùng bao nhiêu bộ nhớ thật? thuộc cgroup nào?

**21 systemd** ([`21-systemd-and-services.md`](21-systemd-and-services.md))
- Mô tả hợp đồng dừng (signal và timeout). Vì sao có `Type=notify`?
- Socket activation cho bạn lợi ích gì về restart và về đặc quyền?

### Vẽ lại từ trí nhớ
1. Ranh giới user/kernel với cánh cửa syscall, và vị trí của epoll, socket, file
   ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)).
2. Đường đi của một packet: NIC, DMA, IRQ/softirq, TCP, socket queue, wake up, `read`
   ([`02-hardware-basics.md`](02-hardware-basics.md)).
3. fd table → open file description → inode, với `dup`/`fork` dùng chung
   ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)).
4. Các state của process và các chuyển đổi giữa chúng (running, runnable, sleeping, zombie)
   ([`12-cpu-scheduling.md`](12-cpu-scheduling.md), [`04-process-lifecycle.md`](04-process-lifecycle.md)).
5. Một virtual address space (code, heap, stack, mmap) ánh xạ tới RAM, page cache và swap
   ([`16-memory.md`](16-memory.md)).
6. Các netfilter hook cùng routing decision
   ([`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md)).
7. Namespace và cgroup bao quanh một process ([`13-containers.md`](13-containers.md)).

### Dự đoán, rồi chạy
Viết dự đoán trước, chạy, rồi ghi lại điều bất ngờ.
1. `ulimit -n 64`, rồi mở 100 connection tới server của bạn. Accept loop làm gì?
2. `rm` một file log mà một process vẫn đang mở. `df` có đổi không? Khi nào?
3. `kill -TERM 1` trong một container, có và không có handler. Chuyện gì xảy ra, và exit code là gì?
4. `strace -c` một HTTP server hello-world. Syscall nào chiếm phần lớn?
5. Ghi 1 GiB có và không có `fsync`. Dự đoán tỷ lệ thời gian.
6. Chạy một chương trình tokio nặng CPU với `--cpus=1` và 8 worker. `cpu.stat` hiện gì?

### Gotcha: OS chỉ là một đống sự thật, cho đến khi bạn kể lại được nó
Phép thử của việc hiểu không phải là nhớ một định nghĩa. Nó là khả năng kể một *câu chuyện*, ví dụ "chuyện gì xảy ra
khi một request tới và một worker đang bị block trên đĩa?", bằng cách dùng bộ khung. Nếu bạn kể được ba câu chuyện như
vậy mà không cần ghi chú, các sự thật sẽ ở lại trong đầu bạn.

## Practice
1. Gập file này lại và viết ra mười lăm sự thật của bộ khung từ trí nhớ. Tự chấm điểm. Lặp lại sau 1, 3 và 7 ngày, và
   ghi điểm vào learning log của bạn.
2. Biến mỗi câu có dấu sao mà bạn đã trả lời sai thành một flashcard, kèm link tới section tương ứng.
3. Không dùng ghi chú, kể thành tiếng câu chuyện "một request tới [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy)
   trong khi một worker thread đang bị block khi đọc một file". Ở mỗi bước, gọi tên syscall, cơ chế kernel và hành
   động của scheduler, đồng thời vẽ các hình số 1, 2 và 4 ở trên.
4. Chạy ba thí nghiệm trong mục "Dự đoán, rồi chạy" và ghi lại mỗi thí nghiệm dạy bạn điều gì mà việc chỉ đọc không dạy
   được.
