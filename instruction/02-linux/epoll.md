# epoll

Edge-trigger vs level-trigger, event loop.

## What to learn

### Why epoll exists
`select`/`poll` re-scan every watched fd on every call — O(n) per wake-up
regardless of how many fds are actually ready. A proxy holding tens of
thousands of idle keep-alive connections pays that cost constantly. `epoll`
keeps the watch list inside the kernel (`epoll_create1`) and only returns the
fds that became ready, so the cost is proportional to *ready* fds, not
*watched* fds.

### The three syscalls
```rust
// epoll_create1(0)  -> epoll fd
// epoll_ctl(epfd, EPOLL_CTL_ADD/MOD/DEL, fd, &event) -> register/modify/remove
// epoll_wait(epfd, &mut events, max, timeout_ms)     -> block until ready
use libc::{epoll_event, EPOLLIN, EPOLLOUT};

let mut ev = epoll_event { events: (EPOLLIN | EPOLLOUT) as u32, u64: fd as u64 };
```
The `u64` field (or `ptr` in the union) is your only correlation between the
event and "which connection is this" — in practice you stash a token/index
into it and look up connection state in a slab/`Vec`.

### Level-triggered vs edge-triggered
Level-triggered (the default) fires every time you call `epoll_wait` as long
as the fd *is* readable/writable — safe but can spin if you don't drain the
fd. Edge-triggered (`EPOLLET`) fires only on the *transition* to
readable/writable, which means you **must** read/write in a loop until you
get `EWOULDBLOCK`/`EAGAIN`, or you'll miss data that arrived after your last
partial read and the fd will never notify you again. Gotcha: mixing
level-triggered assumptions with `EPOLLET` is the single most common source
of "connection randomly hangs forever" bugs in hand-rolled event loops.

### The event loop pattern
```
register listener with EPOLLIN
loop:
    n = epoll_wait(epfd, events, MAX, -1)
    for each ready event:
        if it's the listener -> accept() in a loop until EAGAIN, register each new fd
        else -> read/write in a loop until EAGAIN, update per-connection state machine
```
Every ready fd maps to a small state machine (reading headers, reading body,
writing response...) — the event loop itself does no protocol logic, it just
drives whichever state machine is attached to that fd's token.

### Why tokio's reactor exists
Tokio's reactor is exactly this event loop, generalized: one thread (or a
small pool) owns the epoll fd, and every `.await` on a socket registers a
waker against a token instead of blocking. See `03-rust/async.md` and
`04-runtime/tokio.md` — understanding raw epoll first makes tokio's
`Poll::Pending` / waker dance click, because it's the same registration model
with the polling loop and bookkeeping hidden from you.

### EAGAIN handling
Non-blocking sockets return `EWOULDBLOCK`/`EAGAIN` instead of blocking when
there's no data/buffer space. In edge-triggered mode this is not an error —
it's your loop-exit signal. Forgetting to set `O_NONBLOCK` on accepted
sockets (only the listener being non-blocking isn't enough) is a classic bug
that silently reverts you to blocking I/O per-connection.

## Practice
1. In a scratch project (not part of this workspace — there's no dedicated lab for raw epoll here), implement a non-blocking listener and register it with `epoll_create1`/`epoll_ctl`.
2. Run the echo server in level-triggered mode first; confirm it works under `nc` and a simple concurrent load generator.
3. Switch to `EPOLLET` and reproduce a stuck connection by *not* looping to EAGAIN on read — observe the hang, then fix it.
4. Add a second registered fd (e.g. a pipe used as a shutdown signal) and dispatch on it inside the same loop.
5. Compare `strace -c` syscall counts between your epoll version and a naive `select`-based version under 1000 idle connections.
6. Once done, read tokio's reactor source (`tokio::runtime::io`) and identify where it calls the same three syscalls you just used by hand.
