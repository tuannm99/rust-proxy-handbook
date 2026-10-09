# Files and the Filesystem: Inodes, VFS, Page Cache, and Durability

What a "file" really is on Linux, what a path resolves to, and what `write()` does
and doesn't promise. Needed for log files, static serving, config reload, and for
understanding why "everything is a file" lets one `epoll` watch sockets and pipes
alike ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)).

## What to learn

### Inodes: the file is not its name
A filesystem stores each file as an **inode**: a record holding the file's type, size,
owner, permissions ([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)), timestamps and the
location of its data blocks — but **not its name**. Names live in **directories**, which
are just files mapping `name -> inode number`. A name is a **hard link** to an inode;
one inode can have several names, and the inode (and data) is freed only when its **link
count** reaches zero *and* no process still has it open. Consequences you will meet:

- `rm` on a file a process still has open removes the name, but the data stays alive and
  keeps consuming disk until the last fd closes (`df` says full, `du` says it isn't —
  `lsof +L1` finds such files; classic after rotating a log without telling the process).
- A **symlink** is a different thing: a tiny file whose content is a path, resolved at
  use time (it can dangle, and it can point anywhere).
- Renaming within a filesystem is just editing directories — atomic, and no data moves.

```text
$ ls -li file hardlink symlink      # -i prints inode numbers
1048590 -rw-r--r-- 2 u u  5 file
1048590 -rw-r--r-- 2 u u  5 hardlink   <- same inode, link count 2
1048600 lrwxrwxrwx 1 u u  4 symlink -> file
```

### Path resolution and the VFS
Opening `/var/log/proxy/access.log` makes the kernel walk the path: start at the root
directory, look up `var` in it, then `log`, `proxy`, `access.log`, checking permissions on
each directory along the way (needing **execute/search** permission on directories, not
just read). The kernel caches those lookups in the **dentry cache**. The **VFS** (virtual
filesystem) is the layer making all this uniform: your `open`/`read`/`write` calls go to
the VFS, which dispatches to the concrete filesystem — ext4, xfs, tmpfs (RAM), overlayfs
(containers, [`13-containers.md`](13-containers.md)), NFS (network), `/proc` (kernel state exposed as
files). **Mounts** graft one filesystem onto a directory of another; a path crosses mount
points invisibly. This is also why a path is only a *request*: between your `stat` and your
`open`, someone can swap a symlink — the **TOCTOU** (time-of-check to time-of-use) race
behind many file-serving vulnerabilities ([`05-http-stack/06-static.md`](../05-http-stack/06-static.md),
[`07-security/04-normalization.md`](../07-security/04-normalization.md)). Defend by opening first, then checking the *open
fd* (`fstat`), never the path.

### File descriptors vs open file descriptions
`open()` returns an fd — an index in your process's table — which points to an **open file
description** in the kernel holding the **current offset** and flags, which points to the
inode. `dup()`/`fork()` copy the *fd*, not the description, so the duplicates **share one
offset**: a parent and child writing to an inherited fd interleave correctly, and two
`read`s on duplicated fds consume sequential data. Opening the same path twice creates two
independent descriptions with their own offsets. `O_APPEND` makes each `write` seek to end
atomically, which is why many processes can safely append whole lines to one log file.
`pread`/`pwrite` take an explicit offset and don't touch the shared one — what thread pools
and tokio's file helpers use.

