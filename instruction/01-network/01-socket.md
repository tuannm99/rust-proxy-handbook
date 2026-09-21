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
(see `02-linux/01-epoll.md`'s exercise) comprehensible instead of magic.

### Blocking vs non-blocking
A blocking socket's `read`/`write`/`accept` parks the calling thread until
data/connections are ready. A non-blocking socket returns `EWOULDBLOCK`/
`EAGAIN` immediately instead — which is the foundation an event loop
(epoll, see `02-linux/01-epoll.md`) is built on: register the fd, block on
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

## Practice

1. Trace `strace -f` on a simple `nc -l` session and identify the
   `socket`/`bind`/`listen`/`accept` syscalls in order.
2. Implement `labs/00-tcp-server` using tokio's `TcpListener`, then
   compare it against the raw-`libc`-epoll echo server you'll build in
   `02-linux/01-epoll.md`'s exercise (a scratch project, not part of this
   workspace) — same behavior, very different code.
3. In that raw-epoll version, deliberately try a non-blocking `read()`
   before data is ready and confirm you get `EAGAIN`; handle it correctly
   instead of treating it as an error.
4. Set `SO_REUSEADDR` on your echo server and verify (via
   `SIGKILL` + immediate restart) that it no longer fails with
   "Address already in use".
5. Overwhelm your own accept loop on purpose (sleep before each `accept`)
   and use `ss -ltn` to watch the backlog queue fill up.
