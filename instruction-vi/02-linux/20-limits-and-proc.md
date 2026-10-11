# Giới hạn tài nguyên và /proc: Process được phép làm gì, và cách xem nó đang làm gì

File này nói về các trần mà kernel áp lên một process (số file mở, số process, bộ nhớ bị lock, core dump), và
filesystem `/proc`, qua đó bạn soi được mọi process đang chạy mà không cần debugger. Lỗi production phổ biến nhất
của một proxy phải giữ nhiều connection, "too many open files", nằm ở đây.

## What to learn

### rlimit: trần cho từng process
Mỗi process có các **resource limit** (`setrlimit`/`getrlimit`, hoặc `ulimit` trong shell). Mỗi limit có hai giá
trị:
- Giới hạn **soft**: giá trị mà kernel đang thực sự áp dụng.
- Giới hạn **hard**: mức trần mà một process không đặc quyền được phép nâng soft limit lên.

Những limit hay gây rắc rối cho một network service:

- **`RLIMIT_NOFILE`** (`ulimit -n`): số fd tối đa được mở ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)).
  Mặc định theo lịch sử là **1024**. Mỗi client socket, mỗi upstream socket, mỗi file, mỗi instance `epoll`, pipe
  và timer đều tính. Một proxy có 10.000 client và các upstream được pool cần nhiều hơn 10.000 rất nhiều. Khi chạm
  trần, `accept()`/`open()`/`socket()` fail với **`EMFILE`**.
- **`RLIMIT_NPROC`**: số process/thread cho mỗi user (ảnh hưởng đến thread pool).
- **`RLIMIT_CORE`**: kích thước core dump (`0` = không tạo). Một core file chứa bộ nhớ của process, **gồm cả private
  key và token** ([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)). Hãy tắt nó, hoặc
  kiểm soát thật chặt.
- **`RLIMIT_MEMLOCK`**: lượng bộ nhớ có thể bị lock trong RAM (`mlock`, và bộ nhớ ring của io_uring trên kernel cũ
  hơn, [`15-io_uring.md`](15-io_uring.md)).
- **`RLIMIT_STACK`**: kích thước stack của main thread (mặc định 8 MiB). Stack của các thread khác được đặt riêng.
- **`RLIMIT_AS`/`RLIMIT_DATA`**: trần address space. Hiếm khi hữu ích với một runtime vốn reserve các vùng ảo lớn.
  Hãy dùng giới hạn bộ nhớ của cgroup thay thế ([`13-containers.md`](13-containers.md),
  [`16-memory.md`](16-memory.md)).

`cat /proc/<pid>/limits` cho thấy các limit *thực tế* của một process đang chạy. Hãy luôn kiểm tra file này thay vì
`ulimit` trong shell của bạn, vì service thường được khởi động bởi một thứ khác.

### Giới hạn toàn hệ thống và nơi đặt limit
Cũng có các trần áp dụng cho cả kernel:
- `fs.file-max`: tổng số file mở. `/proc/sys/fs/file-nr` hiện mức đang dùng.
- `fs.nr_open`: giá trị lớn nhất mà `RLIMIT_NOFILE` của bất kỳ process nào được đặt.
- `kernel.pid_max`.
- `fs.inotify.max_user_watches`.
- `/etc/security/limits.conf` theo từng user (qua PAM, chỉ áp dụng cho login session).

**Một service được systemd khởi động sẽ bỏ qua `limits.conf`.** Hãy đặt `LimitNOFILE=` trong unit
([`21-systemd-and-services.md`](21-systemd-and-services.md)). Container thừa kế limit từ runtime
(`docker run --ulimit nofile=65536:65536`, hoặc mặc định của runtime).

Câu "tôi đã đặt `ulimit -n` mà không có tác dụng" gần như luôn nghĩa là limit được đặt sai chỗ. `/proc/<pid>/limits`
sẽ cho bạn thấy điều đó.

### Xử lý EMFILE trong accept loop
Hết fd bên trong vòng `accept` rất khó chịu. Các connection đang chờ vẫn nằm trong accept queue
([`01-network/11-socket.md`](../01-network/11-socket.md)), listening socket vẫn đọc được, và một vòng lặp ngây thơ sẽ
**quay ở 100% CPU** để retry `accept()` cứ fail mãi.