### Buffered, cached, and durable are three different things
A `write()` to a file normally returns as soon as the data is copied into the kernel's
**page cache** ([`16-kernel/08-page-cache.md`](../16-kernel/08-page-cache.md)); the kernel writes it to disk later
(**write-back**, within seconds). So a successful `write` means "another process can read
it" but **not** "it is on disk": a power loss or kernel crash can lose it. `fsync(fd)` blocks
until that file's data (and metadata) reaches stable storage — slow (milliseconds on an SSD)
and the price of durability. There is a second buffer above this in user space (Rust's
`BufWriter`, libc's `stdio`): data there isn't even in the kernel yet, so a crash or
`process::exit` loses it unless flushed.

Reads hit the page cache first, which is why a second read of a hot file is memory-fast and
why `sendfile` can stream static assets without touching userspace
([`18-zerocopy.md`](18-zerocopy.md), [`16-memory.md`](16-memory.md)). Cold-file reads block on the disk —
unlike sockets, regular files are **always "ready"** to `epoll`, so you cannot make disk I/O
non-blocking with `epoll`. tokio runs file operations on a blocking thread pool for exactly
this reason (`tokio::fs`); io_uring is the real async-file answer ([`15-io_uring.md`](15-io_uring.md)).

### Atomic replace: write-temp, fsync, rename
To update a file readers might see mid-write (a config, a certificate, a cache file), never
overwrite in place. The pattern: write a temp file **in the same directory** (same
filesystem), `fsync` it, `rename(tmp, final)` — readers see the old file or the new one,
never a torn mix, because rename is atomic. Config reload
([`09-architecture/03-config.md`](../09-architecture/03-config.md)) and certificate rotation depend on this. A process can
then be told to reload by a signal ([`17-signals.md`](17-signals.md)) or notice by itself via **inotify** — a
kernel facility delivering "this path changed" events as readable fds (the `notify` crate).
Watch the *directory*: editors save by rename, so the watched inode vanishes.

### Special filesystems: /proc, /sys, /dev
`/proc` and `/sys` are not on disk: they are the kernel's state presented as files.
`/proc/<pid>/fd`, `/proc/<pid>/status`, `/proc/meminfo`, `/proc/net/tcp` and `/sys/class/net/`
let you inspect a live system with `cat` ([`20-limits-and-proc.md`](20-limits-and-proc.md)). `/dev` holds
device nodes (`/dev/null`, `/dev/urandom`, `/dev/tty`). `tmpfs` (`/tmp` or `/dev/shm` on many systems)
lives in RAM — fast, but counted against memory, and it vanishes on reboot.

### Limits, errors, and full disks
A filesystem can be "full" two ways: out of **blocks** (`df -h`) or out of **inodes**
(`df -i`; millions of tiny cache files will do it) — both give `ENOSPC`. A proxy writing
access logs to a full disk gets write errors on the request path; decide deliberately
whether that fails requests or drops logs ([`08-observability/01-logging.md`](../08-observability/01-logging.md)).
`EMFILE`/`ENFILE` mean per-process/system fd limits ([`20-limits-and-proc.md`](20-limits-and-proc.md)).

### Gotcha: log rotation and the moved file
`logrotate` renames `access.log` to `access.log.1` and creates a new file — but your process
still holds an fd to the *old inode* and keeps writing to `access.log.1`. Fix by reopening on
a signal (`SIGHUP`, [`17-signals.md`](17-signals.md)), or `copytruncate` (race-prone), or logging to stdout and
letting the supervisor handle files.

## Practice

1. Run `stat file` and `ls -li`, make a hard link (`ln`) and a symlink (`ln -s`), and show
   the inode numbers and link counts. Delete the original and show which survives and which
   dangles.
2. Create a 100 MB file, open it with `tail -f` (or a scratch program holding the fd),
   `rm` it, and show with `df`, `lsof +L1` and `ls -l /proc/<pid>/fd` that space is still
   used; then end the process and watch `df` drop.
3. Show an `O_APPEND` log written by two processes at once
   (`for i in $(seq 1000); do echo a >> f; done & ...`) comes out intact, and contrast
   with two processes writing at explicit offsets (`pwrite`) overwriting each other.
4. Write 1 GiB with and without `fsync` in a scratch Rust program (`File::sync_all`),
   time both, and explain the gap; check `grep -E 'Dirty|Writeback' /proc/meminfo` during the
   unsynced write.
5. Implement the write-temp-fsync-rename pattern for a config file in a scratch program,
   run a reader loop (`while true; do cat cfg; done`) alongside, and confirm it never sees a
   partial file; then break it on purpose (write in place) and catch a torn read. This is the
   mechanism under [`labs/13-hot-reload`](../../labs/13-hot-reload); trace it with
   `strace -f -e trace=openat,read,write,fsync,rename` and identify each syscall.
6. In [`labs/04-static-server`](../../labs/04-static-server), serve a symlink pointing outside the
   served root and confirm your server refuses it, then explain the TOCTOU race that opening
   by path then checking would leave.
