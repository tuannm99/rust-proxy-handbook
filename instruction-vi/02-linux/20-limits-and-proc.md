# Giới hạn tài nguyên và /proc: Process được phép gì, và cách xem nó đang làm gì

Các trần mà kernel áp lên một process (file mở, process, memory lock, core dump) và filesystem `/proc` qua đó
bạn soi được mọi process đang sống mà không cần debugger. Lỗi production phổ biến nhất của một proxy nặng
connection — "too many open files" — nằm ở đây.

## What to learn

### rlimit: trần theo từng process
Mỗi process có các **resource limit** (`setrlimit`/`getrlimit`, `ulimit` của shell), mỗi cái có một giới hạn **soft**
(thứ kernel đang enforce) và một giới hạn **hard** (trần mà một process không đặc quyền được nâng soft limit lên). Những
cái cắn một network service:

- **`RLIMIT_NOFILE`** (`ulimit -n`) — số fd mở tối đa ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)). Mặc định theo lịch sử là **1024**. Mỗi client
  socket, mỗi upstream socket, mỗi file, instance `epoll`, pipe và timer đều tính. Một proxy với 10.000 client và upstream được
  pool cần hơn 10.000 nhiều. Chạm trần thì `accept()`/`open()`/`socket()` fail với **`EMFILE`**.
- **`RLIMIT_NPROC`** — số process/thread cho mỗi user (ảnh hưởng thread pool).
- **`RLIMIT_CORE`** — kích thước core dump (`0` = không có). Một core file chứa bộ nhớ của process, **gồm cả private key và
  token** ([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)); hãy tắt hoặc kiểm soát chặt.
- **`RLIMIT_MEMLOCK`** — bộ nhớ có thể bị khóa trong RAM (`mlock`, và bộ nhớ ring của io_uring trên kernel cũ hơn —
  [`15-io_uring.md`](15-io_uring.md)).
- **`RLIMIT_STACK`** — kích thước stack của main thread (mặc định 8 MiB); stack của thread được đặt riêng.
- **`RLIMIT_AS`/`RLIMIT_DATA`** — trần address-space; hiếm khi hữu ích với một runtime reserve các vùng ảo lớn — hãy dùng giới hạn
  bộ nhớ của cgroup thay thế ([`13-containers.md`](13-containers.md), [`16-memory.md`](16-memory.md)).

`cat /proc/<pid>/limits` cho thấy các limit *thực tế* của một process đang chạy — luôn kiểm tra cái này thay vì `ulimit` của shell
bạn, vì service được khởi động bởi thứ khác.

### Giới hạn toàn hệ thống và nơi đặt limit
Cũng có các trần toàn kernel: `fs.file-max` (tổng file mở, `/proc/sys/fs/file-nr` hiện mức dùng), `fs.nr_open` (giá trị lớn nhất mà
`RLIMIT_NOFILE` của bất kỳ process nào được đặt), `kernel.pid_max`, `fs.inotify.max_user_watches`, và `/etc/security/limits.conf` theo
user (PAM — chỉ áp dụng cho login session). **Một service khởi động bởi systemd bỏ qua `limits.conf`**; hãy set `LimitNOFILE=` trong
unit ([`21-systemd-and-services.md`](21-systemd-and-services.md)). Container thừa kế limit từ runtime (`docker run --ulimit nofile=65536:65536`, hoặc mặc định của runtime).
"Tôi đã set `ulimit -n` mà không có tác dụng" gần như luôn nghĩa là limit được đặt sai chỗ, điều mà `/proc/<pid>/limits` lộ ra.

### Xử lý EMFILE trong accept loop
Hết fd trong một vòng `accept` rất khó chịu: connection đang chờ vẫn nằm trong accept queue ([`01-network/11-socket.md`](../01-network/11-socket.md)), listening socket
vẫn đọc được, và một vòng lặp ngây thơ **quay ở 100% CPU** retry `accept()` cứ fail mãi. Server tốt (1) nâng `RLIMIT_NOFILE` lúc khởi
động lên hard limit (`setrlimit`, qua crate `rlimit` hoặc `libc`), (2) khi gặp `EMFILE`/`ENFILE` thì log, **lùi lại một chút** (ví dụ sleep
vài ms) thay vì quay vòng, và đôi khi giữ một fd dự phòng để accept-rồi-đóng một connection nhằm xả tải một cách nhẹ nhàng
([`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md)), và (3) chặn số connection đồng thời một cách có chủ đích để limit là một chính sách chứ không phải tai nạn
([`07-security/09-ddos.md`](../07-security/09-ddos.md)). Đếm fd trong monitoring (`ls /proc/<pid>/fd | wc -l` so với limit, [`08-observability/02-metrics.md`](../08-observability/02-metrics.md)) và cảnh báo ở ~80%.
Một độ trôi tăng đều là **leak** (thường là các socket `CLOSE_WAIT`, [`01-network/12-tcp.md`](../01-network/12-tcp.md)).

### /proc: bảng process sống của kernel dưới dạng file
`/proc/<pid>/` phơi mọi thứ kernel biết về một process dưới dạng file đọc được ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)):

| File | Cho thấy |
|---|---|
| `status` | tên, state, PID/PPID, UID, `Threads`, `VmRSS`, `VmSize`, signal mask, capability |
| `cmdline`, `environ`, `exe`, `cwd` | argument, environment, binary, working directory |
| `fd/`, `fdinfo/` | các fd đang mở dưới dạng symlink tới thứ chúng trỏ (`socket:[12345]`, `/path`), kèm offset |
| `limits` | các rlimit đang thực sự có hiệu lực |
| `maps`, `smaps`, `smaps_rollup` | memory map; RSS/PSS/dirty theo từng vùng ([`16-memory.md`](16-memory.md)) |
| `stat`, `sched`, `schedstat` | CPU time, thống kê scheduling (thời gian chờ run-queue, [`12-cpu-scheduling.md`](12-cpu-scheduling.md)) |
| `io` | byte đã đọc/ghi, số syscall |
| `task/<tid>/` | một directory cho mỗi thread |
| `net/tcp`, `net/sockstat` | bảng socket của network namespace của process (thứ `ss` đọc) |
| `cgroup`, `ns/` | cgroup và namespace nào ([`13-containers.md`](13-containers.md)) |

Toàn hệ thống: `/proc/meminfo`, `/proc/loadavg`, `/proc/stat`, `/proc/interrupts`, `/proc/softirqs`, `/proc/net/snmp` (counter TCP/IP —
retransmit, reset — `nstat` đọc nó), `/proc/sys/` (các sysctl, [`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md)), và `/proc/pressure/*` (PSI). `/sys` phản chiếu
state của device và cgroup ([`13-containers.md`](13-containers.md)).

