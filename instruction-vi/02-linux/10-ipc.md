# Inter-Process Communication: Pipe, Unix Socket, Shared Memory, futex

File này nói về cách các process riêng biệt, và các thread, nói chuyện và đồng bộ với nhau trên
cùng một máy. Nó quan trọng với proxy vì:
- Hot restart trao một listening socket giữa các process.
- Control plane nói chuyện với proxy qua Unix socket.
- Mọi `Mutex` cuối cùng đều dựa vào một primitive của kernel được giải thích ở đây.

File dựa trên [`04-process-lifecycle.md`](04-process-lifecycle.md) và
[`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md).

## What to learn

### Vì sao cần IPC: mặc định các process bị cách ly
Mỗi process có bộ nhớ riêng ([`08-memory-basics.md`](08-memory-basics.md)), nên process này không
đọc được biến của process kia. Muốn hợp tác, chúng phải nhờ kernel cung cấp một kênh chung.

Mỗi cơ chế dưới đây là một sự đánh đổi giữa tốc độ (shared memory: không phải copy), cấu trúc
(message hay byte stream) và sự tiện lợi. Tất cả, trừ shared memory thô, đều là **file
descriptor**, nên cắm được vào `epoll` giống như socket ([`14-epoll.md`](14-epoll.md)).

### Pipe: một byte queue của kernel giữa các process có quan hệ
`pipe()` trả về hai fd: một đầu write và một đầu read. Phía sau là một kernel buffer có kích thước
cố định (mặc định 64 KiB). Byte ghi vào sẽ ra theo đúng thứ tự.

- Đọc một pipe rỗng thì **block** (hoặc trả `EAGAIN` nếu non-blocking).
- Ghi vào pipe đầy cũng block. Đây là **backpressure** ở dạng thuần túy nhất (ý tưởng giống hệt
  qua mạng ở [`01-network/13-tcp-reliability.md`](../01-network/13-tcp-reliability.md)).
- Khi mọi đầu write đã đóng, `read` trả về 0 (EOF).
- Ghi vào pipe không còn reader thì phát ra `SIGPIPE`/`EPIPE`.

`a | b` của shell chỉ là một pipe nối stdout của `a` với stdin của `b`. Một lần write có kích thước
≤ `PIPE_BUF` (4096) byte là atomic, nghĩa là không bị xen kẽ với dữ liệu của writer khác.

**Named pipe (FIFO)** cho pipe một path, để các process không liên quan cũng mở được. Pipe là một
chiều và hướng byte, không có ranh giới message
([`01-network/03-byte-streams.md`](../01-network/03-byte-streams.md)).

### Unix domain socket: socket không đi qua mạng
**Unix domain socket** có cùng API với TCP (`socket`, `bind`, `listen`, `accept`, `connect`,
[`01-network/11-socket.md`](../01-network/11-socket.md)). Khác biệt là địa chỉ của nó là một **path
trong filesystem** (hoặc một tên abstract trên Linux), và mọi thứ diễn ra bên trong kernel: không
có IP, không có TCP, không có checksum. Latency và chi phí CPU thấp hơn nhiều so với TCP loopback.

Các loại: `SOCK_STREAM` (giống TCP) và `SOCK_DGRAM`/`SOCK_SEQPACKET` (giữ ranh giới message).

Kiểm soát truy cập chỉ là **permission của file** tại path socket
([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)). Server còn có thể
hỏi xem ai đã connect, qua `SO_PEERCRED` (PID/UID của peer, do kernel đảm bảo).

Proxy dùng Unix socket cho admin/control API, cho backend fastcgi hoặc upstream local, và (đây là lý
do nó nằm trong file này) để truyền fd.

```rust
let l = tokio::net::UnixListener::bind("/run/proxy/admin.sock")?;
let (stream, _addr) = l.accept().await?;   // cùng hình dạng với TcpListener
```
**Gotcha:** `bind` fail với `EADDRINUSE` nếu socket file còn sót lại từ lần chạy bị crash. Hãy xóa
file cũ lúc khởi động (hoặc dùng abstract socket). Đồng thời hãy đặt permission cho directory chứa
nó, vì *ai mở được path thì connect được*.

### Truyền file descriptor: SCM_RIGHTS
Một Unix socket có thể mang **fd như một ancillary data** (`sendmsg` với `SCM_RIGHTS`). Kernel sẽ
cài một bản sao của open file description của bên gửi vào bảng fd của bên nhận.

Đây là cách **hot restart** không downtime hoạt động. Proxy cũ gửi listening socket của nó (và có
thể cả các connection đang sống) cho binary mới qua một Unix socket. Nhờ vậy port không bao giờ bị
đóng và client không bao giờ bị từ chối
([`09-architecture/05-rolling-restart.md`](../09-architecture/05-rolling-restart.md)). Hot restart của
Envoy và nhiều thiết lập socket-activation ([`21-systemd-and-services.md`](21-systemd-and-services.md))
đều dựa vào cơ chế này.

*Số* fd ở bên nhận sẽ khác, nhưng kernel object bên dưới thì giống nhau
([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)). Trong Rust, dùng `sendmsg`/`recvmsg` qua
`nix`, hoặc các crate như `sendfd`.

### Shared memory và mmap: nhanh nhất, nhưng bạn tự lo đồng bộ
Hai process có thể map **cùng các physical page** vào address space của mình (`shm_open` +
`mmap(MAP_SHARED)`, hoặc `mmap` một file, hoặc file tmpfs trong `/dev/shm`). Khi đó đọc và ghi không
tốn syscall hay copy nào. Đây là IPC nhanh nhất có thể. Nhưng kernel **không đảm bảo thứ tự và
không cung cấp lock**. Bạn phải tự dùng atomic, một lock đặt trong vùng chia sẻ, hoặc một lock-free
ring buffer. Nếu một writer crash, nó có thể để lại cấu trúc dữ liệu cập nhật dở.

Dùng shared memory cho các cache lớn dùng chung (một cache chia sẻ giữa các *process* worker,
[`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)) và metrics counter. Một proxy theo mô
hình nhiều process cần shared memory cho mọi state, ví dụ counter của rate limit
([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)) hay cache index. Trong khi đó, các thread
trong cùng một process chỉ cần chia sẻ bằng `Arc`.

