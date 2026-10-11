# Vòng đời Process: fork, exec, wait, Zombie và PID 1

File này nói về cách một process được sinh ra, bị thay thế, được giám sát và được dọn dẹp
(reap). Nó cũng giải thích vì sao hành vi của proxy phụ thuộc vào việc nó là con của shell,
của systemd hay của container runtime. File dựa trên
[`03-processes-and-threads.md`](03-processes-and-threads.md).

## What to learn

### fork: nhân bản một process
`fork()` tạo ra một **child** gần như giống hệt caller:

- Cùng code.
- Cùng nội dung bộ nhớ. Bộ nhớ được copy theo kiểu copy-on-write, nên việc copy diễn ra
  lười và rất rẻ ([`16-memory.md`](16-memory.md)).
- Cùng các **file descriptor đang mở**. Cả hai process giờ trỏ vào *cùng* các kernel object
  ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)).

`fork` trả về **hai lần**: ở parent nó trả về PID của child, ở child nó trả về `0`. Từ đó hai
process đi hai hướng khác nhau.

Chi tiết về việc chia sẻ fd rất quan trọng. Một listening socket được thừa kế qua `fork` là
cách cổ điển nhất để chạy nhiều worker trên cùng một port. Nó cũng giải thích vì sao một
socket "đã đóng" vẫn có thể còn mở: một child bị bỏ quên vẫn đang giữ nó.

```rust
// hình dạng thô, qua libc (code scratch, không thuộc workspace)
let pid = unsafe { libc::fork() };
match pid {
    -1 => /* lỗi */ {},
    0  => /* child: pid là 0 ở đây */ {},
    _  => /* parent: pid là id của child */ {},
}
```
Trong một process multi-threaded (mọi chương trình tokio), bạn không được `fork` rồi tiếp tục
chạy code Rust trong child. Ở child chỉ có thread gọi fork còn sống. Những lock mà thread
khác đang giữ sẽ bị khóa vĩnh viễn. Vì vậy mới cần lời gọi tiếp theo.

### exec: thay chương trình
`execve(path, argv, envp)` **thay thế** process image hiện tại bằng một chương trình mới. PID
giữ nguyên, các fd đang mở cũng giữ nguyên (trừ fd được đánh dấu **close-on-exec**), còn bộ
nhớ thì hoàn toàn mới.

`fork` + `exec` là cách mọi shell chạy một lệnh, và là cách `std::process::Command` tạo một
child. (Bên trong, Rust dùng `posix_spawn` hoặc `fork+exec` theo cách an toàn.) Các đối số
(`argv`) và **environment** (`envp`) được thừa kế hoặc đặt ở bước này. Environment là cách
các cấu hình như `RUST_LOG` đến được process.

**Gotcha: fd bị rò rỉ qua exec.** Nếu child sau `exec` thừa kế một fd mà bạn không định chia
sẻ (một file bí mật, một listening socket), đó là một bug kinh điển. Thư viện chuẩn của Rust
mở file và socket với `O_CLOEXEC` theo mặc định. Với các lời gọi `libc::socket` thô, bạn phải
tự thêm cờ này (`SOCK_CLOEXEC`). Ngoại lệ là khi bạn *muốn* thừa kế, ví dụ trong hot restart,
khi bạn trao listening socket cho binary mới
([`09-architecture/05-rolling-restart.md`](../09-architecture/05-rolling-restart.md)).

### exit và wait: zombie
Một process kết thúc bằng `exit(code)`, bằng cách return từ `main`, hoặc bằng một signal gây
chết ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)). Kernel giải phóng bộ
nhớ và đóng các fd của nó. Nhưng kernel vẫn giữ lại một **entry nhỏ trong process table**
chứa exit status, cho đến khi parent lấy nó bằng `wait()`/`waitpid()`.

Một process đã kết thúc mà chưa được wait gọi là **zombie** (hiện `Z` trong `ps`). Nó không
dùng CPU hay bộ nhớ, chỉ chiếm một PID. Nếu parent không bao giờ wait, PID sẽ bị rò rỉ cho
đến khi `fork` fail với `EAGAIN`.

Exit status mã hóa một trong hai thứ: exit code (theo quy ước, 0 là thành công), hoặc "bị
giết bởi signal N". Trong shell, trường hợp sau hiện thành `128+N`. Vì vậy exit code 137 nghĩa
là `SIGKILL` (thường do OOM killer, [`16-memory.md`](16-memory.md)), còn 143 nghĩa là
`SIGTERM`.

```rust
let mut child = std::process::Command::new("sleep").arg("1").spawn()?;
let status = child.wait()?;           // reap zombie; status.code() / .signal()
```

### Orphan và PID 1
Nếu parent chết trước, các child của nó trở thành **orphan** và được **reparent** cho PID 1
(hoặc "subreaper" gần nhất). Process nhận nuôi này phải `wait()` chúng. Trên một host bình
thường, PID 1 là `init`/`systemd` và nó làm đúng việc này.

Trong container, **PID 1 chính là process của bạn** (process đầu tiên trong PID namespace,
[`13-containers.md`](13-containers.md)). Kernel đối xử đặc biệt với PID 1:

