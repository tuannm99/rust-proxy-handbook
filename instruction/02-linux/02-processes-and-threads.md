# Processes and Threads

Part of the from-scratch fundamentals series — see `02-linux/01-fundamentals.md`
for the full index.

## What to learn

### A process: its own private world
A **process** is a running program with its own private address space —
its own view of memory, isolated from every other process, enforced by
the kernel's memory-management hardware (the MMU, see
`04-memory-basics.md`). Two processes cannot read or corrupt each other's
memory by accident. A process also owns its own set of open file
descriptors (`03-kernel-and-syscalls.md`), its own process ID, and its
own resource limits.

Creating a new process (`fork()` on Unix, under the hood of
`std::process::Command` in Rust) is comparatively expensive: a new
address space has to be set up (though Linux's copy-on-write makes the
*initial* `fork()` itself cheap — the real cost shows up as the child
writes to its own copies of pages).

### A thread: sharing the world, but not the stack
A **thread** is a unit of scheduling *within* a process. All threads in
one process **share** that process's address space — the same heap, the
same global variables, the same open file descriptors — but each thread
gets its own stack and its own CPU register state, so the kernel can run
threads independently and interleave or parallelize them across cores.

This is why threads are cheaper to create than processes (no new address
space to set up, just a new stack and some kernel bookkeeping) and why
sharing data between threads is easy-but-risky (same memory, so a data
race is possible if two threads touch it without coordination) while
sharing data between processes is hard-but-safe-by-default (separate
memory; you have to explicitly opt into sharing via IPC — pipes, shared
memory segments, sockets).

```rust
// two threads sharing one process's heap via Arc<Mutex<_>> — this only
// compiles/makes sense because threads share an address space; the
// equivalent across two separate processes needs real IPC instead
let counter = std::sync::Arc::new(std::sync::Mutex::new(0));
let c2 = counter.clone();
std::thread::spawn(move || { *c2.lock().unwrap() += 1; });
```

`03-rust/04-sync.md`'s entire subject — `Arc`, `Mutex`, atomics — exists
because tokio runs your async tasks on a pool of OS threads sharing one
address space, and Rust's type system is what catches the shared-memory
risk described above at compile time instead of at 3am in production.

### Tokio tasks are neither: cheaper than both
Tokio's runtime (`04-runtime/01-tokio.md`) is a small pool of real OS
threads, each capable of running many of your `async fn` tasks,
cooperatively switching between them. A tokio task is not a thread and
not a process — it's much cheaper than either (no kernel stack, no
separate scheduling entity the *kernel* knows about), and many of them
time-share a handful of real OS threads. This three-level stack —
processes contain threads, threads (in an async program) run many tasks
— is worth keeping straight: "concurrency" (many tasks making progress
in an interleaved way) and "parallelism" (multiple things running at the
literal same instant, needing multiple cores) are related but distinct;
tokio's multi-threaded runtime gives you both, a single-threaded runtime
gives you only the first.

### Two schedulers, stacked
With more runnable threads than CPU cores — normal on any real machine —
the kernel's scheduler decides which thread runs on which core for how
long, switching between them (a context switch,
`03-kernel-and-syscalls.md`'s subject). Tokio has its *own* scheduler one
level up, deciding which of *your* tasks a given OS thread works on next.
These are genuinely two different, independently-acting schedulers: the
kernel doesn't know your tokio tasks exist at all, and tokio doesn't
control which core its own worker threads land on. When you're
diagnosing an unexpected delay (`04-runtime/02-waker.md`,
`08-observability/04-profiling.md`), knowing which of the two layers you
're looking at is often the whole question.

Gotcha: a tokio task that runs a long synchronous computation blocks the
*OS thread* it's currently on — and because that thread is shared by
tokio's own scheduler across potentially many tasks, one bad task stalls
every other task queued on that thread, invisible to the kernel scheduler
entirely (the OS thread looks perfectly busy; it's just busy on the wrong
thing).

## Practice
1. Write a tiny program that spawns 3 OS threads sharing one `Vec` behind
   a `Mutex`, and a second version using plain `Rc` without
   synchronization — confirm the second doesn't compile, and read the
   compiler error to see what it's actually objecting to.
2. Run `ps -eLf` (Linux) and find a multi-threaded process on your
   machine (e.g. your browser, or a running `tokio` program) — count how
   many threads (LWPs) it has versus one for a single-threaded process
   like a shell.
3. Spawn a `labs/00-tcp-server` instance and, while it's handling several
   idle connections, check `ps -eLf | grep tcp-server` — confirm the
   number of OS threads is small and roughly matches your core count, not
   your connection count.
4. Write a small program that spends 2 seconds in a tight CPU loop inside
   one `tokio::spawn`ed task on a single-threaded runtime
   (`#[tokio::main(flavor = "current_thread")]`) while another task tries
   to `tokio::time::sleep` for 100ms concurrently — measure how late the
   sleep actually fires, and explain why using the stacked-scheduler
   model above.