Đừng bao giờ đặt `Vec`, `String` hay pointer vào shared memory. Một pointer chỉ có nghĩa trong
address space đã ghi ra nó.

### eventfd, signalfd, timerfd: event ở dạng file descriptor
Một số event của kernel được phơi ra dưới dạng fd, để một vòng `epoll` có thể chờ tất cả cùng lúc.

- **`eventfd`** là một bộ đếm mà bạn ghi vào để wake up bên đang chờ. Đây là cách rẻ để "wake up
  event loop" từ thread khác, và runtime dùng nó bên trong
  ([`04-runtime/02-waker.md`](../04-runtime/02-waker.md)).
- **`signalfd`** biến signal thành dữ liệu đọc được ([`17-signals.md`](17-signals.md)).
- **`timerfd`** giao các lần timer expire ([`11-time-and-timers.md`](11-time-and-timers.md)).

Bài học thiết kế: nếu bạn biến được một event thành fd, event loop của bạn chỉ cần đúng một cơ chế
chờ.

### futex: một Mutex thực sự chờ như thế nào
Trong một process (hoặc giữa các process, nếu dùng shared memory), `Mutex`, `Condvar`, channel và
parking đều được xây trên syscall **futex** ("fast userspace mutex").

Đường nhanh **chỉ là một phép atomic compare-and-swap trong user space**, không tốn syscall khi
không có tranh chấp. Chỉ khi có tranh chấp, thread mới gọi `futex(WAIT)` để sleep cho đến khi thread
khác gọi `futex(WAKE)` trên cùng địa chỉ.

