# Debugging Toolkit

The tools for answering "what is my program actually doing?" without
guessing. Most self-study time lost on a proxy isn't spent writing code —
it's spent staring at code that looks correct while it misbehaves. Every
tool here replaces staring with evidence. Referenced from
[`00-introduction/03-study-loop.md`](../00-introduction/03-study-loop.md)'s "when you're stuck" protocol.

## What to learn

### Start from the method, not the tool
Debugging is a loop: form a hypothesis about where the bug is, pick the
cheapest tool that would confirm or kill it, run it, repeat. The failure
mode is the reverse — changing code at random until the symptom moves. Two
habits make the loop fast: **reproduce first** (a bug you can't trigger on
demand can't be fixed with confidence), and **shrink the reproduction**
until it's small enough that the cause has nowhere to hide.

### Your own program's view: panics, logs, `tracing`
`RUST_BACKTRACE=1` turns a panic's one-line message into a stack trace;
it's the first thing to set when anything crashes. For everything else,
structured logs beat `println!`: `tracing` with a `RUST_LOG`-style filter
lets you turn on detail for one module without drowning in the rest
([`08-observability/01-logging.md`](../08-observability/01-logging.md)).

```rust
tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();
// RUST_LOG=reverse_proxy=debug,hyper=info cargo run -p reverse-proxy
```
Gotcha: a log line that says "sent response" proves your code *called*
write, not that bytes reached the client. For anything involving the
network, confirm with a tool that watches the wire.

### The wire's view: `curl -v`, `nc`, `ss`, `tcpdump`
- `curl -v` (add `--http2` or `--http1.1` to force a version) shows the exact request and response headers — the first check for any HTTP bug.
- `nc` (netcat) sends hand-typed or malformed bytes, which `curl` won't. Essential for testing a parser against broken input.
- `ss -tanp` lists every socket with its state (`ESTAB`, `TIME_WAIT`, `CLOSE_WAIT`...) and owning process. A pile of `CLOSE_WAIT` means *your* code isn't closing connections the peer already closed.
- `tcpdump -i lo -A port 8080` (or Wireshark on the capture) shows the actual packets: whether the bytes were sent, in what order, and who closed first.

```text
curl -v --http1.1 http://127.0.0.1:8080/        # what did the server say?
printf 'GET / HTTP/1.1\r\n\r\n' | nc 127.0.0.1 8080   # a request curl would refuse to send
ss -tanp | grep 8080                              # who holds which sockets, in what state?
```

### The kernel's view: `strace`
`strace -f -e trace=network,read,write -p <pid>` shows every syscall the
process makes. It answers questions no log can: is the process blocked in
`epoll_wait` (idle, waiting) or spinning on `accept` returning `EMFILE`
([`07-security/09-ddos.md`](../07-security/09-ddos.md))? Did `write` return fewer bytes than asked (a short
write, [`01-network/12-tcp.md`](../01-network/12-tcp.md))? `strace -c` gives a syscall count summary,
the quickest way to see what dominates.

### The runtime's view: `tokio-console`
For async-specific bugs — a task that never wakes, a worker stalled by
blocking code — `tokio-console` shows every task, how long it's been
idle, and how long each poll took ([`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md)).
A task with a huge poll time is blocking a worker thread; a task idle
forever with no waker activity is a lost wakeup ([`04-runtime/02-waker.md`](../04-runtime/02-waker.md)).

### Stepping through code: `rust-gdb` / `rust-lldb`
A debugger shines for logic bugs in synchronous code — a parser state
machine taking the wrong branch. Build without optimizations (the default
`dev` profile), run under `rust-gdb target/debug/<bin>`, set a breakpoint
on a function, inspect variables. Gotcha: stepping through async code is
painful, because execution jumps between tasks at every `.await`; for
async bugs, `tracing` and `tokio-console` usually get you there faster.

### Performance and memory: `perf`, flamegraphs, heap profilers
When the bug is "too slow" or "memory keeps growing," measure before
changing anything. `cargo flamegraph` (wrapping `perf`) shows where CPU
time goes ([`08-observability/04-profiling.md`](../08-observability/04-profiling.md)); a heap profiler such as `dhat`
(as a crate) or `heaptrack` shows who allocated what, and whether it was
ever freed. RSS that grows under steady load is either a leak or an
unbounded buffer — the heap profile tells you which.

### Undefined behavior: Miri and sanitizers
If your code has `unsafe`, a test passing proves little.
`cargo +nightly miri test` catches out-of-bounds access, use-after-free,
and invalid aliasing that real hardware silently tolerates
([`03-rust/03-unsafe.md`](../03-rust/03-unsafe.md), [`12-testing/04-ci-tooling.md`](04-ci-tooling.md)).

## Practice
Build these in order.

1. In [`labs/00-tcp-server`](../../labs/00-tcp-server), run the server under `strace -f -e trace=network` while connecting with `nc`. **Done when** you can point to the `accept`, `read`, and `write` syscalls for one echoed line.
2. Capture one echo round trip with `tcpdump -i lo -A port <port>`. **Done when** you can identify the handshake, the data packets, and which side sent the first `FIN`.
3. Deliberately leave a connection half-handled (stop reading from one client) and find it with `ss -tanp`. **Done when** you can explain the state the socket is stuck in.
4. Add `tracing` with an environment filter to [`labs/00-tcp-server`](../../labs/00-tcp-server). **Done when** you can switch per-connection debug logging on and off with `RUST_LOG` without recompiling.
5. Introduce a `std::thread::sleep` inside a connection handler and find it with `tokio-console`. **Done when** the console shows you the offending task by its poll time.
6. Generate a flamegraph of [`labs/00-tcp-server`](../../labs/00-tcp-server) under load. **Done when** you can name the function where most CPU time goes and say whether that's expected.
