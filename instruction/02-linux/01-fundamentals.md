# Linux/OS Fundamentals

Lesson one of the OS path, and the one every later file leans on. It
assumes nothing: if you already know what an operating system does, skim it
in twenty minutes; if not, read it twice and do the Practice before moving
on. An OS looks like a hundred unrelated topics (processes, memory, files,
signals, containers) until you see that they are all answers to one
question — *how do many programs safely share one machine?* — so this file
gives you that question and the map first.

## What to learn

### What an operating system is for
A computer has a few physical resources: CPU cores, RAM, disks, network
cards, a clock. Many programs want them at once, and none should be able to
wreck the others. The **operating system** does three jobs:

1. **Multiplex** each resource among many programs (give each the illusion of
   its own CPU and its own memory).
2. **Abstract** the hardware behind simple, uniform ideas, so you never program
   a disk controller or network card directly.
3. **Protect**: stop one program from reading or corrupting another's memory
   or the kernel itself.

Every topic in this directory is one row of this table:

| Hardware | What the OS turns it into | You use it through | Read |
|---|---|---|---|
| CPU | **process / thread**, scheduled in turns | `fork`, `clone`, `exec` | [`03-processes-and-threads.md`](03-processes-and-threads.md), [`04-process-lifecycle.md`](04-process-lifecycle.md), [`12-cpu-scheduling.md`](12-cpu-scheduling.md) |
| RAM | **virtual memory**: a private address space | `mmap`, `brk`, plain pointers | [`08-memory-basics.md`](08-memory-basics.md), [`16-memory.md`](16-memory.md) |
| Disk | **files** and directories | `open`, `read`, `write` | [`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md) |
| Network card | **sockets** | `socket`, `bind`, `connect` | [`01-network/11-socket.md`](../01-network/11-socket.md) |
| Clock | time and timers | `clock_gettime`, `timerfd` | [`11-time-and-timers.md`](11-time-and-timers.md) |
| Other programs | pipes, signals, shared memory | `pipe`, `kill`, `mmap` | [`10-ipc.md`](10-ipc.md), [`17-signals.md`](17-signals.md) |
| Who may do what | users, permissions | `chmod`, capabilities | [`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md) |
| Limits and isolation | rlimits, **namespaces, cgroups** | `setrlimit`, containers | [`20-limits-and-proc.md`](20-limits-and-proc.md), [`13-containers.md`](13-containers.md) |

### The software stack, and what "Linux" means
From bottom to top: **hardware**, the **kernel**, **system libraries**
(`libc` wraps syscalls into functions like `read()`), and **programs** (your
shell, `curl`, your proxy). Strictly, **Linux is only the kernel**; a
*distribution* (Ubuntu, Debian, Alpine) bundles that kernel with `libc`, a
package manager, and thousands of programs. This explains real-world
differences: Alpine uses `musl` instead of `glibc` (so DNS and some
behavior differ, [`01-network/14-dns.md`](../01-network/14-dns.md)), a container ships its own userland but
shares the host's kernel ([`13-containers.md`](13-containers.md)), and `uname -r` shows the *kernel*
version while `cat /etc/os-release` shows the *distro*.

### A program's whole life, from `./proxy` to exit
This is the story the rest of the directory zooms in on:

1. You type `./proxy`. The **shell** (a normal program) `fork`s a copy of itself,
   and the child `exec`s `proxy` ([`04-process-lifecycle.md`](04-process-lifecycle.md)).
2. The **kernel** reads the executable file, sets up a fresh private **address
   space**, maps the program's code into it ([`08-memory-basics.md`](08-memory-basics.md)), and makes a
   thread runnable.
3. The **dynamic linker** loads shared libraries (`ldd ./proxy` lists them); then
   `main` runs. Rust's runtime and tokio start worker **threads**.
4. The program makes **syscalls** to do anything real — `socket`, `bind`,
   `epoll_wait`, `read`, `write` — each crossing into the kernel and back
   ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)); results are **file descriptors**.
5. When it waits for I/O, it **blocks** and the scheduler runs something else
   ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)); when data arrives, an **interrupt** wakes it
   ([`02-hardware-basics.md`](02-hardware-basics.md)).
6. A **signal** (`SIGTERM`) asks it to stop; it drains and calls `exit`
   ([`17-signals.md`](17-signals.md)). The kernel frees its memory and fds; the parent
   `wait`s for the exit status ([`04-process-lifecycle.md`](04-process-lifecycle.md)).

