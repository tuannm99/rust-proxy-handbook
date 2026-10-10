# Process Lifecycle: fork, exec, wait, Zombies, and PID 1

How a process is born, replaced, supervised and reaped — and why a proxy's
behavior as a child of a shell, systemd or a container runtime depends on it.
Builds on [`03-processes-and-threads.md`](03-processes-and-threads.md).

## What to learn

### fork: clone the process
`fork()` creates a **child** that is a near-exact copy of the caller: same
code, same memory contents (copy-on-write, so the copy is lazy and cheap —
[`16-memory.md`](16-memory.md)), same **open file descriptors** (both processes now refer to
the *same* kernel objects; [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)). It returns **twice**: the
child's PID in the parent, and `0` in the child. From then on they diverge. The
shared-fd detail is important: a listening socket inherited across `fork` is the
oldest way to run several workers on one port, and the reason a "closed" socket may
stay open if a forgotten child still holds it.

```rust
// the raw shape, via libc (scratch code, not part of the workspace)
let pid = unsafe { libc::fork() };
match pid {
    -1 => /* error */ {},
    0  => /* child: pid 0 here */ {},
    _  => /* parent: pid is the child's id */ {},
}
```
Rust code in a multi-threaded process (any tokio program) must not `fork` and keep
running Rust code in the child: only the forking thread survives there, and locks held
by other threads stay locked forever. Hence the next call.

### exec: replace the program
`execve(path, argv, envp)` **replaces** the current process image with a new program:
same PID, same open fds (unless marked **close-on-exec**), brand-new memory. `fork` +
`exec` is how every shell launches a command and how `std::process::Command` spawns a
child (Rust uses `posix_spawn`/`fork+exec` internally, safely). The arguments (`argv`)
and **environment** (`envp`) are inherited or set at this point; the environment is
how config like `RUST_LOG` reaches a process.

**Gotcha: leaked fds across exec.** An fd inherited by an exec'd child that you didn't
mean to share (a secret file, a listening socket) is a classic bug. Rust's std opens
files and sockets with `O_CLOEXEC` by default; raw `libc::socket` calls need the flag
(`SOCK_CLOEXEC`) explicitly — except when you *want* inheritance, as in a hot restart
that hands the listening socket to the new binary ([`09-architecture/05-rolling-restart.md`](../09-architecture/05-rolling-restart.md)).

### exit and wait: the zombie
A process ends via `exit(code)`, returning from `main`, or a fatal signal
([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)). The kernel frees its memory and closes its
fds — but keeps a tiny **process-table entry** holding the exit status until the
parent collects it with `wait()`/`waitpid()`. A finished-but-not-yet-waited process is a
**zombie** (`Z` in `ps`): it uses no CPU or memory, only a PID. A parent that never waits
leaks PIDs until `fork` fails with `EAGAIN`. The exit status encodes either the exit
code (0 = success by convention) or "killed by signal N" — `128+N` in a shell, so exit
code 137 means `SIGKILL` (often the OOM killer, [`16-memory.md`](16-memory.md)) and 143 means `SIGTERM`.

```rust
let mut child = std::process::Command::new("sleep").arg("1").spawn()?;
let status = child.wait()?;           // reaps the zombie; status.code() / .signal()
```

### Orphans and PID 1
If the parent dies first, its children become **orphans** and are **reparented** to
PID 1 (or the nearest "subreaper"), which must `wait()` for them. On a normal host PID 1
is `init`/`systemd`, which does. In a container, **PID 1 is your process** (the first
process in the PID namespace, [`13-containers.md`](13-containers.md)) — and the kernel treats it
specially: it **ignores signals it has no handler for** (so `SIGTERM` does nothing
unless you handle it — [`17-signals.md`](17-signals.md)), and it is responsible for reaping
orphaned children. A proxy that is PID 1 and spawns helpers can accumulate zombies, and
a proxy that doesn't install a `SIGTERM` handler can't be stopped gracefully. The
standard fixes: install the handler (needed anyway for graceful shutdown,
[`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md)), or run a tiny init (`tini`, `docker run
--init`) as PID 1.

### Process groups, sessions, and the controlling terminal
Processes belong to a **process group**, and groups to a **session**; a session may have
a **controlling terminal**. A shell makes each pipeline its own group so Ctrl-C (`SIGINT`)
goes to the **foreground group** only. A **daemon** historically detached from the
terminal (`fork`, `setsid`) so closing the terminal (`SIGHUP`) wouldn't kill it; today
a supervisor (systemd, a container runtime) does that job and the daemon just stays in the
foreground logging to stdout/stderr ([`21-systemd-and-services.md`](21-systemd-and-services.md)). `kill -TERM -<pgid>` signals a
whole group — how a supervisor stops a process and the children it spawned.

### Threads are processes that share things
On Linux a thread is created by `clone()` with flags saying *what to share* (memory, fds,
signal handlers). `fork` is `clone` sharing nothing; `pthread_create` is `clone` sharing
nearly everything. Each thread has its own **TID**; the thread-group leader's TID is the
process PID. This is why `ps -eLf` shows threads as rows, why `/proc/<pid>/task/<tid>`
exists, and why a signal sent to a process is delivered to *one arbitrary thread* that
doesn't block it ([`17-signals.md`](17-signals.md)).

### Gotcha: the process you think you started isn't the one running
`sh -c "./proxy"` may leave a shell between the supervisor and your binary, so signals
sent to the PID hit the shell, not the proxy; `exec ./proxy` in the script replaces the
shell so the proxy *is* the PID. Container `CMD` in shell form (`CMD ./proxy`) wraps in
`sh -c`; exec form (`CMD ["./proxy"]`) doesn't. This one detail is behind many "my
container ignores SIGTERM and takes 10 seconds to stop" reports.

## Practice

1. In a shell, run `sleep 100 &`, then `ps -o pid,ppid,pgid,sid,stat,cmd` for it and the
   shell; identify the parent PID, process group and session, and read the `S`
   (sleeping) state. `kill %1` and note the exit status shown by `wait`.
2. Create a zombie on purpose: a scratch Rust program that `Command::spawn`s `true`
   and then `sleep`s 60 s without calling `wait()`; show the `Z` entry with
   `ps -o pid,ppid,stat,cmd --ppid <parent>` and confirm it disappears once the parent
   calls `wait()` or exits.
3. Orphan a child: spawn `sleep 300` from a shell that then exits (`sh -c 'sleep 300 &'`),
   and use `ps -o pid,ppid,cmd -C sleep` to watch its `PPID` change to 1 (or a subreaper).
4. Make a shell script that `exec`s a binary versus one that doesn't, run each under
   `ps -ef --forest`, and show the extra `sh` in the tree; then send `SIGTERM` to the
   recorded PID in both cases and describe the difference.
5. Run your proxy ([`labs/05-reverse-proxy`](../../labs/05-reverse-proxy)) as PID 1 in a container
   (`docker run --rm` of a minimal image, or `unshare --pid --fork --mount-proc`), try
   `docker stop`/`kill -TERM 1` with and without a `tokio::signal` handler installed, and
   record how long shutdown takes and the exit code (`echo $?`: 143 vs 137).
6. Inspect `/proc/<pid>/status` (`PPid`, `Threads`, `State`), `/proc/<pid>/fd`, and
   `ls /proc/<pid>/task` for a running multi-threaded tokio program; match the thread
   count to the runtime's worker threads plus its blocking pool.
