# Inter-Process Communication: Pipes, Unix Sockets, Shared Memory, futex

How separate processes — and threads — talk and synchronize on one machine. Matters
for a proxy because a hot restart hands a listening socket between processes, a control
plane talks to the proxy over a Unix socket, and every `Mutex` bottoms out in a kernel
primitive here. Builds on [`04-process-lifecycle.md`](04-process-lifecycle.md) and [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md).

## What to learn

### Why IPC exists: isolation is the default
Each process has its own memory ([`08-memory-basics.md`](08-memory-basics.md)), so one cannot read another's
variables. To cooperate they ask the kernel for a shared channel. Every mechanism below is a
trade between speed (shared memory: no copy), structure (messages vs bytes) and convenience.
All except raw shared memory are **file descriptors**, so they plug into `epoll` like sockets
([`14-epoll.md`](14-epoll.md)).

### Pipes: a kernel byte queue between related processes
`pipe()` returns two fds: write end and read end, backed by a fixed-size kernel buffer (64 KiB
by default). Bytes written come out in order; reading an empty pipe **blocks** (or `EAGAIN` if
non-blocking); writing a full pipe blocks — **backpressure** in its purest form
([`01-network/13-tcp-reliability.md`](../01-network/13-tcp-reliability.md) is the same idea over a network). When all write ends are closed,
`read` returns 0 (EOF); writing to a pipe with no readers raises `SIGPIPE`/`EPIPE`. A shell's `a | b`
is a pipe connecting `a`'s stdout to `b`'s stdin. Writes of ≤ `PIPE_BUF` (4096) bytes are atomic —
not interleaved with another writer's. **Named pipes (FIFOs)** give a pipe a path so unrelated
processes can open it. Pipes are unidirectional and byte-oriented, with no message boundaries
([`01-network/03-byte-streams.md`](../01-network/03-byte-streams.md)).

### Unix domain sockets: sockets without the network
A **Unix domain socket** has the same API as TCP (`socket`, `bind`, `listen`, `accept`,
`connect`, [`01-network/11-socket.md`](../01-network/11-socket.md)) but the address is a **filesystem path** (or an abstract name
on Linux) and everything stays in the kernel — no IP, no TCP, no checksum, far lower latency and
CPU than loopback TCP. Types: `SOCK_STREAM` (like TCP) and `SOCK_DGRAM`/`SOCK_SEQPACKET` (message
boundaries preserved). Access control is just **file permissions** on the socket path
([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)), and the server can ask who connected via `SO_PEERCRED` (the peer's
PID/UID, which the kernel guarantees). A proxy uses them for the admin/control API, for local
fastcgi/upstream backends, and — the reason it is in this file — to pass fds.

```rust
let l = tokio::net::UnixListener::bind("/run/proxy/admin.sock")?;
let (stream, _addr) = l.accept().await?;   // same shape as TcpListener
```
**Gotcha:** binding fails with `EADDRINUSE` if the socket file already exists from a crashed run —
remove the stale file at startup (or use an abstract socket); and set the directory's permissions,
since *anyone who can open the path can connect*.

### Passing file descriptors: SCM_RIGHTS
A Unix socket can carry an **fd as ancillary data** (`sendmsg` with `SCM_RIGHTS`): the kernel
installs a duplicate of the sender's open file description into the receiver's fd table. This is how
a zero-downtime **hot restart** works: the old proxy sends its listening socket (and optionally live
connections) to the new binary over a Unix socket, so the port never closes and clients are never
refused ([`09-architecture/05-rolling-restart.md`](../09-architecture/05-rolling-restart.md)). Envoy's hot restart and many
socket-activation setups ([`21-systemd-and-services.md`](21-systemd-and-services.md)) rely on it. The receiver's fd *number* is
different; the kernel object is the same ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)). Rust: `sendmsg`/`recvmsg` via
`nix` or `sendfd`-style crates.

