# Socket Programming

Study bind/listen/accept/connect/send/recv/shutdown.

## What to learn

### The syscall lifecycle
`socket()` creates a file descriptor; `bind()` attaches it to a local
address/port; `listen()` marks it as a passive listening socket with a
backlog queue; `accept()` pulls a completed connection off that backlog and
returns a *new* fd for that connection (the listening fd keeps listening).
On the client side, `connect()` performs the TCP 3-way handshake. This
maps directly onto `TcpListener::bind` + `.accept()` and `TcpStream::connect`
in Rust, but knowing the raw syscalls is what makes a raw-`libc` epoll loop
(see [`02-linux/14-epoll.md`](../02-linux/14-epoll.md)'s exercise) comprehensible instead of magic.

### Blocking vs non-blocking
A blocking socket's `read`/`write`/`accept` parks the calling thread until
data/connections are ready. A non-blocking socket returns `EWOULDBLOCK`/
`EAGAIN` immediately instead — which is the foundation an event loop
(epoll, see [`02-linux/14-epoll.md`](../02-linux/14-epoll.md)) is built on: register the fd, block on
*many* fds at once in `epoll_wait`, and only call `read`/`write` when told
the fd is ready. Tokio's `TcpListener`/`TcpStream` are non-blocking
underneath and integrate with its reactor automatically.

### SO_REUSEADDR and SO_REUSEPORT
`SO_REUSEADDR` lets you rebind a port still in `TIME_WAIT` from a previous
process instance (essential for fast restarts — without it your proxy
fails to bind for ~60s after a restart). `SO_REUSEPORT` lets *multiple*
independent sockets bind the *same* port, with the kernel load-balancing
incoming connections across them — used to run one listener per worker
thread/process instead of a single shared-fd listener with a lock.

```rust
let socket = tokio::net::TcpSocket::new_v4()?;
socket.set_reuseaddr(true)?;
socket.bind("0.0.0.0:8080".parse()?)?;
let listener = socket.listen(1024)?;
```

### The backlog and connection storms
The `listen()` backlog is a bounded queue of connections that have
completed the TCP handshake but haven't been `accept()`-ed yet. If your
accept loop falls behind (e.g. blocked doing something else), the backlog
fills and the kernel starts dropping or resetting new SYNs — this shows up
as mysterious client-side connection resets under load, not an obvious
error in your proxy. Sizing the backlog and never blocking the accept loop
matters more than it looks like at small scale.

### shutdown() vs close()
`shutdown(fd, SHUT_WR)` sends a TCP FIN, telling the peer "I'm done
writing", while keeping the fd open for reading — this is how you signal
end-of-request-body without hanging up the whole connection. `close()`
releases the fd entirely. Tokio exposes this as `AsyncWriteExt::shutdown`.
Getting this wrong is a common source of "half the response got cut off"
bugs in a hand-rolled proxy.

### Addresses, and asking a socket about itself
A socket address is a `sockaddr`: family (`AF_INET`, `AF_INET6`, `AF_UNIX`), address and port, in **network byte order** (big-endian) — which is why raw
`libc` code calls `htons`/`htonl` and Rust's `SocketAddr` hides it. Binding to `0.0.0.0` (or `[::]`) listens on every interface; `127.0.0.1` only on loopback,
so a server bound there is unreachable from another machine or from a container's own network namespace
([`02-linux/19-netfilter-and-linux-networking.md`](../02-linux/19-netfilter-and-linux-networking.md)). Port `0` asks the kernel to pick a free port (`local_addr()` then
tells you which) — the idiom for tests. `getsockname` returns *your* end of a connection and `getpeername` the *other* end (`peer_addr()`); behind a NAT or
load balancer the peer is the middlebox, not the client ([`15-http.md`](15-http.md)).

### Non-blocking connect and the buffers behind a socket
`connect()` on a non-blocking socket returns `EINPROGRESS` immediately; the handshake finishes in the background and the socket becomes **writable** when it is
done (check `SO_ERROR` for failure) — which is exactly what tokio's `TcpStream::connect(..).await` wraps, and why a connect *timeout* has to be added by you
([`02-linux/11-time-and-timers.md`](../02-linux/11-time-and-timers.md)). Every socket has a kernel **send buffer** and **receive buffer**: `write` succeeds when bytes fit in the send buffer
(not when the peer got them, [`12-tcp.md`](12-tcp.md)); `read` drains the receive buffer. They are sized by `SO_SNDBUF`/`SO_RCVBUF`, but setting them disables Linux's
autotuning ([`13-tcp-reliability.md`](13-tcp-reliability.md)) — leave them alone unless measuring.

### Backlog, in numbers
`listen(fd, backlog)` sets the accept-queue size, but the kernel **silently clamps it to `net.core.somaxconn`** (4096 on recent kernels, 128 on old ones), so
`listen(1024)` may give less than you asked. Watch it: `ss -ltn` shows `Recv-Q` (connections currently waiting) against `Send-Q` (the effective limit)
([`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md)); `nstat -az TcpExtListenOverflows TcpExtListenDrops` counts connections dropped because the queue was full. A growing
overflow counter means the accept loop is too slow ([`04-runtime/`](../04-runtime)), not that the network is bad.

### SO_LINGER and what `close` really does
By default `close()` returns immediately and the kernel keeps sending any unsent data in the background, then does the orderly FIN. `SO_LINGER` with a
nonzero timeout makes `close` block until the data is sent or the timeout passes; with **zero** timeout it aborts the connection with an immediate **RST**,
discarding unsent data ([`12-tcp.md`](12-tcp.md)) — occasionally used to shed misbehaving clients without piling up `TIME_WAIT`, dangerous otherwise.

## Practice

1. Trace `strace -f` on a simple `nc -l` session and identify the
   `socket`/`bind`/`listen`/`accept` syscalls in order.
2. Implement [`labs/00-tcp-server`](../../labs/00-tcp-server) using tokio's `TcpListener`, then
   compare it against the raw-`libc`-epoll echo server you'll build in
   [`02-linux/14-epoll.md`](../02-linux/14-epoll.md)'s exercise (a scratch project, not part of this
   workspace) — same behavior, very different code.
3. In that raw-epoll version, deliberately try a non-blocking `read()`
   before data is ready and confirm you get `EAGAIN`; handle it correctly
   instead of treating it as an error.
4. Set `SO_REUSEADDR` on your echo server and verify (via
   `SIGKILL` + immediate restart) that it no longer fails with
   "Address already in use".
5. Overwhelm your own accept loop on purpose (sleep before each `accept`)
   and use `ss -ltn` to watch the backlog queue fill up.
6. Bind to port `0`, print `local_addr()`, then compare `ss -ltn` for
   `listen(5)` vs `listen(65535)` against `sysctl net.core.somaxconn`, and
   confirm the effective `Send-Q` is the clamped value.
