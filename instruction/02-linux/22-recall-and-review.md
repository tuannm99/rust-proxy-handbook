# Recall and Review: Making the OS Material Stick

Reading a file once produces the feeling of understanding, not the memory of
it. OS topics are especially easy to forget because they look like unrelated
facts (fork, inode, cgroup, futex...) — memory sticks when they hang off a
structure. This file is the retrieval kit for `02-linux/`: a skeleton, questions
per file, drawings to reproduce from memory, and predict-then-run experiments.
It teaches nothing new; it makes you use what the other files taught. The method
(cover-answer-check, derive before re-reading, spaced reviews at 1/3/7/21 days)
is the same as in [`01-network/22-recall-and-review.md`](../01-network/22-recall-and-review.md) and
[`00-introduction/03-study-loop.md`](../00-introduction/03-study-loop.md).

## What to learn

### The one question that organizes everything
*How do many programs safely share one machine?* Each topic is an answer for one
resource: CPU -> processes and the scheduler; RAM -> virtual memory; disk -> files;
network card -> sockets; other programs -> IPC and signals; "who may do what" ->
users and capabilities; "how much may you use" -> rlimits and cgroups; "what can you
see" -> namespaces. When a fact feels loose, ask: *which resource, and what is the
OS multiplexing, abstracting or protecting here?* ([`01-fundamentals.md`](01-fundamentals.md))

### The skeleton: fifteen facts to hold in your head
1. The **kernel** is the only code that touches hardware; everything else runs in **user space**
   and crosses to the kernel only through **syscalls**. [`01-fundamentals.md`](01-fundamentals.md), [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)
2. A syscall costs a mode switch (and often a context switch), so **syscall count matters**;
   a **context switch** also costs warm CPU caches. [`02-hardware-basics.md`](02-hardware-basics.md)
3. The CPU enforces privilege and the **MMU** enforces memory isolation; a bad access is a
   **page fault**. [`02-hardware-basics.md`](02-hardware-basics.md), [`16-memory.md`](16-memory.md)
4. A **process** = private address space + fds; a **thread** shares them; tokio **tasks** are
   neither. [`03-processes-and-threads.md`](03-processes-and-threads.md)
5. `fork` copies (copy-on-write), `exec` replaces, `wait` reaps; an unreaped child is a **zombie**;
   in a container **PID 1** is special. [`04-process-lifecycle.md`](04-process-lifecycle.md)
6. Almost everything is a **file descriptor**: an index to a kernel object; one **epoll** can wait
   on all of them. [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md), [`14-epoll.md`](14-epoll.md)
7. A file is an **inode**; names are directory entries; `write` lands in the **page cache**, durable
   only after `fsync`; replace atomically with write-temp + `rename`. [`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)
8. Permissions are checked against **effective UID/GID**; root bypasses them; **capabilities** slice
   root up (`CAP_NET_BIND_SERVICE` for port 443). [`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)
9. Memory is **virtual**, **lazy** (allocated on first touch) and **reclaimable**; RSS ≠ VSZ;
   exceeding a cgroup limit means **OOM kill** (exit 137). [`08-memory-basics.md`](08-memory-basics.md), [`16-memory.md`](16-memory.md)
