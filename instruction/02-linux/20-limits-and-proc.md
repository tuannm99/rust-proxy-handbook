# Resource Limits and /proc: What a Process Is Allowed, and How to See What It's Doing

The ceilings the kernel enforces on a process (open files, processes, memory locks, core dumps) and
the `/proc` filesystem through which you can inspect any live process without a debugger. The most common
production failure of a connection-heavy proxy — "too many open files" — lives here.

## What to learn

### rlimits: per-process ceilings
Each process has **resource limits** (`setrlimit`/`getrlimit`, shell `ulimit`), each with a **soft** limit (what the
kernel enforces now) and a **hard** limit (the ceiling an unprivileged process may raise its soft limit to).
The ones that bite a network service:

- **`RLIMIT_NOFILE`** (`ulimit -n`) — max open fds ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)). Historically default **1024**. Every client
  socket, every upstream socket, every file, `epoll` instance, pipe and timer counts. A proxy at 10,000 clients with
  pooled upstreams needs well over 10,000. Hit it and `accept()`/`open()`/`socket()` fail with **`EMFILE`**.
- **`RLIMIT_NPROC`** — processes/threads per user (affects thread pools).
- **`RLIMIT_CORE`** — core dump size (`0` = none). A core file contains the process memory, **including private keys and
  tokens** ([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)); keep disabled or tightly controlled.
- **`RLIMIT_MEMLOCK`** — memory that may be locked in RAM (`mlock`, and io_uring ring memory on older kernels —
  [`15-io_uring.md`](15-io_uring.md)).
- **`RLIMIT_STACK`** — main-thread stack size (8 MiB default); thread stacks are set separately.
- **`RLIMIT_AS`/`RLIMIT_DATA`** — address-space caps; rarely useful with a runtime that reserves large virtual regions —
  use cgroup memory limits instead ([`13-containers.md`](13-containers.md), [`16-memory.md`](16-memory.md)).

`cat /proc/<pid>/limits` shows a running process's *actual* limits — always check this rather than your shell's `ulimit`,
since the service was started by something else.