Một server tốt làm ba việc:
1. Lúc khởi động, nâng `RLIMIT_NOFILE` lên hard limit (`setrlimit`, qua crate `rlimit` hoặc `libc`).
2. Khi gặp `EMFILE`/`ENFILE`, log lại và **lùi lại một chút** (ví dụ sleep vài ms) thay vì quay vòng. Đôi khi giữ sẵn
   một fd dự phòng, để accept rồi đóng ngay một connection, nhằm xả tải một cách nhẹ nhàng
   ([`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md)).
3. Đặt trần số connection đồng thời một cách có chủ đích, để limit là một chính sách chứ không phải tai nạn
   ([`07-security/09-ddos.md`](../07-security/09-ddos.md)).

Hãy đếm số fd trong monitoring (`ls /proc/<pid>/fd | wc -l` so với limit,
[`08-observability/02-metrics.md`](../08-observability/02-metrics.md)) và cảnh báo ở khoảng 80%. Nếu số fd tăng đều
đặn, đó là **leak** (thường là các socket `CLOSE_WAIT`, [`01-network/12-tcp.md`](../01-network/12-tcp.md)).

### /proc: bảng process đang sống của kernel, trình bày dưới dạng file
`/proc/<pid>/` phơi ra mọi thứ kernel biết về một process dưới dạng các file đọc được
([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)):

| File | Cho thấy |
|---|---|
| `status` | tên, state, PID/PPID, UID, `Threads`, `VmRSS`, `VmSize`, signal mask, capability |
| `cmdline`, `environ`, `exe`, `cwd` | argument, environment, binary, working directory |
| `fd/`, `fdinfo/` | các fd đang mở, dưới dạng symlink tới thứ chúng trỏ đến (`socket:[12345]`, `/path`), kèm offset |
| `limits` | các rlimit đang thực sự có hiệu lực |
| `maps`, `smaps`, `smaps_rollup` | memory map; RSS/PSS/dirty theo từng vùng ([`16-memory.md`](16-memory.md)) |
| `stat`, `sched`, `schedstat` | CPU time, thống kê scheduling (thời gian chờ trong run queue, [`12-cpu-scheduling.md`](12-cpu-scheduling.md)) |
| `io` | số byte đã đọc/ghi, số syscall |
| `task/<tid>/` | một directory cho mỗi thread |
| `net/tcp`, `net/sockstat` | bảng socket của network namespace của process (thứ mà `ss` đọc) |
| `cgroup`, `ns/` | thuộc cgroup và namespace nào ([`13-containers.md`](13-containers.md)) |

Các file toàn hệ thống:
- `/proc/meminfo`, `/proc/loadavg`, `/proc/stat`, `/proc/interrupts`, `/proc/softirqs`.
- `/proc/net/snmp`: counter TCP/IP như retransmit, reset. `nstat` đọc file này.
- `/proc/sys/`: các sysctl ([`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md)).
- `/proc/pressure/*`: PSI.

`/sys` phản chiếu trạng thái của device và cgroup ([`13-containers.md`](13-containers.md)).

### Khớp một socket fd với một connection
`ls -l /proc/<pid>/fd` hiện các entry `socket:[inode]`. `ss -tnp` cho biết socket nào (và process nào) thuộc
connection nào. Inode là chỗ nối hai thứ đó (`ss -e` in ra inode).

Vì vậy, khi `lsof -p <pid>` hiện 40.000 socket, bạn có thể nhóm chúng theo state và peer
(`ss -tn state close-wait`) để quyết định đó là leak hay chỉ là tải bình thường.