10. A blocking syscall parks the thread; **event loops** exist so one thread serves many connections;
    **signals** interrupt from outside. [`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md), [`17-signals.md`](17-signals.md)
11. IPC choices: pipe, **Unix socket** (can pass fds with `SCM_RIGHTS` -> hot restart), shared
    memory (fast, you synchronise), **futex** under every `Mutex`. [`10-ipc.md`](10-ipc.md)
12. **Monotonic** time for durations, **wall-clock** for human timestamps; every wait needs a
    **timeout** and ideally a deadline. [`11-time-and-timers.md`](11-time-and-timers.md)
13. The scheduler keeps **run queues**; I/O-bound tasks get good latency; a cgroup **CPU limit**
    throttles whole processes. [`12-cpu-scheduling.md`](12-cpu-scheduling.md)
14. A **container** = namespaces (what you see) + cgroups (what you may use) on a **shared kernel**,
    not a VM. [`13-containers.md`](13-containers.md)
15. A service has a **contract** with its supervisor: foreground, log to stdout, handle `SIGTERM`,
    stay under `RLIMIT_NOFILE`, report readiness. [`20-limits-and-proc.md`](20-limits-and-proc.md), [`21-systemd-and-services.md`](21-systemd-and-services.md)

### Question bank, by file
Answer each without notes, then check the arrow. Starred (*) questions are the ones
people most often get wrong.

**01 fundamentals** ([`01-fundamentals.md`](01-fundamentals.md))
- What three jobs does an OS do? Is Linux an OS or a kernel?
- Narrate `./proxy` from keypress to exit in six steps. *

**02 hardware** ([`02-hardware-basics.md`](02-hardware-basics.md))
- What makes a syscall different from a normal function call, at the CPU level?
- Trace a network frame from the wire to your `read()` return. Where are the copies? *
- What are `us`, `sy`, `si`, `wa`, `st` in `top`, and which would you check for a proxy at 100% CPU?

**03 processes and threads** ([`03-processes-and-threads.md`](03-processes-and-threads.md))
- What does a thread share with its siblings, and what is private to it?
- Why is a tokio task cheaper than a thread? What are the "two stacked schedulers"?

**04 process lifecycle** ([`04-process-lifecycle.md`](04-process-lifecycle.md))
- What do `fork` and `exec` each do, and why are they separate calls?
- What is a zombie, who creates one, and what does exit code 137 mean? *
- Why does a proxy running as PID 1 ignore `SIGTERM`? Fix it two ways.

**05 kernel and syscalls** ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md))
- Why do three `write`s cost more than one `writev`?
- If two processes both hold "fd 5", are they the same thing? When does an object really close? *
- Root vs kernel mode: what is the difference?

**06 filesystem** ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md))
- What is stored in an inode and what is not? Why can `df` say full while `du` says not? *
- Does a successful `write` mean the data is on disk? What does `fsync` add?
- Write the steps to update a config file atomically. Why must the temp file be in the same directory?
- Why is open-then-`fstat` safer than `stat`-then-open?

**07 users and capabilities** ([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md))
- Which UID do permission checks use? What does the `x` bit mean on a directory?
- How can a proxy bind port 443 without running as root? Name three ways. *
- In what order do you drop privileges, and why must you check every return value?

**08 memory basics** ([`08-memory-basics.md`](08-memory-basics.md))
- Why can two processes both use address `0x1000` without conflict?
- Order registers, L1, L3, RAM, SSD, network by latency, with rough numbers.

**09 blocking I/O and signals** ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md))
- What happens to a thread inside a blocking `read`? Why does thread-per-connection stop scaling?
- Why is doing real work inside a signal handler dangerous?

**10 IPC** ([`10-ipc.md`](10-ipc.md))
- Pipe vs Unix socket vs shared memory: when each?
- How does a hot restart keep the listening port open across a binary swap? *
- What does an uncontended `Mutex` cost compared with a contended one, and why?

**11 time** ([`11-time-and-timers.md`](11-time-and-timers.md))
- Which clock for a timeout, which for a certificate expiry check, and what goes wrong if you swap them? *
- Why does tokio keep its own timer wheel? What does dropping a `timeout`-ed future do?
- Idle timeout vs total deadline: what attack does each stop?

**12 CPU scheduling** ([`12-cpu-scheduling.md`](12-cpu-scheduling.md))
- What is run-queue wait and why can't efficient code remove it?
- A container with `cpus=2` on a 64-core host runs 64 tokio workers. What happens? *
- High load average but low CPU use: what is the likely cause?

**13 containers** ([`13-containers.md`](13-containers.md))
- What do namespaces limit and what do cgroups limit? Does a container have its own kernel?
- Why does `127.0.0.1` inside a container not reach a host service?

**14 epoll** ([`14-epoll.md`](14-epoll.md))
- Level- vs edge-triggered: what must you do differently with edge-triggered? *
- Why can a regular file not be made non-blocking with epoll?

**15 io_uring** ([`15-io_uring.md`](15-io_uring.md))
- Readiness vs completion model: what is the practical difference?

**16 memory** ([`16-memory.md`](16-memory.md))
- Minor vs major page fault. Why are the first requests after startup slower?
- `free -h` shows little "free" memory. Is that a problem? Which number matters? *
- What does a cgroup OOM kill look like from the outside, and what in a proxy multiplies memory use?

**17 signals** ([`17-signals.md`](17-signals.md))
- What should a proxy do on `SIGTERM` vs `SIGHUP`? Why can't `SIGKILL` be handled?

**18 zero-copy** ([`18-zerocopy.md`](18-zerocopy.md))
- Which copies does `sendfile` remove? Why is zero-copy hard with TLS?

**19 netfilter** ([`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md))
- Name the five hooks. Where does DNAT happen, and where SNAT?
- `DROP` vs `REJECT`: what does the client see for each? *
- What happens when conntrack fills, and what symptom appears?

