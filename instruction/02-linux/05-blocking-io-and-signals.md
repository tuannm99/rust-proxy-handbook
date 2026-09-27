# Blocking I/O, Event Loops, and Signals

Part of the from-scratch fundamentals series — see `02-linux/01-fundamentals.md`
for the full index. Two ideas bundled together because they're both about
the same underlying question: how does your program find out that
something it's waiting on has happened?

## What to learn

### Blocking vs non-blocking syscalls
A syscall that has nothing to do yet — `read()` on a socket with no data
available — has two possible behaviors: **block** (the calling thread is
suspended by the kernel and doesn't run again until data arrives — cheap
to write, but that thread does nothing else in the meantime) or
**non-block** (the syscall returns immediately with an error,
`EWOULDBLOCK`/`EAGAIN`, meaning "nothing ready, ask again later").

```rust
// blocking: this line simply doesn't return until data arrives
let n = blocking_socket.read(&mut buf)?;

// non-blocking: returns immediately either way
match nonblocking_socket.read(&mut buf) {
    Ok(n) => { /* got n bytes */ }
    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => { /* nothing yet */ }
    Err(e) => return Err(e),
}
```

### Why one-thread-per-connection doesn't scale
One blocking thread per connection is the simplest possible design and
doesn't scale — thousands of idle keep-alive connections would mean
thousands of OS threads mostly doing nothing, each with its own stack
memory and kernel scheduling overhead (`02-processes-and-threads.md`).
The alternative is non-blocking sockets plus a mechanism to ask the
kernel "tell me which of these thousand fds actually have something
ready" in one call, instead of polling each one yourself — that
mechanism is `epoll` (`02-linux/07-epoll.md`), and it's the entire
foundation tokio's reactor is built on (`04-runtime/01-tokio.md`).

This is the literal reason `labs/00-tcp-server`'s done-criteria insists
on handling 500+ connections without 500+ threads: it's forcing you to
actually feel the difference this section describes, not just read about
it.

### The event loop pattern, one level up from epoll itself
Whether you hand-roll it (the exercise in `02-linux/07-epoll.md`) or let
tokio do it for you, the shape is always: register interest in a set of
fds, block *once* on "tell me when any of them are ready" rather than
blocking per-fd, and dispatch to whatever logic owns each ready fd when
the wait returns. One thread (or a small pool) can service thousands of
connections this way because it's never blocked waiting on any single
one — it's blocked, at most, waiting for *something, anything* to become
ready.

### Signals: the kernel interrupting your process
A **signal** is a notification the kernel delivers to a process
asynchronously — it can arrive between any two instructions, interrupting
whatever the process was doing, unlike a syscall which your code
deliberately initiates. `kill -TERM <pid>` and Ctrl-C both work by
sending a signal. This is a fundamentally different delivery mechanism
than a function return or a syscall result: your program didn't ask "is
there a signal for me?" — the kernel simply preempts it.

### Why naive signal handling is dangerous
This asynchronicity is exactly why signal handling is fragile if done
naively: a handler that runs "whenever, no matter what the process was
doing" cannot safely do most normal things (allocate memory, lock a
mutex) because it might have interrupted code that was already in the
middle of doing exactly that — locking a mutex the handler now also tries
to lock deadlocks the process against itself. The set of operations safe
to perform inside a raw signal handler is called **async-signal-safe**,
and it's a short list that excludes most of what feels like "normal
code."

The idiomatic fix, and what every serious async runtime does: the raw
handler does nothing but write a byte to a pipe/eventfd (or increment an
atomic — both are on the async-signal-safe list), and your actual
reload/shutdown logic runs later, on a normal thread, woken by that
write through the same event-loop mechanism described above. `tokio::signal`
implements exactly this pattern for you; `02-linux/10-signals.md` covers
the specific signals (`SIGHUP`, `SIGTERM`) a proxy cares about and the
API.

### The common thread
Blocking I/O readiness and signal delivery are both instances of "my
program needs to react to something external, at a time it doesn't
control" — the difference is only in mechanism (a syscall you're
positioned to retry, vs. an interruption that finds you wherever you are).
Both end up funneled, in a well-built async program, through the same
event loop: epoll readiness events and signal-derived pipe/eventfd writes
are dispatched by the identical mechanism, which is why `tokio::select!`
can wait on a socket read and a signal in the same expression without
anything special going on underneath.

## Practice
1. Write a program that blocks on `std::net::TcpStream::read` with no
   data arriving, and in another terminal confirm (via `ps` or `top`)
   that the thread is sitting idle rather than spinning — then explain
   why 500 such threads would be a real resource cost even though none of
   them are using CPU.
2. In a scratch project, set a socket non-blocking (`set_nonblocking(true)`)
   and call `read()` on it with no data available — confirm you get
   `WouldBlock` immediately rather than the call hanging.
3. Write a tiny binary that registers `tokio::signal` handlers for
   `SIGHUP` and `SIGTERM` and just prints which one fired; send both with
   `kill -HUP <pid>` / `kill -TERM <pid>` and confirm both are caught
   without killing the process — then compare against sending `SIGKILL`,
   which you cannot catch at all.
4. Read the list of async-signal-safe functions (`man 7 signal-safety`)
   and identify at least two very ordinary-looking operations (e.g.
   `malloc`, `printf`) that are *not* on it — explain, from the mutex
   example above, why calling either from a raw handler is dangerous.
