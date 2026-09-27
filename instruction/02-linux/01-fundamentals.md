# Linux/OS Fundamentals

## Status: the one exception to "not a tutorial"

Same exception as `01-network/01-fundamentals.md`, for the same reason:
`07-epoll.md`, `09-memory.md`, `10-signals.md`, and `11-zerocopy.md` all
open by assuming you already know what a syscall, a file descriptor, and
a process's address space are. This file and its five siblings exist to
make sure that's true before you hit them. Skip the whole group if it
already is.

## What to learn

### What the kernel is for
The **kernel** is the one program on the machine allowed to talk to
hardware directly, manage memory across all processes, and enforce
isolation between them. Everything else — your proxy, your shell, every
other process — runs in **user space**, with no direct access to
hardware and no ability to touch another process's memory. Almost every
theme in `02-linux/` is some consequence of that one split: what crossing
it costs (`03-kernel-and-syscalls.md`), what it isolates
(`02-processes-and-threads.md`, `06-containers.md`), and what happens
when the kernel needs to interrupt you rather than wait to be asked
(`05-blocking-io-and-signals.md`).

### The five pieces, and where each lives
- **`02-processes-and-threads.md`** — what a process and a thread
  actually are, why threads are cheaper, and why sharing memory between
  them is the reason `03-rust/04-sync.md` exists.
- **`03-kernel-and-syscalls.md`** — the user space/kernel space boundary,
  what a syscall costs, and file descriptors — the integer handle
  everything in `02-linux/07-epoll.md` is built around.
- **`04-memory-basics.md`** — the minimum needed to make
  `02-linux/09-memory.md`'s opening paragraph land as familiar rather
  than new, plus where RAM sits relative to cache and disk.
- **`05-blocking-io-and-signals.md`** — why a syscall can block a thread,
  why event loops exist as the alternative, and what a signal is (an
  interruption from outside your normal control flow, not a return
  value).
- **`06-containers.md`** — what a container actually is (isolated
  processes on one shared kernel, not a tiny VM), since
  Kubernetes/pods/cgroups are referenced constantly from
  `09-architecture/` onward with nothing ever defining them.

Read them in that order once; after that, treat each as a standalone
lookup.

## Practice
1. Run `uname -a` and read your kernel version; run `ps aux` and pick
   three processes — for each, guess (then verify with `man`/docs)
   roughly what it does.
2. Read the five sibling files in order, then come back here and explain,
   in one sentence each: why a thread is cheaper than a process, what a
   syscall actually crosses, why `read()` can block, and what a container
   isolates that a plain process doesn't.
3. Run `cat /proc/version` and `cat /proc/cpuinfo | grep -c processor` —
   confirm you can find your kernel version and core count without a GUI
   tool.
