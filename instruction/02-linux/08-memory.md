# Memory

Virtual memory, page cache, NUMA.

## What to learn

### Virtual memory basics
Every process gets its own virtual address space; the kernel's page tables
map virtual pages to physical frames (or to "not present, fault me"). This is
why `fork()` is cheap (copy-on-write page tables, not memory), why a `Vec`'s
buffer can be resized without the OS actually zeroing gigabytes up front
(lazy/overcommitted pages), and why RSS and virtual size are different
numbers that both matter when you're sizing a proxy's memory limits.

### Overcommit and OOM
Linux by default allows virtual allocations that exceed physical + swap
(`vm.overcommit_memory`); the process only pays for a page when it's first
written to. Gotcha: a proxy that pre-allocates large buffer pools can *look*
fine on `malloc` and then get OOM-killed under load once those buffers are
actually touched — `cgroup` memory limits (common under Kubernetes) enforce
against RSS, not virtual size, so watch actual resident memory, not what
`Vec::with_capacity` "reserved."

### Page cache and static files
The kernel keeps recently-read file data in RAM as the page cache, backing
both `read()` and `mmap()`. This is why `sendfile()` (see
`02-linux/10-zerocopy.md`) is fast for repeatedly-served static assets — the
data is often already resident, and the kernel copies page-cache-to-socket
without round-tripping through your process's userspace buffers at all. This
directly informs how `05-http-stack/05-static.md` should serve files: let the
kernel's cache do the caching rather than re-implementing an LRU in
userspace for cold data that's already hot in the page cache.

### NUMA
On multi-socket machines, memory is partitioned per-socket ("node"), and
accessing memory on a *remote* node's controller costs meaningfully more
latency than local-node memory. Thread-per-core / shard-per-core proxy
architectures (relevant if you ever move beyond tokio's work-stealing
scheduler to something like `glommio`) want each core allocating and freeing
memory pinned to its own NUMA node — `numactl --hardware` shows your
topology, and `numactl --cpunodebind=N --membind=N` is the blunt way to pin a
process while experimenting.

### Why allocator choice matters under load
The default glibc allocator has per-thread arenas that can fragment badly
under high-churn, multi-threaded allocation patterns (e.g. a request/response
buffer allocated-and-freed on every connection). Proxies commonly switch to
`jemalloc` or `mimalloc` (`tikv-jemallocator`/`mimalloc` crates in Rust) for
more predictable tail latency and lower fragmentation. This is a real,
measurable difference for `proxy` under sustained
throughput, not a micro-optimization.

## Practice
1. Run `/proc/self/status` (`VmRSS` vs `VmSize`) in a small Rust program before/after a large `Vec::with_capacity` allocation, before/after actually writing to it.
2. Reproduce the overcommit gotcha: allocate more virtual memory than physical RAM, confirm it "succeeds," then write to it and watch RSS climb (do this in a container/VM with a memory limit, not your main machine).
3. Use `/proc/self/smaps` or `pmap` to inspect a running proxy's memory map and identify the page-cache-backed vs anonymous regions.
4. Swap `proxy`'s allocator to `mimalloc` via `#[global_allocator]` and benchmark allocation-heavy request handling before/after.
5. If you have access to a multi-socket machine, run `numactl --hardware` and explain what a "remote" memory access would cost relative to local.
