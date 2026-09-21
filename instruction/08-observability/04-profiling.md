# Profiling
perf, flamegraph, bpftrace.

## What to learn
### perf + flamegraph for CPU time
`perf record -F 99 -p <pid> -g -- sleep 30` samples the call stack 99x/sec; `cargo flamegraph` (wraps `perf` + inferno) turns that into a visual flamegraph where wide frames = more CPU time. For a proxy, expect to see time split across TLS handshake/crypto, HTTP parsing, and syscalls (read/write/epoll_wait) — if the flamegraph is dominated by allocator frames (`malloc`/`free`), that's usually a sign of unnecessary cloning of headers/bodies per request.
Build with `debug = true` under `[profile.release]` so symbols resolve, or the flamegraph is useless.

Gotcha: symbols alone aren't enough — `perf -g` also needs to *unwind* the
stack, and release Rust omits frame pointers by default, which produces
flamegraphs that are one frame deep and useless in a different way. Build
with `RUSTFLAGS="-C force-frame-pointers=yes"`, or use `--call-graph dwarf`
(more accurate, much larger perf.data, higher overhead).

Gotcha: `perf` needs permission. `kernel.perf_event_paranoid` commonly
defaults to a value that blocks profiling unprivileged processes, and
inside a container you generally need `CAP_PERFMON` (or `--privileged`)
plus a matching kernel. Sort this out *before* the incident where you need
a profile.

### Async makes flamegraphs lie about causality
This is the gotcha that matters most for this codebase, and it surprises
almost everyone the first time.

A flamegraph shows the call stack at sample time. In async Rust, that
stack is `worker thread → executor loop → poll() → your future's poll`.
It is **not** the logical path of a request. A future that awaits four
things sequentially appears as four unrelated `poll` stacks under the
executor, with no parent-child relationship between them — the logical
structure you care about is in the *state machine*, not on the stack.

Two consequences:
- **Attribution is wrong.** You cannot read "this upstream call caused
  that allocation" off the graph, because the upstream call's continuation
  is polled from the executor, not from the code that awaited it.
- **Waiting is invisible.** CPU profiling samples threads that are *on
  CPU*. A request spending 200ms waiting on an upstream contributes
  nothing to the flamegraph. If your proxy is slow because it's waiting,
  the flamegraph will look perfectly healthy and tell you nothing.

Use traces (`08-observability/03-tracing.md`) for causality and latency
attribution; use flamegraphs for "what is burning CPU." They answer
different questions and mixing them up wastes a lot of time.

### Off-CPU time and tokio-console
Since CPU profiles hide waiting, you need a second tool for the other
half. Off-CPU profiling (via `bpftrace` on scheduler tracepoints, or
`offcputime` from bcc) measures where threads *block*, which is where a
proxy's latency usually lives.