- Nó **bỏ qua các signal mà process không có handler**. Nghĩa là `SIGTERM` sẽ không có tác
  dụng gì, trừ khi bạn tự xử lý nó ([`17-signals.md`](17-signals.md)).
- Nó chịu trách nhiệm reap các child orphan.

Hệ quả: một proxy làm PID 1 và có spawn helper có thể tích lũy zombie. Một proxy không cài
`SIGTERM` handler thì không thể dừng một cách graceful.

Có hai cách sửa chuẩn:
- Cài handler. Dù sao bạn cũng cần nó cho graceful shutdown
  ([`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md)).
- Chạy một init nhỏ (`tini`, `docker run --init`) làm PID 1.

### Process group, session và controlling terminal
Mỗi process thuộc về một **process group**, và mỗi group thuộc về một **session**. Một session
có thể có một **controlling terminal**. Shell tạo mỗi pipeline thành một group riêng, để
Ctrl-C (`SIGINT`) chỉ đến **foreground group**.

Ngày xưa, một **daemon** tự tách khỏi terminal (`fork`, `setsid`) để việc đóng terminal
(`SIGHUP`) không giết nó. Ngày nay việc đó do supervisor (systemd, container runtime) làm.
Daemon chỉ cần chạy ở foreground và log ra stdout/stderr
([`21-systemd-and-services.md`](21-systemd-and-services.md)).

`kill -TERM -<pgid>` gửi signal cho cả group. Đó là cách supervisor dừng một process cùng các
child mà nó đã spawn.

### Thread là process chia sẻ một số thứ
Trên Linux, một thread được tạo bằng `clone()` với các cờ cho biết *chia sẻ gì* (bộ nhớ, fd,
signal handler). `fork` là `clone` mà không chia sẻ gì. `pthread_create` là `clone` mà chia sẻ
gần như mọi thứ.

Mỗi thread có **TID** riêng. TID của thread đầu tiên trong nhóm (thread-group leader) chính là
PID của process. Từ đó suy ra:
- `ps -eLf` hiện các thread thành từng dòng.
- `/proc/<pid>/task/<tid>` tồn tại.
- Một signal gửi tới process sẽ được giao cho *một thread bất kỳ* không chặn signal đó
  ([`17-signals.md`](17-signals.md)).

### Gotcha: process bạn nghĩ đã khởi động không phải process đang chạy
`sh -c "./proxy"` có thể để lại một shell nằm giữa supervisor và binary của bạn. Khi đó signal
gửi tới PID sẽ trúng shell chứ không trúng proxy. Dùng `exec ./proxy` trong script thì shell bị
thay thế, và proxy *chính là* PID đó.

Với `CMD` của container:
- Dạng shell (`CMD ./proxy`) được bọc trong `sh -c`.
- Dạng exec (`CMD ["./proxy"]`) thì không.

Chi tiết này đứng sau rất nhiều báo cáo kiểu "container của tôi bỏ qua SIGTERM và mất 10 giây
mới dừng".

## Practice

1. Trong shell, chạy `sleep 100 &`, rồi chạy `ps -o pid,ppid,pgid,sid,stat,cmd` cho nó và cho
   shell. Xác định parent PID, process group và session, và đọc state `S` (sleeping). Chạy
   `kill %1` và ghi lại exit status do `wait` hiện ra.
2. Cố tình tạo một zombie. Viết một chương trình Rust scratch gọi `Command::spawn` lệnh `true`
   rồi `sleep` 60 giây mà không gọi `wait()`. Cho thấy entry `Z` bằng
   `ps -o pid,ppid,stat,cmd --ppid <parent>`, và xác nhận nó biến mất khi parent gọi `wait()`
   hoặc thoát.
3. Tạo một orphan: spawn `sleep 300` từ một shell, rồi cho shell đó thoát
   (`sh -c 'sleep 300 &'`). Dùng `ps -o pid,ppid,cmd -C sleep` để xem `PPID` của nó đổi thành 1
   (hoặc một subreaper).
4. Viết hai shell script, một cái `exec` một binary và một cái không. Chạy từng cái dưới
   `ps -ef --forest` và cho thấy `sh` thừa trong cây process. Sau đó gửi `SIGTERM` tới PID đã
   ghi lại trong cả hai trường hợp và mô tả sự khác biệt.
5. Chạy proxy của bạn ([`labs/05-reverse-proxy`](../../labs/05-reverse-proxy)) làm PID 1 trong
   một container (`docker run --rm` với một image tối giản, hoặc
   `unshare --pid --fork --mount-proc`). Thử `docker stop` hoặc `kill -TERM 1` khi có và khi
   không có `tokio::signal` handler, và ghi lại việc shutdown mất bao lâu cùng exit code
   (`echo $?`: 143 so với 137).
6. Xem `/proc/<pid>/status` (`PPid`, `Threads`, `State`), `/proc/<pid>/fd` và
   `ls /proc/<pid>/task` của một chương trình tokio multi-threaded đang chạy. Đối chiếu số
   thread với số worker thread của runtime cộng với blocking pool của nó.

---

<!-- nav -->
← Trước: [`02-linux/03-processes-and-threads.md`](03-processes-and-threads.md) · [Mục lục](00-README.md) · Tiếp theo: [`02-linux/05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md) →
