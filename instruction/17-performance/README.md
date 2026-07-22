# Performance

Hardware-level performance tuning — how the CPU and memory subsystem
actually behave, distinct from the syscall-level zero-copy techniques in
`02-linux/zerocopy.md` (which this folder cross-references rather than
duplicates).

## Status: index only — not written, not blocking

Nothing here is written yet and no `labs/` crate links to it. Two files
reference it in passing — `14-memory/fragmentation.md` (struct size classes)
and `14-memory/slab-allocator.md` (cache locality) — and both stand on their
own without it.

When to come back: **after** profiling (`08-observability/profiling.md`)
shows a hot path worth optimizing. Reading about cache lines and false
sharing before you have a flamegraph pointing at them produces
micro-optimizations of code that was never the bottleneck. The natural
trigger is `proxy` under `12-testing/load-testing.md` load, not any
particular lab.

## Planned topics

- `cpu-cache.md` — cache lines, L1/L2/L3, why data layout affects hot-path latency
- `false-sharing.md` — two unrelated atomics on the same cache line silently serializing your "lock-free" code
- `numa.md` — non-uniform memory access, pinning threads/memory to a node on multi-socket machines
- `memory-layout.md` — struct field ordering, padding, `#[repr(C)]` vs Rust's default layout
- `branch-prediction.md` — why unpredictable branches on a hot path cost more than they look like they should
- `simd.md` — vectorized operations, where they show up in a proxy (header parsing, checksums)

Zero-copy I/O (`sendfile`/`splice`/`mmap`) is covered in
`02-linux/zerocopy.md`, not duplicated here.