Three ideas explain most of what follows, so memorise them: **(1)** user space
and kernel space are separated, and the only door between them is the syscall;
**(2)** almost everything — files, sockets, pipes, timers — is a **file
descriptor**, so one mechanism (`epoll`) can wait on all of it; **(3)** the
kernel *multiplexes* — each process sees its own CPU and memory, which is an
illusion the OS maintains and which has costs (context switches, page faults)
you can measure.

### What the kernel is for
The **kernel** is the one program on the machine allowed to talk to
hardware directly, manage memory across all processes, and enforce
isolation between them. Everything else — your proxy, your shell, every
other process — runs in **user space**, with no direct access to
hardware and no ability to touch another process's memory. Almost every
theme in [`02-linux/`](.) is some consequence of that one split: what crossing
it costs ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)), what it isolates
([`03-processes-and-threads.md`](03-processes-and-threads.md), [`13-containers.md`](13-containers.md)), and what happens
when the kernel needs to interrupt you rather than wait to be asked
([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)).

### The twelve pieces, and where each lives
Read them in this order; each builds on the ones above it.

- **[`02-hardware-basics.md`](02-hardware-basics.md)** — the machine itself: CPU privilege levels,
  the MMU, interrupts, DMA, and how a packet from the network card ends up
  in your `read()`.
- **[`03-processes-and-threads.md`](03-processes-and-threads.md)** — what a process and a thread
  actually are, why threads are cheaper, and why sharing memory between
  them is the reason [`03-rust/04-sync.md`](../03-rust/04-sync.md) exists.
- **[`04-process-lifecycle.md`](04-process-lifecycle.md)** — `fork`, `exec`, `wait`, zombies, orphans
  and why PID 1 in a container is special.
- **[`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)** — the user space/kernel space boundary,
  what a syscall costs, and file descriptors — the integer handle
  everything in [`02-linux/14-epoll.md`](14-epoll.md) is built around.
- **[`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)** — inodes, paths, the page cache, what
  `write` does and doesn't guarantee, atomic file replacement.
- **[`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)** — who a process *is* to the
  kernel, permission bits, capabilities, and binding port 443 without root.
- **[`08-memory-basics.md`](08-memory-basics.md)** — the minimum needed to make
  [`02-linux/16-memory.md`](16-memory.md)'s opening paragraph land as familiar rather
  than new, plus where RAM sits relative to cache and disk.
- **[`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)** — why a syscall can block a thread,
  why event loops exist as the alternative, and what a signal is (an
  interruption from outside your normal control flow, not a return
  value).
- **[`10-ipc.md`](10-ipc.md)** — pipes, Unix sockets, passing file descriptors
  between processes, shared memory, and how a `Mutex` really waits (`futex`).
- **[`11-time-and-timers.md`](11-time-and-timers.md)** — monotonic vs wall-clock time, how timers
  and timeouts work, and why every wait in a proxy needs a deadline.
- **[`12-cpu-scheduling.md`](12-cpu-scheduling.md)** — run queues, priorities, affinity, load
  average, and CPU-limit throttling in containers.
- **[`13-containers.md`](13-containers.md)** — what a container actually is (isolated
  processes on one shared kernel, not a tiny VM), since
  Kubernetes/pods/cgroups are referenced constantly from
  [`09-architecture/`](../09-architecture) onward with nothing ever defining them.

Read them in that order once; after that, treat each as a standalone
lookup. The Kernel mechanisms files after them
([`14-epoll.md`](14-epoll.md) onward) also include the operational layer — Linux networking
and netfilter ([`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md)), resource limits and
`/proc` ([`20-limits-and-proc.md`](20-limits-and-proc.md)), and running as a service
([`21-systemd-and-services.md`](21-systemd-and-services.md)).

## Practice
1. Run `uname -a` and read your kernel version; run `ps aux` and pick
   three processes — for each, guess (then verify with `man`/docs)
   roughly what it does.
2. Read the twelve sibling files in order, then come back here and explain,
   in one sentence each: why a thread is cheaper than a process, what a
   syscall actually crosses, why `read()` can block, and what a container
   isolates that a plain process doesn't. Then narrate, without notes, what
   happens from a packet arriving at the network card to your `read()`
   returning ([`02-hardware-basics.md`](02-hardware-basics.md)).
3. Run `cat /proc/version` and `cat /proc/cpuinfo | grep -c processor` —
   confirm you can find your kernel version and core count without a GUI
   tool.
4. Run `strace -f ls 2>&1 | head -30` and, using the "life of a program" list above, find the `execve`, the library loading (`openat` of `.so` files), and the first real work (`write`); then `ldd $(which ls)` and `cat /proc/self/maps | head` to see the libraries and address space you just read about.
5. Without notes, tell the story of `./proxy` from keypress to exit in six steps, naming the syscall or kernel mechanism in each; check it against this file.
