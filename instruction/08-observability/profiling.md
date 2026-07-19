# Profiling
perf, flamegraph, bpftrace.

## What to learn
### perf + flamegraph for CPU time
`perf record -F 99 -p <pid> -g -- sleep 30` samples the call stack 99x/sec; `cargo flamegraph` (wraps `perf` + inferno) turns that into a visual flamegraph where wide frames = more CPU time. For a proxy, expect to see time split across TLS handshake/crypto, HTTP parsing, and syscalls (read/write/epoll_wait) — if the flamegraph is dominated by allocator frames (`malloc`/`free`), that's usually a sign of unnecessary cloning of headers/bodies per request.
Build with `debug = true` under `[profile.release]` so symbols resolve, or the flamegraph is useless.

### bpftrace for syscall/latency-level questions perf can't answer
`perf` tells you where CPU time goes; `bpftrace` (built on eBPF) answers "how long did each `read()` syscall block" or "how many TCP retransmits happened" without modifying the binary. Example: histogram of `accept()` latency to catch a listener backlog problem invisible in application-level metrics.
```
bpftrace -e 'tracepoint:syscalls:sys_enter_read /pid == $1/ { @start[tid] = nsecs; }
             tracepoint:syscalls:sys_exit_read /@start[tid]/ { @read_ns = hist(nsecs - @start[tid]); delete(@start[tid]); }'
```

### Where a Rust proxy's time actually goes
1. Syscalls (epoll_wait/read/write) — see `02-linux/epoll.md`, `02-linux/zerocopy.md` for how to reduce these.
2. Allocation — every `Vec<u8>`/`String` clone on the hot path costs; profile with `heaptrack` or `dhat` (via the `dhat` crate) alongside CPU profiling.
3. TLS — handshake CPU cost is real at high connection-churn (short-lived connections re-handshake constantly); session resumption (`01-network/tls.md`) matters more than micro-optimizing the parser.
Gotcha: profiling a debug build is close to meaningless — always profile `--release`, and profile under realistic concurrent load (see `12-testing/load-testing.md`), not a single curl request.

### Reading a flamegraph without fooling yourself
Wide ≠ slow-per-call, wide = *total* time across all samples — a fast function called constantly can look identical to a slow function called rarely. Cross-reference with `perf stat` counters (instructions, cache-misses, context-switches) before concluding "this function is the bottleneck."

## Practice
1. Build `proxy` in release mode with debug symbols and run `cargo flamegraph` while driving it with the load generator from `12-testing/load-testing.md`.
2. Identify the top 3 widest frames; for each, decide: syscall (expected), allocation (fixable), or business logic (expected).
3. Use `bpftrace` to histogram `accept()`-to-`read()` latency under load and compare to the `proxy_request_duration_seconds` histogram from `08-observability/metrics.md` — do they agree?
4. Intentionally introduce a per-request `header.clone()` into a hot path, re-profile, and confirm you can see it in the flamegraph.
5. Re-run with TLS enabled (`01-network/tls.md`) vs plaintext and compare where CPU time shifts.
