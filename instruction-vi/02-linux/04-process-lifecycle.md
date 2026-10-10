# Vòng đời Process: fork, exec, wait, Zombie, và PID 1

Một process được sinh ra, bị thay thế, được giám sát và được thu dọn như thế nào — và vì
sao hành vi của proxy khi là con của một shell, systemd hay container runtime lại phụ
thuộc vào nó. Xây trên [`03-processes-and-threads.md`](03-processes-and-threads.md).

## What to learn

### fork: nhân bản process
`fork()` tạo một **child** gần như là bản sao y hệt của caller: cùng code, cùng nội dung
bộ nhớ (copy-on-write, nên việc copy là lười và rẻ — [`16-memory.md`](16-memory.md)), cùng **file
descriptor đang mở** (cả hai process giờ cùng trỏ vào *cùng* các kernel object;
[`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)). Nó trả về **hai lần**: PID của child ở parent, và `0` ở
child. Từ đó chúng rẽ nhánh. Chi tiết chia sẻ fd rất quan trọng: một listening socket
được thừa kế qua `fork` là cách cổ nhất để chạy nhiều worker trên một port, và là lý do
một socket "đã đóng" có thể vẫn mở nếu một child bị quên vẫn giữ nó.

```rust
// hình dạng thô, qua libc (code scratch, không thuộc workspace)
let pid = unsafe { libc::fork() };
match pid {
    -1 => /* lỗi */ {},
    0  => /* child: pid là 0 ở đây */ {},
    _  => /* parent: pid là id của child */ {},
}
```
Code Rust trong một process đa luồng (mọi chương trình tokio) không được `fork` rồi tiếp
tục chạy code Rust trong child: chỉ thread gọi fork sống sót ở đó, và các lock do thread
khác giữ sẽ bị khóa vĩnh viễn. Vì thế mới có lời gọi tiếp theo.

### exec: thay thế chương trình
`execve(path, argv, envp)` **thay thế** process image hiện tại bằng một chương trình mới:
cùng PID, cùng fd đang mở (trừ khi đánh dấu **close-on-exec**), bộ nhớ hoàn toàn mới.
`fork` + `exec` là cách mọi shell chạy một lệnh và cách `std::process::Command` spawn một
child (Rust dùng `posix_spawn`/`fork+exec` bên trong, một cách an toàn). Các đối số
(`argv`) và **environment** (`envp`) được thừa kế hoặc đặt tại thời điểm này; environment
là cách cấu hình như `RUST_LOG` đến được một process.

**Gotcha: fd rò rỉ qua exec.** Một fd được child exec thừa kế mà bạn không định chia sẻ
(một file bí mật, một listening socket) là bug kinh điển. std của Rust mở file và socket với
`O_CLOEXEC` theo mặc định; các lời gọi `libc::socket` thô cần cờ này (`SOCK_CLOEXEC`) một
cách tường minh — trừ khi bạn *muốn* thừa kế, như trong một hot restart trao listening socket
cho binary mới ([`09-architecture/05-rolling-restart.md`](../09-architecture/05-rolling-restart.md)).

### exit và wait: zombie
Một process kết thúc qua `exit(code)`, return từ `main`, hoặc một signal gây chết
([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)). Kernel giải phóng bộ nhớ và đóng các fd của nó — nhưng giữ
lại một **entry nhỏ trong process table** chứa exit status cho tới khi parent thu nó bằng
`wait()`/`waitpid()`. Một process đã xong nhưng chưa được wait là **zombie** (`Z` trong
`ps`): nó không dùng CPU hay bộ nhớ, chỉ một PID. Một parent không bao giờ wait sẽ làm rò
PID cho tới khi `fork` fail với `EAGAIN`. Exit status mã hóa hoặc exit code (0 = thành công
theo quy ước) hoặc "bị giết bởi signal N" — `128+N` trong shell, nên exit code 137 nghĩa là
`SIGKILL` (thường là OOM killer, [`16-memory.md`](16-memory.md)) và 143 nghĩa là `SIGTERM`.

```rust
let mut child = std::process::Command::new("sleep").arg("1").spawn()?;
let status = child.wait()?;           // thu dọn zombie; status.code() / .signal()
```

### Orphan và PID 1
Nếu parent chết trước, các child của nó trở thành **orphan** và được **reparent** cho PID 1
(hoặc "subreaper" gần nhất), cái mà phải `wait()` chúng. Trên một host bình thường PID 1 là
`init`/`systemd`, và nó làm vậy. Trong container, **PID 1 là process của bạn** (process đầu
tiên trong PID namespace, [`13-containers.md`](13-containers.md)) — và kernel đối xử đặc biệt với nó: nó
**bỏ qua các signal mà nó không có handler** (nên `SIGTERM` không làm gì trừ khi bạn xử lý —
[`17-signals.md`](17-signals.md)), và nó chịu trách nhiệm thu dọn các child mồ côi. Một proxy là PID 1 và
spawn helper có thể tích lũy zombie, và một proxy không cài `SIGTERM` handler thì không thể
dừng một cách graceful. Các cách sửa chuẩn: cài handler (dù sao cũng cần cho graceful
shutdown, [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md)), hoặc chạy một init nhỏ (`tini`, `docker
run --init`) làm PID 1.

### Process group, session, và controlling terminal
Process thuộc về một **process group**, và group thuộc về một **session**; một session có thể
có một **controlling terminal**. Shell làm mỗi pipeline thành group riêng để Ctrl-C
(`SIGINT`) chỉ đến **foreground group**. Một **daemon** trước đây tách khỏi terminal
(`fork`, `setsid`) để việc đóng terminal (`SIGHUP`) không giết nó; ngày nay một supervisor
(systemd, container runtime) làm việc đó và daemon chỉ ở foreground, log ra stdout/stderr
([`21-systemd-and-services.md`](21-systemd-and-services.md)). `kill -TERM -<pgid>` gửi signal cho cả group — cách một supervisor
dừng một process và các child nó đã spawn.

### Thread là process chia sẻ các thứ
Trên Linux một thread được tạo bằng `clone()` với các cờ nói *chia sẻ gì* (bộ nhớ, fd, signal
handler). `fork` là `clone` không chia sẻ gì; `pthread_create` là `clone` chia sẻ gần như mọi
thứ. Mỗi thread có **TID** riêng; TID của thread-group leader là PID của process. Đó là lý do
`ps -eLf` hiện thread thành các hàng, vì sao `/proc/<pid>/task/<tid>` tồn tại, và vì sao một
signal gửi tới process được giao cho *một thread tùy ý* không block nó
([`17-signals.md`](17-signals.md)).

### Gotcha: process bạn nghĩ đã khởi động không phải cái đang chạy
`sh -c "./proxy"` có thể để lại một shell giữa supervisor và binary của bạn, nên signal gửi
tới PID trúng shell, không phải proxy; `exec ./proxy` trong script thay thế shell để proxy
*chính là* PID. `CMD` của container ở dạng shell (`CMD ./proxy`) bọc trong `sh -c`; dạng exec
(`CMD ["./proxy"]`) thì không. Chi tiết này nằm đằng sau nhiều báo cáo "container của tôi bỏ
qua SIGTERM và mất 10 giây mới dừng".

## Practice

1. Trong shell, chạy `sleep 100 &`, rồi `ps -o pid,ppid,pgid,sid,stat,cmd` cho nó và shell;
   xác định parent PID, process group và session, và đọc state `S` (sleeping). `kill %1` và
   ghi lại exit status hiện bởi `wait`.
2. Cố tình tạo một zombie: một chương trình Rust scratch `Command::spawn` lệnh `true` rồi
   `sleep` 60 s mà không gọi `wait()`; cho thấy entry `Z` bằng
   `ps -o pid,ppid,stat,cmd --ppid <parent>` và xác nhận nó biến mất khi parent gọi `wait()`
   hoặc thoát.
3. Làm mồ côi một child: spawn `sleep 300` từ một shell rồi shell đó thoát
   (`sh -c 'sleep 300 &'`), và dùng `ps -o pid,ppid,cmd -C sleep` để xem `PPID` của nó đổi
   thành 1 (hoặc một subreaper).
4. Làm một shell script `exec` một binary và một cái không, chạy mỗi cái dưới
   `ps -ef --forest`, cho thấy `sh` thừa trong cây; rồi gửi `SIGTERM` tới PID đã ghi trong cả
   hai trường hợp và mô tả sự khác biệt.
5. Chạy proxy của bạn ([`labs/05-reverse-proxy`](../../labs/05-reverse-proxy)) làm PID 1 trong một container
   (`docker run --rm` một image tối giản, hoặc `unshare --pid --fork --mount-proc`), thử
   `docker stop`/`kill -TERM 1` có và không có `tokio::signal` handler được cài, và ghi lại
   việc shutdown mất bao lâu và exit code (`echo $?`: 143 vs 137).
6. Xem `/proc/<pid>/status` (`PPid`, `Threads`, `State`), `/proc/<pid>/fd`, và
   `ls /proc/<pid>/task` của một chương trình tokio đa luồng đang chạy; khớp số thread với
   các worker thread của runtime cộng blocking pool của nó.
