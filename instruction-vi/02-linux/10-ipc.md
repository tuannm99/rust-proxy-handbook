# Inter-Process Communication: Pipe, Unix Socket, Shared Memory, futex

Các process riêng biệt — và các thread — nói chuyện và đồng bộ với nhau như thế nào trên một
máy. Quan trọng với proxy vì hot restart trao một listening socket giữa các process, một control
plane nói chuyện với proxy qua Unix socket, và mọi `Mutex` đều chạm đáy ở một primitive kernel ở
đây. Xây trên [`04-process-lifecycle.md`](04-process-lifecycle.md) và [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md).

## What to learn

### Vì sao IPC tồn tại: cô lập là mặc định
Mỗi process có bộ nhớ riêng ([`08-memory-basics.md`](08-memory-basics.md)), nên cái này không đọc được biến của cái
kia. Để hợp tác chúng xin kernel một kênh chung. Mỗi cơ chế dưới đây là một sự đánh đổi giữa tốc
độ (shared memory: không copy), cấu trúc (message vs byte) và sự tiện lợi. Tất cả trừ shared
memory thô đều là **file descriptor**, nên cắm được vào `epoll` như socket ([`14-epoll.md`](14-epoll.md)).

### Pipe: một byte queue của kernel giữa các process có quan hệ
`pipe()` trả về hai fd: đầu write và đầu read, được hỗ trợ bởi một kernel buffer kích thước cố
định (mặc định 64 KiB). Byte ghi vào ra theo thứ tự; đọc một pipe rỗng thì **block** (hoặc
`EAGAIN` nếu non-blocking); ghi vào pipe đầy thì block — **backpressure** ở dạng thuần túy nhất
([`01-network/13-tcp-reliability.md`](../01-network/13-tcp-reliability.md) là cùng ý tưởng qua mạng). Khi mọi đầu write đã đóng, `read` trả về 0
(EOF); ghi vào pipe không còn reader sẽ nâng `SIGPIPE`/`EPIPE`. `a | b` của shell là một pipe nối
stdout của `a` với stdin của `b`. Write ≤ `PIPE_BUF` (4096) byte là atomic — không xen kẽ với
writer khác. **Named pipe (FIFO)** cho pipe một path để các process không liên quan mở được.
Pipe một chiều và hướng byte, không có ranh giới message ([`01-network/03-byte-streams.md`](../01-network/03-byte-streams.md)).

### Unix domain socket: socket không qua mạng
**Unix domain socket** có cùng API với TCP (`socket`, `bind`, `listen`, `accept`, `connect`,
[`01-network/11-socket.md`](../01-network/11-socket.md)) nhưng địa chỉ là một **path filesystem** (hoặc một tên abstract trên Linux) và
mọi thứ ở trong kernel — không IP, không TCP, không checksum, latency và CPU thấp hơn nhiều so với
TCP loopback. Loại: `SOCK_STREAM` (như TCP) và `SOCK_DGRAM`/`SOCK_SEQPACKET` (giữ ranh giới
message). Kiểm soát truy cập chỉ là **permission của file** tại path socket
([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)), và server có thể hỏi ai đã connect qua `SO_PEERCRED` (PID/UID của peer, được
kernel đảm bảo). Proxy dùng chúng cho admin/control API, cho backend fastcgi/upstream local, và —
lý do nó nằm trong file này — để truyền fd.

```rust
let l = tokio::net::UnixListener::bind("/run/proxy/admin.sock")?;
let (stream, _addr) = l.accept().await?;   // cùng hình dạng với TcpListener
```
**Gotcha:** bind fail với `EADDRINUSE` nếu socket file còn lại từ một lần chạy bị crash — hãy xóa
file cũ lúc khởi động (hoặc dùng abstract socket); và đặt permission cho directory, vì *ai mở được
path thì connect được*.

### Truyền file descriptor: SCM_RIGHTS
Một Unix socket có thể mang **fd như ancillary data** (`sendmsg` với `SCM_RIGHTS`): kernel cài một
bản sao của open file description của bên gửi vào bảng fd của bên nhận. Đây là cách một **hot
restart** không downtime hoạt động: proxy cũ gửi listening socket của nó (và tùy chọn các connection
đang sống) cho binary mới qua một Unix socket, nên port không bao giờ đóng và client không bao giờ
bị từ chối ([`09-architecture/05-rolling-restart.md`](../09-architecture/05-rolling-restart.md)). Hot restart của Envoy và nhiều thiết lập
socket-activation ([`21-systemd-and-services.md`](21-systemd-and-services.md)) dựa vào nó. *Số* fd của bên nhận khác; kernel object thì
giống ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)). Rust: `sendmsg`/`recvmsg` qua `nix` hoặc các crate kiểu `sendfd`.