### System-wide limits and where limits are set
There are also kernel-wide ceilings: `fs.file-max` (total open files, `/proc/sys/fs/file-nr` shows usage), `fs.nr_open` (the
maximum any process's `RLIMIT_NOFILE` can be set to), `kernel.pid_max`, `fs.inotify.max_user_watches`, and per-user
`/etc/security/limits.conf` (PAM — applies to login sessions only). **A service started by systemd ignores `limits.conf`**; set
`LimitNOFILE=` in the unit ([`21-systemd-and-services.md`](21-systemd-and-services.md)). Containers inherit limits from the runtime
(`docker run --ulimit nofile=65536:65536`, or the runtime's defaults). "I set `ulimit -n` and it didn't help" nearly always means
the limit is set in the wrong place, which `/proc/<pid>/limits` reveals.

### Handling EMFILE in the accept loop
Running out of fds in an `accept` loop is nasty: the pending connection stays in the accept queue ([`01-network/11-socket.md`](../01-network/11-socket.md)),
the listening socket remains readable, and a naive loop **spins at 100% CPU** retrying `accept()` that keeps failing. Good servers
(1) raise `RLIMIT_NOFILE` at startup to the hard limit (`setrlimit`, via the `rlimit` crate or `libc`), (2) on `EMFILE`/`ENFILE`
log, **back off briefly** (e.g. sleep a few ms) instead of spinning, and sometimes keep a spare fd to accept-and-close one
connection to shed load gracefully ([`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md)), and (3) bound the number of concurrent
connections deliberately so the limit is a policy, not an accident ([`07-security/09-ddos.md`](../07-security/09-ddos.md)). Count fds in monitoring
(`ls /proc/<pid>/fd | wc -l` against the limit, [`08-observability/02-metrics.md`](../08-observability/02-metrics.md)) and alert at ~80%. A steady upward drift is a **leak**
(often `CLOSE_WAIT` sockets, [`01-network/12-tcp.md`](../01-network/12-tcp.md)).

### /proc: the kernel's live process table as files
`/proc/<pid>/` exposes everything the kernel knows about a process as readable files ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)):

| File | Shows |
|---|---|
| `status` | name, state, PID/PPID, UIDs, `Threads`, `VmRSS`, `VmSize`, signal masks, capabilities |
| `cmdline`, `environ`, `exe`, `cwd` | arguments, environment, binary, working directory |
| `fd/`, `fdinfo/` | open fds as symlinks to what they point at (`socket:[12345]`, `/path`), with offsets |
| `limits` | the rlimits actually in force |
| `maps`, `smaps`, `smaps_rollup` | memory map; per-region RSS/PSS/dirty ([`16-memory.md`](16-memory.md)) |
| `stat`, `sched`, `schedstat` | CPU times, scheduling stats (run-queue wait, [`12-cpu-scheduling.md`](12-cpu-scheduling.md)) |
| `io` | bytes read/written, syscall counts |
| `task/<tid>/` | one directory per thread |
| `net/tcp`, `net/sockstat` | the socket table for the process's network namespace (what `ss` reads) |
| `cgroup`, `ns/` | which cgroup and namespaces ([`13-containers.md`](13-containers.md)) |

System-wide: `/proc/meminfo`, `/proc/loadavg`, `/proc/stat`, `/proc/interrupts`, `/proc/softirqs`, `/proc/net/snmp` (TCP/IP counters —
retransmits, resets — `nstat` reads it), `/proc/sys/` (the sysctls, [`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md)), and `/proc/pressure/*`
(PSI). `/sys` mirrors device and cgroup state ([`13-containers.md`](13-containers.md)).

### Matching a socket fd to a connection
`ls -l /proc/<pid>/fd` shows `socket:[inode]` entries; `ss -tnp` shows which socket (and process) belongs to each connection; the
inode links the two (`ss -e` prints inodes). So when `lsof -p <pid>` shows 40,000 sockets you can group them by state and peer
(`ss -tn state close-wait`) and decide whether it's a leak or load.

### The observability toolbox, by question
- *What syscalls is it making, and failing?* `strace -f -p <pid>` (`-c` summary, `-e trace=network`); see `EMFILE`, `EAGAIN`, `ECONNRESET`
  directly ([`12-testing/05-debugging.md`](../12-testing/05-debugging.md)). Expensive on a hot process — it pauses it at every syscall; prefer `perf trace` or
  eBPF ([`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md)) in production.
- *Where is CPU going?* `perf top`, `perf record -g` + flamegraph ([`08-observability/04-profiling.md`](../08-observability/04-profiling.md)); `pidstat -t 1` per-thread.
- *Who has this file/port open?* `lsof -i :8080`, `ss -tlnp`, `fuser`.
- *Is memory growing?* `/proc/<pid>/status` `VmRSS`, `smaps_rollup`, heap profilers ([`16-memory.md`](16-memory.md)).
- *Disk or network saturation?* `iostat -x 1`, `sar -n DEV 1`, `ip -s link`.
- *What did the kernel complain about?* `dmesg -T`, `journalctl -k` — OOM kills, conntrack full, segfaults, hardware errors.

### Gotcha: limits are inherited, and changed limits aren't retroactive
A child inherits its parent's rlimits at `fork`, so a proxy started from your shell has your shell's limit, and one started
by a supervisor has the supervisor's. Raising a limit in the unit file affects only **new** processes — restart the service and
confirm in `/proc/<pid>/limits`. Also the unit's `LimitNOFILE=infinity` may map to a huge number and break some programs that
size arrays from it; set a concrete value (e.g. 1048576) and check.

## Practice

Build these in order.

1. Run `ulimit -n`, `ulimit -Hn`, `cat /proc/self/limits`, and `cat /proc/sys/fs/file-nr`. **Done when** you can explain soft vs hard
   and what each number means.
2. Cause EMFILE: `ulimit -n 64`, run [`labs/00-tcp-server`](../../labs/00-tcp-server), and open more than 64 connections (a loop of `nc`, or `wrk`). **Done when** `strace -f -e
   trace=accept4` shows `EMFILE`, you have seen with `top` whether the server spins or backs off, and you have made it back off instead of spinning.
3. Raise the limit inside the process at startup with `setrlimit` (the `rlimit` crate). **Done when** `cat /proc/<pid>/limits` shows the
   raised value.
4. Start a service through a systemd unit with and without `LimitNOFILE=65536` ([`21-systemd-and-services.md`](21-systemd-and-services.md)). **Done when** `/proc/<pid>/limits`
   shows both values and you can say why `ulimit -n` in your shell was irrelevant.
5. For your running [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), script a `proxy-vitals.sh` that prints one line per second: thread count and `VmRSS` from
   `/proc/<pid>/status`, fd count from `/proc/<pid>/fd`, key `smaps_rollup` fields, and `io` counters. **Done when** the numbers move sensibly under a
   load test and match `top`/`ss`.
6. Leak fds on purpose in a scratch handler (forget to drop sockets). **Done when** you have watched `ls /proc/<pid>/fd | wc -l` climb under
   load and identified, with `ss -tn state close-wait` and `ls -l /proc/<pid>/fd`, what leaked.
7. Enable core dumps (`ulimit -c unlimited`; check `cat /proc/sys/kernel/core_pattern`), crash a scratch program with `kill -SEGV`, and open the core with
   `rust-gdb`. **Done when** `bt` prints a backtrace and you can state what sensitive data a core of your proxy would contain.
