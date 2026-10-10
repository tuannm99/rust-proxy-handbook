# Hardware Basics: CPU, Interrupts, DMA, and How a Packet Reaches Your Code

The machine the kernel manages. Not computer architecture as a field — just the
parts you need so that "syscall," "interrupt," "context switch," and "zero-copy"
stop being magic words. Part of the from-scratch fundamentals series; see
[`01-fundamentals.md`](01-fundamentals.md) for the index.

## What to learn

### The CPU: fetch, execute, and a ladder of speeds
A CPU core repeatedly fetches an instruction, decodes it and executes it,
holding working values in a few dozen **registers**. Everything else is
slower and farther away: L1 cache (~1 ns), L2/L3 (~4–40 ns), main memory (~100 ns),
SSD (~100 µs), network (~100 µs–100 ms) — see [`08-memory-basics.md`](08-memory-basics.md) for the
hierarchy and [`17-performance/`](../17-performance) for what it does to your data
structures. A modern CPU has several **cores** (independent execution units) and
often **hyperthreads** (two hardware threads sharing one core's resources). The
kernel sees each hardware thread as a "CPU" (`nproc` counts them), and tokio's
default worker count is exactly that number ([`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md)).
Machine code is specific to an **instruction set** (x86-64, aarch64), which is why
you cross-compile and why SIMD code is per-architecture.

### Privilege levels: why the kernel can do what you can't
The CPU itself has modes. Kernel code runs in a privileged mode (ring 0 on x86)
where it can execute special instructions (talk to devices, edit page tables, mask
interrupts) and touch any memory; user code runs unprivileged (ring 3) and *any*
attempt to run a privileged instruction or touch memory not mapped for it makes the
CPU **trap** into the kernel. A **syscall** is a deliberate, controlled trap: a
special instruction (`syscall`) switches the CPU into kernel mode at a fixed entry
point ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)). The process isolation of
[`03-processes-and-threads.md`](03-processes-and-threads.md) is enforced by hardware (the **MMU**, below), not by good
manners.

### The MMU: virtual addresses are a hardware feature
Every address your program uses is **virtual**. The **MMU** translates each one
to a physical address on every access, using page tables the kernel maintains, in
fixed-size **pages** (4 KiB typically). A **TLB** caches recent translations; a miss
costs a table walk. If a page isn't mapped or isn't allowed (write to read-only, user
touching kernel memory), the CPU raises a **page fault** and the kernel decides:
load it, allocate it, or kill the process with `SIGSEGV`
([`16-memory.md`](16-memory.md)). Isolation, `mmap`, copy-on-write `fork`, lazy allocation
and memory-mapped files are all this one mechanism.

### Interrupts: the device taps the CPU on the shoulder
A CPU can't poll every device. Instead devices raise an **interrupt**: the CPU
stops what it's doing, saves its state, and jumps to a kernel **interrupt handler**,
then resumes. The handler is kept tiny — acknowledge the device, schedule the real
work as a **softirq** — because interrupts disable further ones
([`16-kernel/04-interrupt.md`](../16-kernel/04-interrupt.md)). Besides devices, a **timer interrupt**
fires (hundreds or thousands of times per second) giving the kernel the periodic
chance to preempt the running task ([`12-cpu-scheduling.md`](12-cpu-scheduling.md)); without it a
looping process would own its core forever. Interrupts and traps are how the
outside world and your own faults get the kernel's attention; **signals**
([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)) are the kernel relaying such events *up* to your process.

### DMA and the network card: how bytes arrive without the CPU copying them
A **NIC** (network interface card) has hardware **ring buffers** in RAM. When a frame
arrives, the NIC writes it straight into a pre-allocated memory buffer using
**DMA** (direct memory access — a device writing to RAM without the CPU moving each
byte), advances the ring, and raises an interrupt (or the kernel polls, under load —
**NAPI**). The kernel's network stack then processes the frame ([`01-network/07-link-layer.md`](../01-network/07-link-layer.md)
-> IP -> TCP), puts the payload on the socket's receive queue, and **wakes the
process** waiting in `epoll_wait`/`read`. Only then does your code run, and `read()`
copies the bytes from the kernel's buffer into yours. On send, the reverse: your
`write` copies into the socket buffer, TCP segments it, and the NIC DMAs frames out.

```text
wire -> NIC -> DMA into ring buffer -> IRQ/NAPI -> softirq: IP+TCP processing
     -> socket receive queue -> wake epoll_wait -> your read() copies to user buffer
```

Two consequences: every byte you proxy is copied at least twice
(kernel->user on read, user->kernel on write), which is what
[`18-zerocopy.md`](18-zerocopy.md) tries to eliminate; and the interrupt/softirq work happens on
whichever core the NIC's queue is bound to, which is what RSS/RPS
([`16-kernel/05-rss.md`](../16-kernel/05-rss.md), [`16-kernel/06-rps.md`](../16-kernel/06-rps.md)) tune.

### Storage, briefly
Disks and SSDs are **block devices**: read/written in fixed-size blocks, orders of
magnitude slower than RAM, and with a large gap between sequential and random access
(far larger on spinning disks). The kernel hides this with the **page cache** (RAM
holds recently used file data) and writes back **lazily**
([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)). A proxy mostly avoids the disk — except for logs,
cache files and static assets — which is why an unexpected `write()` to a slow log
file can stall a request path.

### What a context switch is
Switching a core from one thread to another means saving the old thread's
registers and instruction pointer, switching the page tables to the new thread's
process (if different), and restoring the new thread's registers — plus the invisible
cost afterward: the new thread's data isn't in the CPU caches or TLB, so it runs
slowly until they warm up. A syscall by itself switches *mode*, not thread; a **context
switch** changes *which thread runs*. Direct cost is a few microseconds; the cache
penalty is often larger. This is the real reason thread-per-connection loses to an
event loop ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)) at tens of thousands of connections.

### Gotcha: "the CPU is busy" is four different things
`top`'s CPU line splits time into `us` (user code), `sy` (kernel: syscalls, network
stack), `si`/`hi` (soft/hard interrupts), `wa` (waiting on disk I/O), and `st`
(stolen by the hypervisor in a VM). A proxy at 100% of one core with high `si` is
bottlenecked on packet processing (RSS/RPS, [`16-kernel/`](../16-kernel)), not your Rust code;
high `sy` points at syscall overhead ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)); high `st`
means a noisy neighbor. Look at the split before optimizing anything.

## Practice

1. Run `lscpu` and `nproc`; identify sockets, cores per socket, threads per core,
   cache sizes, and NUMA nodes, then compare `nproc` against the number of worker
   threads tokio starts in [`labs/00-tcp-server`](../../labs/00-tcp-server).
2. Read `/proc/interrupts` twice, ten seconds apart, while running a `curl` loop
   against a local server (or `iperf3` between hosts); find the NIC's (or `lo`'s
   softirq) rows and see which CPU's counters moved. Then read `/proc/softirqs` and
   find `NET_RX` and `TIMER`.
3. Run `vmstat 1` and read `cs` (context switches/second) and `in` (interrupts/second)
   while idle, then under a load test of [`labs/00-tcp-server`](../../labs/00-tcp-server); compare a
   thread-per-connection variant (a scratch program using `std::thread::spawn`) to your
   tokio one at 1,000 concurrent connections.
4. Run `mpstat -P ALL 1` (or `top` then `1`) under load and read the `%usr`, `%sys`,
   `%soft` split per core; note which core handles network softirqs.
5. Use `perf stat -e context-switches,cpu-migrations,page-faults ./target/release/<bin>`
   on a small program to see context switches and page faults as counters
   ([`12-testing/05-debugging.md`](../12-testing/05-debugging.md)).
6. Time sequential vs random access on a 1 GiB array in a scratch Rust program and
   explain the gap in terms of the cache/TLB hierarchy above.