### Shared memory và mmap: nhanh nhất, và bạn tự lo đồng bộ
Hai process có thể map **cùng các physical page** vào address space của mình (`shm_open` +
`mmap(MAP_SHARED)`, hoặc `mmap` một file hay file tmpfs `/dev/shm`). Khi đó đọc và ghi không tốn
syscall hay copy — IPC nhanh nhất có thể — nhưng kernel cho bạn **không đảm bảo thứ tự hay lock**:
bạn cần atomic, một lock trong vùng chia sẻ, hoặc một lock-free ring buffer, và một writer crash có
thể để lại cấu trúc cập nhật dở. Dùng cho shared cache lớn (một cache chia sẻ giữa các *process*
worker, [`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)) và metrics counter. Một proxy mô hình đa process cần shared memory
cho mọi state — counter rate-limit ([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)), cache index — mà các thread trong một process
chỉ cần chia sẻ bằng `Arc`. Đừng bao giờ đặt `Vec`, `String` hay pointer vào shared memory — pointer
chỉ hợp lệ trong address space đã ghi ra nó.

### eventfd, signalfd, timerfd: event dưới dạng file descriptor
Một số event kernel được phơi ra dưới dạng fd chính để một vòng `epoll` có thể chờ mọi thứ.
**`eventfd`** là một bộ đếm bạn ghi vào để wake up người chờ — một cách "wake up event loop"
liên thread rẻ tiền (thứ runtime dùng bên trong, [`04-runtime/02-waker.md`](../04-runtime/02-waker.md)). **`signalfd`** biến signal
thành dữ liệu đọc được ([`17-signals.md`](17-signals.md)); **`timerfd`** giao các lần timer expire
([`11-time-and-timers.md`](11-time-and-timers.md)). Bài học thiết kế: khi bạn biến được một event thành fd, event loop của bạn
giữ được một cơ chế duy nhất.

### futex: một Mutex thực sự chờ thế nào
Trong một process (hoặc xuyên process, với shared memory), `Mutex`, `Condvar`, channel và parking đều
được xây trên syscall **futex** ("fast userspace mutex"). Đường nhanh **chỉ là một atomic
compare-and-swap trong user space** — không syscall nếu không ai tranh chấp. Chỉ khi tranh chấp một
thread mới gọi `futex(WAIT)` để sleep cho tới khi thread khác gọi `futex(WAKE)` trên cùng địa chỉ. Đó là
vì sao một lock không tranh chấp tốn vài nano giây còn một lock bị tranh chấp tốn micro giây (một
syscall cộng một context switch, [`02-hardware-basics.md`](02-hardware-basics.md)). `std::sync::Mutex` của Rust trên Linux
chính là cái này ([`03-rust/`](../03-rust) các file sync). `strace -f` hiện một loạt lời gọi `futex` nghĩa là
tranh chấp lock.

### SysV/POSIX message queue, semaphore, và D-Bus: ở đây phần lớn là legacy
IPC SysV cũ hơn (`shmget`, `msgget`, `semget`) và POSIX message queue vẫn tồn tại và xuất hiện trong
code legacy; với công việc proxy mới, ưu tiên Unix socket (cấu trúc, kiểm soát truy cập, thân thiện
epoll) và shared memory `mmap` (tốc độ). Một network socket tới host khác cũng là IPC — điều mà phần
còn lại của handbook này nói tới.

### Lựa chọn
| Nhu cầu | Dùng |
|---|---|
| byte stream parent <-> child | pipe |
| control/admin API local, backend local | Unix domain socket |
| trao một listener/connection cho process khác | Unix socket + `SCM_RIGHTS` |
| state chia sẻ lớn, latency thấp nhất | shared memory `mmap` + atomic |
| wake up event loop từ thread khác | `eventfd` (hoặc `Notify` của runtime) |
| một thread chờ thread khác | mutex/condvar/channel dựa trên `futex` |

## Practice

1. Chạy `yes | head -c 1G | wc -c` dưới `strace -c -f` và đọc số `read`/`write` để thấy việc pipe
   chia khúc; đọc kích thước pipe buffer bằng
   `python3 -c 'import fcntl,os; r,w=os.pipe(); print(fcntl.fcntl(w,1032))'` (`F_GETPIPE_SZ`, 1032).
2. Cho thấy pipe backpressure và EPIPE: một chương trình Rust scratch ghi vào một pipe không ai đọc —
   quan sát writer block sau ~64 KiB — rồi đóng đầu read và bắt `EPIPE`/`SIGPIPE`.
3. Chạy một Unix-socket echo bằng `socat UNIX-LISTEN:/tmp/s.sock,fork EXEC:cat` và connect bằng
   `socat - UNIX-CONNECT:/tmp/s.sock`; xem `ls -l /tmp/s.sock` (type `s`), rồi benchmark TCP loopback vs
   Unix socket bằng cùng một chương trình scratch nhỏ và so sánh throughput.
4. Thêm một admin endpoint trên Unix socket vào [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) của bạn (ví dụ "in số
   upstream") và giới hạn nó bằng file mode để user khác không connect được; cho thấy `connect` fail với
   `EACCES`.
5. Truyền một fd giữa hai process scratch bằng `SCM_RIGHTS` (qua `nix::sys::socket::sendmsg`/`recvmsg`):
   process A bind một listener và gửi cho B, A thoát, và B vẫn accept connection trên cùng port mà không có
   khoảng hở. Đây là lõi của [`labs/13-hot-reload`](../../labs/13-hot-reload) ở mức process.
6. Map một file `/dev/shm` chia sẻ từ hai process và tăng một `AtomicU64` trong đó 1.000.000 lần mỗi bên;
   xác minh tổng chính xác, rồi thay atomic bằng một read-modify-write thường và quan sát việc mất cập
   nhật. Riêng ra, `strace -f -e trace=futex` một chương trình tokio multi-threaded có một `Mutex` nóng và không
   có, và giải thích sự khác biệt về số lời gọi `futex`.
