# Performance

Hardware-level performance tuning — how the CPU and memory subsystem
actually behave, distinct from the syscall-level zero-copy techniques in
`02-linux/zerocopy.md` (which this folder cross-references rather than
duplicates).

## Planned topics

- `cpu-cache.md` — cache lines, L1/L2/L3, why data layout affects hot-path latency
- `false-sharing.md` — two unrelated atomics on the same cache line silently serializing your "lock-free" code
- `numa.md` — non-uniform memory access, pinning threads/memory to a node on multi-socket machines
- `memory-layout.md` — struct field ordering, padding, `#[repr(C)]` vs Rust's default layout
- `branch-prediction.md` — why unpredictable branches on a hot path cost more than they look like they should
- `simd.md` — vectorized operations, where they show up in a proxy (header parsing, checksums)

Zero-copy I/O (`sendfile`/`splice`/`mmap`) is covered in
`02-linux/zerocopy.md`, not duplicated here.