For async specifically, `tokio-console` is the targeted instrument: it
shows per-task poll counts, poll durations, and — the key signal — tasks
whose individual polls take a long time. A poll that runs for
milliseconds is a task blocking the executor thread
(`03-rust/05-async.md`'s cooperative scheduling), which stalls every other
connection on that worker. Common culprits in a proxy: a synchronous file
read (`05-http-stack/05-static.md`), a large compression
(`05-http-stack/06-compression.md`), regex over a large body
(`07-security/06-waf.md`), or a synchronous log write
(`08-observability/01-logging.md`).

Gotcha: a blocked executor thread shows up as *latency on unrelated
requests*, which is why it's so hard to diagnose from request-level data
alone — the slow request and the affected requests are different requests.

### bpftrace for syscall/latency-level questions perf can't answer
`perf` tells you where CPU time goes; `bpftrace` (built on eBPF) answers "how long did each `read()` syscall block" or "how many TCP retransmits happened" without modifying the binary. Example: histogram of `accept()` latency to catch a listener backlog problem invisible in application-level metrics.
```
bpftrace -e 'tracepoint:syscalls:sys_enter_read /pid == $1/ { @start[tid] = nsecs; }
             tracepoint:syscalls:sys_exit_read /@start[tid]/ { @read_ns = hist(nsecs - @start[tid]); delete(@start[tid]); }'
```

### Where a Rust proxy's time actually goes
1. Syscalls (epoll_wait/read/write) — see `02-linux/01-epoll.md`, `02-linux/05-zerocopy.md` for how to reduce these.
2. Allocation — every `Vec<u8>`/`String` clone on the hot path costs; profile with `heaptrack` or `dhat` (via the `dhat` crate) alongside CPU profiling.
3. TLS — handshake CPU cost is real at high connection-churn (short-lived connections re-handshake constantly); session resumption (`01-network/07-tls.md`) matters more than micro-optimizing the parser.
Gotcha: profiling a debug build is close to meaningless — always profile `--release`, and profile under realistic concurrent load (see `12-testing/01-load-testing.md`), not a single curl request.

Gotcha: allocation shows up in a CPU profile as `malloc`/`free` frames,
but the *cost* of a bad allocation pattern is often fragmentation and RSS
growth over days (`14-memory/06-fragmentation.md`), which no 30-second
profile can see. Track allocated-vs-resident as a gauge in parallel.

### Continuous profiling beats profiling during an incident
A profile captured in a load test reflects your load generator's traffic
mix, not production's — and by the time you're SSHing into a box to run
`perf`, the interesting window may have passed. Continuous profilers
(`pprof-rs` exposing a `/debug/pprof` endpoint, scraped by Parca or
Pyroscope) sample at low frequency all the time, so you can look at a
profile from *last Tuesday at 03:14* when the incident actually happened.

Gotcha: expose that endpoint on the internal listener, not the public one
(same reasoning as `/metrics` in `08-observability/02-metrics.md`) — a
profiling endpoint is both an information leak and a CPU cost anyone can
trigger.

### Reading a flamegraph without fooling yourself
Wide ≠ slow-per-call, wide = *total* time across all samples — a fast function called constantly can look identical to a slow function called rarely. Cross-reference with `perf stat` counters (instructions, cache-misses, context-switches) before concluding "this function is the bottleneck."

Gotcha: also check whether you're CPU-bound at all before optimizing CPU.
If the proxy is at 20% CPU and latency is bad, the flamegraph's widest
frame is irrelevant — the answer is in off-CPU time, lock contention, or
an upstream. `perf stat`'s IPC and context-switch counts, plus overall
utilization, tell you which regime you're in.

## Practice
Build these in order.

1. Configure the build for profiling (`debug = true` in release,
   `force-frame-pointers`) and confirm permissions. **Done when**
   `cargo flamegraph` against `proxy` under load produces a graph with
   deep, symbolized Rust frames — not a flat one.
2. Profile under realistic concurrent load from
   `12-testing/01-load-testing.md`. **Done when** you can name the top three
   widest frames and classify each as syscall (expected), allocation
   (fixable), or logic (expected).
3. Introduce a deliberate per-request `header.clone()` on the hot path and
   re-profile. **Done when** you can see it appear — this calibrates how
   visible a regression of that size actually is.
4. Demonstrate the async attribution problem. **Done when** you can show
   that a slow *upstream* (inject 200ms of delay) does not widen anything
   in the flamegraph, and explain from the profile alone why it doesn't.
5. Run `tokio-console` against `proxy`. **Done when** you can identify the
   task with the longest individual poll duration — then add a synchronous
   10ms operation inside a handler and watch it become the worst offender.
6. Measure the damage a blocking poll does. **Done when** you can show
   p99 latency rising for *other* concurrent requests while one handler
   blocks, and dropping again when you move the work to `spawn_blocking`.
7. Use `bpftrace` to histogram `accept()`-to-first-`read()` latency under
   load. **Done when** you can compare it against
   `proxy_request_duration_seconds` and say whether they agree — a gap
   means time is being spent before your instrumentation starts.
8. Compare TLS vs plaintext profiles. **Done when** you can quantify
   handshake CPU cost per connection and show it falling once session
   resumption is enabled (`01-network/07-tls.md`).
9. (Stretch) Expose `pprof-rs` on the internal listener and capture
   profiles continuously during a chaos test (`12-testing/03-chaos.md`).
   **Done when** you can retrieve the profile from the exact minute a
   fault was injected, after the fact.