### Shared memory and mmap: fastest, and you own the synchronization
Two processes can map the **same physical pages** into their address spaces (`shm_open` +
`mmap(MAP_SHARED)`, or `mmap` of a file or `/dev/shm` tmpfs file). Reads and writes then cost no syscall
and no copy — the fastest IPC possible — but the kernel gives you **no ordering or locking**: you
need atomics, a lock in the shared region, or a lock-free ring buffer, and a crashed writer can leave
the structure half-updated. Used for big shared caches (a cache shared by worker *processes*,
[`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)) and metrics counters. A proxy with a multi-process model needs
shared memory for any state — rate-limit counters ([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)), cache index — that
threads in one process would simply share by `Arc`. Never put a `Vec`, `String` or pointer in shared
memory — pointers are only valid in the address space that wrote them.

### eventfd, signalfd, timerfd: events as file descriptors
Some kernel events are exposed as fds precisely so one `epoll` loop can wait on everything. **`eventfd`**
is a counter you write to wake a waiter — a cheap cross-thread "wake the event loop" (what runtimes
use internally, [`04-runtime/02-waker.md`](../04-runtime/02-waker.md)). **`signalfd`** turns signals into readable data
([`17-signals.md`](17-signals.md)); **`timerfd`** delivers timer expirations ([`11-time-and-timers.md`](11-time-and-timers.md)). The
design lesson: when you can make an event a fd, your event loop stays single-mechanism.

### futex: how a Mutex actually waits
Within a process (or across, with shared memory), `Mutex`, `Condvar`, channels and parking are built
on the **futex** ("fast userspace mutex") syscall. The fast path is **just an atomic compare-and-swap in
user space** — no syscall if nobody contends. Only on contention does a thread call `futex(WAIT)` to sleep
until another thread calls `futex(WAKE)` on the same address. That's why an uncontended lock is a few
nanoseconds and a contended one costs microseconds (a syscall plus a context switch,
[`02-hardware-basics.md`](02-hardware-basics.md)). Rust's `std::sync::Mutex` is exactly this on Linux
([`03-rust/`](../03-rust) sync files). `strace -f` showing a flood of `futex` calls means lock contention.

### SysV/POSIX message queues, semaphores, and D-Bus: mostly legacy here
Older SysV IPC (`shmget`, `msgget`, `semget`) and POSIX message queues exist and appear in legacy
code; for new proxy work, prefer Unix sockets (structure, access control, epoll-friendly) and
`mmap` shared memory (speed). A network socket to another host is also IPC — which is what the rest
of this handbook is about.

### Choosing
| Need | Use |
|---|---|
| parent <-> child byte stream | pipe |
| local control/admin API, local backend | Unix domain socket |
| hand a listener/connection to another process | Unix socket + `SCM_RIGHTS` |
| big shared state, lowest latency | `mmap` shared memory + atomics |
| wake an event loop from another thread | `eventfd` (or runtime `Notify`) |
| one thread waits for another | `futex`-based mutex/condvar/channel |

## Practice

1. Run `yes | head -c 1G | wc -c` under `strace -c -f` and read the `read`/`write` counts to see the pipe's
   chunking; read the pipe buffer size with
   `python3 -c 'import fcntl,os; r,w=os.pipe(); print(fcntl.fcntl(w,1032))'` (`F_GETPIPE_SZ`, 1032).
2. Show pipe backpressure and EPIPE: a scratch Rust program that writes to a pipe nobody reads — observe the
   writer block after ~64 KiB — then closes the read end and captures `EPIPE`/`SIGPIPE`.
3. Run a Unix-socket echo with `socat UNIX-LISTEN:/tmp/s.sock,fork EXEC:cat` and connect with
   `socat - UNIX-CONNECT:/tmp/s.sock`; check `ls -l /tmp/s.sock` (type `s`), then benchmark loopback TCP vs the
   Unix socket with the same small scratch program and compare throughput.
4. Add an admin endpoint on a Unix socket to your [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) (e.g. "print upstream
   count") and restrict it by file mode so another user can't connect; show `connect` failing with `EACCES`.
5. Pass an fd between two scratch processes with `SCM_RIGHTS` (via `nix::sys::socket::sendmsg`/`recvmsg`): process A
   binds a listener and sends it to B, A exits, and B still accepts connections on the same port without a gap.
   This is the core of [`labs/13-hot-reload`](../../labs/13-hot-reload) at the process level.
6. Map a shared `/dev/shm` file from two processes and increment an `AtomicU64` in it 1,000,000 times each; verify
   the total is exact, then replace the atomic with a plain read-modify-write and observe lost updates. Separately,
   `strace -f -e trace=futex` a multi-threaded tokio program with a hot `Mutex` and without one, and explain the
   difference in `futex` call counts.