### Khớp một socket fd với một connection
`ls -l /proc/<pid>/fd` hiện các entry `socket:[inode]`; `ss -tnp` cho biết socket (và process) nào thuộc connection nào; inode nối hai thứ đó
(`ss -e` in inode). Nên khi `lsof -p <pid>` hiện 40.000 socket bạn có thể nhóm chúng theo state và peer (`ss -tn state close-wait`) và quyết
định đó là leak hay là tải.

### Hộp công cụ quan sát, theo câu hỏi
- *Nó đang gọi syscall nào, và cái nào fail?* `strace -f -p <pid>` (`-c` tóm tắt, `-e trace=network`); thấy trực tiếp `EMFILE`, `EAGAIN`, `ECONNRESET`
  ([`12-testing/05-debugging.md`](../12-testing/05-debugging.md)). Đắt trên một process nóng — nó tạm dừng process ở mỗi syscall; trên production ưu tiên `perf trace` hoặc eBPF
  ([`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md)).
- *CPU đang đi đâu?* `perf top`, `perf record -g` + flamegraph ([`08-observability/04-profiling.md`](../08-observability/04-profiling.md)); `pidstat -t 1` theo thread.
- *Ai đang mở file/port này?* `lsof -i :8080`, `ss -tlnp`, `fuser`.
- *Bộ nhớ có tăng không?* `/proc/<pid>/status` `VmRSS`, `smaps_rollup`, heap profiler ([`16-memory.md`](16-memory.md)).
- *Disk hay network bão hòa?* `iostat -x 1`, `sar -n DEV 1`, `ip -s link`.
- *Kernel đã phàn nàn gì?* `dmesg -T`, `journalctl -k` — OOM kill, conntrack đầy, segfault, lỗi phần cứng.

### Gotcha: limit được thừa kế, và limit đã đổi không có hiệu lực hồi tố
Một child thừa kế rlimit của parent lúc `fork`, nên một proxy khởi động từ shell của bạn có limit của shell, và một proxy khởi động bởi
supervisor có limit của supervisor. Nâng limit trong unit file chỉ ảnh hưởng process **mới** — hãy restart service và xác nhận trong
`/proc/<pid>/limits`. Cũng vậy, `LimitNOFILE=infinity` của unit có thể ánh xạ thành một con số khổng lồ và làm hỏng vài chương trình định
cỡ mảng từ nó; hãy đặt một giá trị cụ thể (ví dụ 1048576) và kiểm tra.

## Practice

Hãy làm theo thứ tự.

1. Chạy `ulimit -n`, `ulimit -Hn`, `cat /proc/self/limits`, và `cat /proc/sys/fs/file-nr`. **Done when** bạn giải thích được soft vs hard
   và mỗi con số nghĩa là gì.
2. Gây EMFILE: `ulimit -n 64`, chạy [`labs/00-tcp-server`](../../labs/00-tcp-server), và mở nhiều hơn 64 connection (một vòng `nc`, hoặc `wrk`). **Done when**
   `strace -f -e trace=accept4` hiện `EMFILE`, bạn đã thấy bằng `top` rằng server quay vòng hay lùi lại, và bạn đã làm nó lùi lại thay vì
   quay vòng.
3. Nâng limit bên trong process lúc khởi động bằng `setrlimit` (crate `rlimit`). **Done when** `cat /proc/<pid>/limits` hiện giá trị đã nâng.
4. Khởi động một service qua một systemd unit có và không có `LimitNOFILE=65536` ([`21-systemd-and-services.md`](21-systemd-and-services.md)). **Done when** `/proc/<pid>/limits`
   hiện cả hai giá trị và bạn nói được vì sao `ulimit -n` trong shell của bạn không liên quan.
5. Với [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) đang chạy của bạn, viết một `proxy-vitals.sh` in một dòng mỗi giây: số thread và `VmRSS` từ
   `/proc/<pid>/status`, số fd từ `/proc/<pid>/fd`, các field chính của `smaps_rollup`, và counter `io`. **Done when** các con số di chuyển hợp lý
   dưới một load test và khớp với `top`/`ss`.
6. Cố tình leak fd trong một handler scratch (quên drop socket). **Done when** bạn đã xem `ls /proc/<pid>/fd | wc -l` tăng dưới tải và xác định được,
   bằng `ss -tn state close-wait` và `ls -l /proc/<pid>/fd`, thứ gì đã bị leak.
7. Bật core dump (`ulimit -c unlimited`; kiểm tra `cat /proc/sys/kernel/core_pattern`), làm crash một chương trình scratch bằng `kill -SEGV`, và mở core bằng
   `rust-gdb`. **Done when** `bt` in ra một backtrace và bạn nêu được dữ liệu nhạy cảm nào một core của proxy của bạn sẽ chứa.