**20 limits and /proc** ([`20-limits-and-proc.md`](20-limits-and-proc.md))
- A proxy hits `EMFILE`. What happens to the accept loop if you do nothing? Where do you check the limit that is *actually* in force? *
- Which `/proc` files answer: how many fds? how much real memory? which cgroup?

**21 systemd** ([`21-systemd-and-services.md`](21-systemd-and-services.md))
- Describe the stop contract (signals and timeouts). Why does `Type=notify` exist?
- What does socket activation give you for restarts and for privileges?

### Draw it from memory
1. The user/kernel split with the syscall door, and where epoll, sockets and files sit ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)).
2. The path of a packet: NIC, DMA, IRQ/softirq, TCP, socket queue, wake-up, `read` ([`02-hardware-basics.md`](02-hardware-basics.md)).
3. fd table -> open file description -> inode, with `dup`/`fork` sharing ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)).
4. Process states and transitions (running, runnable, sleeping, zombie) ([`12-cpu-scheduling.md`](12-cpu-scheduling.md), [`04-process-lifecycle.md`](04-process-lifecycle.md)).
5. A virtual address space (code, heap, stack, mmaps) mapped to RAM, the page cache and swap ([`16-memory.md`](16-memory.md)).
6. netfilter hooks with the routing decision ([`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md)).
7. Namespaces and cgroups around one process ([`13-containers.md`](13-containers.md)).

### Predict, then run
Write the prediction first; run it; log surprises.
1. `ulimit -n 64`, then open 100 connections to your server — what does the accept loop do?
2. `rm` a log file a process still has open — does `df` change? When?
3. `kill -TERM 1` inside a container with and without a handler — what happens, and what is the exit code?
4. `strace -c` a hello-world HTTP server — which syscalls dominate?
5. Write 1 GiB with and without `fsync` — predict the ratio.
6. Run a CPU-heavy tokio program with `--cpus=1` and 8 workers — what does `cpu.stat` show?

### Gotcha: the OS is a pile of facts only until you can narrate it
The test of understanding is not recalling a definition but narrating a
*story* — "what happens when a request arrives and a worker is blocked on disk?" — using
the skeleton. If you can tell three such stories without notes, the facts will stay.

## Practice
1. Close this file and write the fifteen skeleton facts from memory; score yourself,
   then repeat after 1, 3 and 7 days and record the scores in your learning log.
2. Turn every starred question you missed into a flashcard with a link to the section.
3. Without notes, tell aloud the story of "a request arrives at [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy)
   while one worker thread is blocked reading a file", naming the syscall, kernel mechanism
   and scheduler action at each step, while drawing items 1, 2 and 4 above.
4. Run three "Predict, then run" experiments and write what each taught you that reading hadn't.
