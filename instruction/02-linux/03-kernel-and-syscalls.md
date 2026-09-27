# Kernel Space, Syscalls, and File Descriptors

Part of the from-scratch fundamentals series — see `02-linux/01-fundamentals.md`
for the full index.

## What to learn

### Crossing the boundary
When your program needs something only the kernel can do — read a file,
send a packet, allocate memory, create a socket — it makes a
**syscall**: a controlled, well-defined request that switches the CPU
into kernel mode, lets the kernel do the privileged work, and switches
back. `bind()`, `listen()`, `accept()`, `read()`, `write()` — everything
in `01-network/07-socket.md` — are syscalls, or thin wrappers around
them.

```rust
// this innocuous-looking call is a user-space -> kernel-space round trip
let n = socket.read(&mut buf).await?;
```

### What a syscall actually costs
Each syscall costs a **context switch** — real, measurable CPU time
spent switching modes (and often switching which thread is actually
running), independent of whatever useful work the syscall does. The CPU
has to save your program's state, switch into a privileged mode with a
different set of memory permissions, validate the request, do the work,
and switch back. None of that is "free" the way a plain function call
is, even though from Rust's point of view a syscall wrapper *looks* like
any other function call.

This is the concrete reason `01-network/07-socket.md`, `02-linux/11-zerocopy.md`,
and the vectored-I/O discussion in that file care about syscall *count*,
not just the bytes moved — `writev` with three buffers costs one context
switch; three separate `write` calls cost three. At high request rates,
syscall overhead is a real, measurable fraction of total CPU time, which
is exactly what `08-observability/04-profiling.md`'s flamegraphs show you
when a proxy's profile is dominated by syscall frames rather than your
own logic.

### File descriptors: everything is a number
Unix's unifying idea is that almost everything you can read from or write
to — a regular file, a TCP socket, a pipe, a timer, an eventfd — is
represented the same way to your process: a small non-negative integer
called a **file descriptor (fd)**. It's an index into a table the kernel
keeps *per process*, and each entry points to the kernel's real object
(an open file, a socket's connection state, etc.) along with an offset
and some flags.

This uniformity is why `02-linux/07-epoll.md` can register a listening
socket, a client socket, *and* a plain pipe on the same `epoll` instance
with the same API — as far as epoll is concerned, they're all just fds
that can become "ready." It's also why `06-proxy/01-upstream.md` and
`07-security/09-ddos.md` talk about fd limits (`ulimit -n`) as a hard
resource ceiling: every open socket, in or out, consumes one entry in that
per-process table, and the table has a configured maximum.

A process starts with three fds already open by convention: `0` (stdin),
`1` (stdout), `2` (stderr) — every fd your program opens afterward gets
the next free number, reused once closed.

```rust
use std::os::unix::io::AsRawFd;
let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
println!("fd = {}", listener.as_raw_fd()); // just an integer
```

### Kernel objects live independently of the fd number
The fd number is *local to your process* — two different processes can
each have "fd 5" open, pointing at completely unrelated kernel objects.
What actually matters is what the kernel object behind the number is and
how many fds (in this or other processes, after a `fork()`) currently
reference it — the object is only fully released once the last reference
is closed. This is background for a gotcha you'll meet directly in
`09-architecture/04-graceful-shutdown.md`: dropping your handle to a
socket doesn't necessarily mean the underlying connection tears down
instantly if something else still references it.

### Kernel mode vs a privileged user (root) — different axes
Worth disentangling since both get called "privileged": **kernel vs user
mode** is a CPU-level distinction about which instructions and memory are
allowed — every process, including ones run by root, executes in user
mode and must syscall into the kernel for privileged operations. **Root
vs non-root** is a kernel-enforced *permission* distinction entirely
within user mode — root's processes still run in user mode and still
make the same syscalls, but the kernel's permission checks (e.g. "may
this process bind port 443," `01-network/02-addressing.md`'s
well-known-ports note) let more of those syscalls succeed.

## Practice
1. Run `ls /proc/self/fd` in a shell (or write a small Rust program,
   print `std::process::id()`, and inspect `/proc/<pid>/fd` from another
   terminal while it runs) and confirm fds 0/1/2 are present; open a file
   and a TCP connection in the program and watch new numbered entries
   appear.
2. `strace -c` a run of `labs/00-tcp-server` handling a few requests and
   read the summary table — identify which syscalls dominate the count,
   and connect at least three of them back to lines in your code.
3. Write a program that opens 5 files without closing any of them, print
   their fd numbers, then close the third one opened and open a new file
   — confirm the new file reuses the number that was just freed.
4. Compare `strace` output for one `write()` of a 3-buffer concatenated
   `Vec<u8>` against `writev` (vectored write, `IoSlice`) of the same
   three buffers unconcatenated — confirm the syscall count differs by
   exactly the amount the text above predicts.