### Bộ công cụ quan sát, sắp xếp theo câu hỏi
- *Nó đang gọi syscall nào, và cái nào fail?* `strace -f -p <pid>` (`-c` để tóm tắt, `-e trace=network`). Bạn thấy
  trực tiếp `EMFILE`, `EAGAIN`, `ECONNRESET` ([`12-testing/05-debugging.md`](../12-testing/05-debugging.md)). Công cụ
  này đắt trên một process đang bận, vì nó tạm dừng process ở mỗi syscall. Trên production, hãy ưu tiên `perf trace`
  hoặc eBPF ([`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md)).
- *CPU đang đi đâu?* `perf top`, `perf record -g` + flamegraph
  ([`08-observability/04-profiling.md`](../08-observability/04-profiling.md)); `pidstat -t 1` để xem theo thread.
- *Ai đang mở file/port này?* `lsof -i :8080`, `ss -tlnp`, `fuser`.
- *Bộ nhớ có đang tăng không?* `VmRSS` trong `/proc/<pid>/status`, `smaps_rollup`, heap profiler
  ([`16-memory.md`](16-memory.md)).
- *Disk hay network có bị bão hòa không?* `iostat -x 1`, `sar -n DEV 1`, `ip -s link`.
- *Kernel đã phàn nàn gì?* `dmesg -T`, `journalctl -k`. Ở đây có OOM kill, conntrack đầy, segfault, lỗi phần cứng.

### Gotcha: limit được thừa kế, và đổi limit không có hiệu lực hồi tố
Một child thừa kế rlimit của parent lúc `fork`. Vì vậy, một proxy khởi động từ shell của bạn có limit của shell, còn
một proxy khởi động bởi supervisor có limit của supervisor. Nâng limit trong unit file chỉ ảnh hưởng đến process
**mới**. Hãy restart service và xác nhận lại trong `/proc/<pid>/limits`.

Ngoài ra, `LimitNOFILE=infinity` trong unit có thể được ánh xạ thành một con số khổng lồ, làm hỏng một số chương
trình vì chúng tạo mảng theo kích thước đó. Hãy đặt một giá trị cụ thể (ví dụ 1048576) và kiểm tra lại.

## Practice

Hãy làm theo thứ tự.

1. Chạy `ulimit -n`, `ulimit -Hn`, `cat /proc/self/limits` và `cat /proc/sys/fs/file-nr`. **Done when** bạn giải
   thích được soft và hard khác nhau thế nào, và ý nghĩa của từng con số.
2. Gây ra EMFILE: chạy `ulimit -n 64`, rồi chạy [`labs/00-tcp-server`](../../labs/00-tcp-server) và mở nhiều hơn 64
   connection (một vòng lặp `nc`, hoặc `wrk`). **Done when** `strace -f -e trace=accept4` hiện `EMFILE`, bạn thấy
   bằng `top` xem server đang quay vòng hay đang lùi lại, và bạn đã sửa để nó lùi lại thay vì quay vòng.
3. Nâng limit bên trong process lúc khởi động bằng `setrlimit` (crate `rlimit`). **Done when**
   `cat /proc/<pid>/limits` hiện giá trị đã nâng.
4. Khởi động một service qua một systemd unit, một lần có và một lần không có `LimitNOFILE=65536`
   ([`21-systemd-and-services.md`](21-systemd-and-services.md)). **Done when** `/proc/<pid>/limits` hiện cả hai giá
   trị, và bạn giải thích được vì sao `ulimit -n` trong shell của bạn không liên quan.
5. Với [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) đang chạy, viết một script `proxy-vitals.sh` in một dòng
   mỗi giây gồm: số thread và `VmRSS` từ `/proc/<pid>/status`, số fd từ `/proc/<pid>/fd`, các field chính của
   `smaps_rollup`, và counter `io`. **Done when** các con số thay đổi hợp lý dưới một load test và khớp với `top`/`ss`.
6. Cố tình leak fd trong một handler scratch (quên drop socket). **Done when** bạn đã xem `ls /proc/<pid>/fd | wc -l`
   tăng dần dưới tải, và xác định được thứ bị leak bằng `ss -tn state close-wait` và `ls -l /proc/<pid>/fd`.
7. Bật core dump (`ulimit -c unlimited`; kiểm tra `cat /proc/sys/kernel/core_pattern`), làm crash một chương trình
   scratch bằng `kill -SEGV`, rồi mở core bằng `rust-gdb`. **Done when** `bt` in ra một backtrace, và bạn nêu được dữ
   liệu nhạy cảm nào mà một core của proxy sẽ chứa.

---

<!-- nav -->
← Trước: [`02-linux/19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md) · [Mục lục](00-README.md) · Tiếp theo: [`02-linux/21-systemd-and-services.md`](21-systemd-and-services.md) →