Vì vậy một lock không bị tranh chấp tốn vài nano giây, còn một lock bị tranh chấp tốn cỡ micro giây
(một syscall cộng một context switch, [`02-hardware-basics.md`](02-hardware-basics.md)).
`std::sync::Mutex` của Rust trên Linux chính là cơ chế này (các file sync trong
[`03-rust/`](../03-rust)). Nếu `strace -f` hiện ra hàng loạt lời gọi `futex`, nghĩa là đang có tranh
chấp lock.

### SysV/POSIX message queue, semaphore và D-Bus: ở đây phần lớn là legacy
IPC kiểu SysV cũ hơn (`shmget`, `msgget`, `semget`) và POSIX message queue vẫn còn tồn tại, và bạn
sẽ gặp chúng trong code legacy. Với công việc proxy mới, hãy ưu tiên Unix socket (có cấu trúc, có
kiểm soát truy cập, thân thiện với epoll) và shared memory `mmap` (tốc độ). Một network socket tới
host khác cũng là IPC, và đó là chủ đề của phần còn lại của handbook này.

### Cách chọn
| Nhu cầu | Dùng |
|---|---|
| byte stream parent ↔ child | pipe |
| control/admin API local, backend local | Unix domain socket |
| trao một listener/connection cho process khác | Unix socket + `SCM_RIGHTS` |
| state chia sẻ lớn, latency thấp nhất | shared memory `mmap` + atomic |
| wake up event loop từ thread khác | `eventfd` (hoặc `Notify` của runtime) |
| một thread chờ thread khác | mutex/condvar/channel dựa trên `futex` |

## Practice

1. Chạy `yes | head -c 1G | wc -c` dưới `strace -c -f` và đọc số lần `read`/`write` để thấy pipe
   chia dữ liệu thành từng khúc. Đọc kích thước pipe buffer bằng
   `python3 -c 'import fcntl,os; r,w=os.pipe(); print(fcntl.fcntl(w,1032))'` (`F_GETPIPE_SZ`, 1032).
2. Cho thấy pipe backpressure và EPIPE. Viết một chương trình Rust scratch ghi vào một pipe không
   ai đọc, quan sát writer bị block sau khoảng 64 KiB. Sau đó đóng đầu read và bắt `EPIPE`/`SIGPIPE`.
3. Chạy một Unix-socket echo bằng `socat UNIX-LISTEN:/tmp/s.sock,fork EXEC:cat` và connect bằng
   `socat - UNIX-CONNECT:/tmp/s.sock`. Xem `ls -l /tmp/s.sock` (type `s`). Sau đó benchmark TCP
   loopback với Unix socket bằng cùng một chương trình scratch nhỏ và so sánh throughput.
4. Thêm một admin endpoint trên Unix socket vào [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy)
   của bạn (ví dụ "in số upstream"), và giới hạn nó bằng file mode để user khác không connect được.
   Cho thấy `connect` fail với `EACCES`.
5. Truyền một fd giữa hai process scratch bằng `SCM_RIGHTS` (qua
   `nix::sys::socket::sendmsg`/`recvmsg`). Process A bind một listener rồi gửi cho B, A thoát, và B
   vẫn accept connection trên cùng port mà không có khoảng hở. Đây là lõi của
   [`labs/13-hot-reload`](../../labs/13-hot-reload) ở mức process.
6. Map một file chia sẻ trong `/dev/shm` từ hai process và mỗi bên tăng một `AtomicU64` trong đó
   1.000.000 lần. Xác minh tổng chính xác. Sau đó thay atomic bằng một phép read-modify-write thường
   và quan sát việc mất cập nhật. Riêng ra, chạy `strace -f -e trace=futex` trên một chương trình
   tokio multi-threaded, một lần có một `Mutex` bị tranh chấp nhiều và một lần không. Giải thích sự
   khác nhau về số lời gọi `futex`.
