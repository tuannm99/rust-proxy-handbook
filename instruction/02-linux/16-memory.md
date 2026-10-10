# Memory

Virtual memory, page cache, NUMA.

## What to learn

### Virtual memory basics
Every process gets its own virtual address space; the kernel's page tables
map virtual pages to physical frames (or to "not present, fault me"). This is
why `fork()` is cheap (copy-on-write page tables, not memory), why a `Vec`'s
buffer can be resized without the OS actually zeroing gigabytes up front
(lazy/overcommitted pages), and why RSS and virtual size are different
numbers that both matter when you're sizing a proxy's memory limits. For
the theory of what the kernel evicts when physical frames run out —
FIFO, LRU, clock, and why thrashing happens — see
[`22-theory/03-page-replacement.md`](../22-theory/03-page-replacement.md).

### Pages, faults, and what "allocating" really does
Memory is managed in **pages** (4 KiB). `malloc`/`Vec` ask the allocator, which asks the kernel for address space
(`brk` for the small heap, `mmap` for big chunks) — and the kernel hands back *virtual* addresses with **no physical memory
behind them yet**. The first time you read or write each page the CPU raises a **page fault** ([`02-hardware-basics.md`](02-hardware-basics.md)); the
kernel allocates a zeroed physical page and maps it. That is a **minor fault** (~0.1–1 µs, no disk). A **major fault** needs I/O: reading a
file-backed page that isn't in the page cache, or bringing a swapped-out page back from disk (milliseconds). `minflt`/`majflt` in
`/proc/<pid>/stat` (and `perf stat -e minor-faults,major-faults`) count them. Practical effects: the *first* requests after startup are
slower because buffers, code pages and caches are faulting in ("warm-up"); a fresh `vec![0; N]` is lazily backed by the kernel's zero page
until written; and a sudden burst of new connections that each touch new buffers shows up as a fault storm in `sy` CPU time. To pay the cost
up front, pre-touch memory or use `MAP_POPULATE`/`mlock` ([`20-limits-and-proc.md`](20-limits-and-proc.md)).

### mmap: anonymous vs file-backed memory
`mmap` maps something into your address space. **Anonymous** mappings are plain memory (large heap allocations, thread stacks). **File-backed**
mappings are windows onto a file through the page cache: program binaries and shared libraries are mapped this way (the same physical pages
shared by every process running that binary), and so can a static file you serve ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md), [`18-zerocopy.md`](18-zerocopy.md)). Mappings
are **private** (writes copy-on-write into your own page, invisible to others) or **shared** (writes go to the file or to other mappers —
[`10-ipc.md`](10-ipc.md)). Gotcha: if another process truncates a file you have mapped, touching the vanished pages raises **`SIGBUS`**, not an error return —
a reason `mmap` of externally-modified files is dangerous in a server.

### Reading memory numbers honestly
Four numbers per process, all different: **VSZ** (virtual size — everything reserved, mostly meaningless), **RSS** (resident — pages actually in RAM,
*including* shared libraries counted in every process), **PSS** (proportional — shared pages divided among their sharers; the honest sum across
processes), and **USS** (unique to this process; what you'd free by killing it). `smaps_rollup` gives them all. System-wide, `free -h` is
commonly misread: **"free" is not the number that matters** — the kernel deliberately fills spare RAM with page cache, which it drops instantly when
needed. Read **"available"** (`MemAvailable` in `/proc/meminfo`), an estimate of what can be allocated without swapping. Inside a container, `memory.current`
in the cgroup includes page cache; the kernel will reclaim cache before killing anything, so the number the OOM killer acts on is closer to "working set"
(`memory.current` minus inactive file cache) — graph that, not raw `memory.current` ([`13-containers.md`](13-containers.md)).

### Swap, reclaim, and latency cliffs
When free memory runs low the kernel **reclaims**: first dropping clean page cache, then **swapping** out anonymous pages to disk. A background thread (`kswapd`)
does this ahead of need; if memory runs out faster than it can, the *allocating thread itself* is forced into **direct reclaim** — it stalls inside `malloc` or a
page fault while the kernel frees memory — which appears as unexplained multi-millisecond latency spikes on a request path that isn't doing anything slow. A
latency-sensitive proxy therefore typically runs with little or no swap (a swapped-out buffer is a request waiting on a disk) and watches **memory pressure**
(`/proc/pressure/memory`, PSI) rather than just usage. `vm.swappiness` biases cache-vs-anonymous reclaim.

### The OOM killer in practice
If reclaim fails, the kernel's **OOM killer** picks a victim by `oom_score` (roughly: the biggest memory user, adjustable via `oom_score_adj`, where -1000 exempts a
process) and sends it `SIGKILL` — no handler runs, no graceful shutdown ([`17-signals.md`](17-signals.md)). There are two arenas: **global** OOM when the whole machine
runs out, and **cgroup** OOM when a container exceeds its `memory.max` ([`13-containers.md`](13-containers.md)) — the common one under Kubernetes, shown as `OOMKilled`, exit code **137**
([`04-process-lifecycle.md`](04-process-lifecycle.md)). Evidence: `dmesg -T | grep -i 'out of memory'`, `memory.events` (`oom_kill` counter). The proxy-specific lesson is that
memory use is *per-connection buffers times connection count*: 64 KiB read + 64 KiB write buffers per connection at 100,000 connections is 12.5 GB before a single request body is
buffered. Cap buffer sizes, request-body sizes and concurrent connections deliberately ([`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md), [`07-security/10-slowloris.md`](../07-security/10-slowloris.md)), and budget
*limit = steady state + burst + allocator overhead* rather than hoping.

### Huge pages (briefly)
A TLB miss costs a page-table walk, so workloads touching large memory benefit from **huge pages** (2 MiB instead of 4 KiB — 512x fewer TLB entries). **Transparent Huge
Pages** (THP) does this automatically, at the price of background compaction that can cause latency stalls; many latency-sensitive systems set THP to `madvise` or `never`
(`/sys/kernel/mm/transparent_hugepage/enabled`) and opt in only where measured to help ([`17-performance/`](../17-performance)).

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
[`02-linux/18-zerocopy.md`](18-zerocopy.md)) is fast for repeatedly-served static assets — the
data is often already resident, and the kernel copies page-cache-to-socket
without round-tripping through your process's userspace buffers at all. This
directly informs how [`05-http-stack/06-static.md`](../05-http-stack/06-static.md) should serve files: let the
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
measurable difference for [`proxy`](../../proxy) under sustained
throughput, not a micro-optimization.

## Practice
1. Run `/proc/self/status` (`VmRSS` vs `VmSize`) in a small Rust program before/after a large `Vec::with_capacity` allocation, before/after actually writing to it.
2. Reproduce the overcommit gotcha: allocate more virtual memory than physical RAM, confirm it "succeeds," then write to it and watch RSS climb (do this in a container/VM with a memory limit, not your main machine).
3. Use `/proc/self/smaps` or `pmap` to inspect a running proxy's memory map and identify the page-cache-backed vs anonymous regions.
4. Swap [`proxy`](../../proxy)'s allocator to `mimalloc` via `#[global_allocator]` and benchmark allocation-heavy request handling before/after.
5. If you have access to a multi-socket machine, run `numactl --hardware` and explain what a "remote" memory access would cost relative to local.
6. Watch faults and the OOM killer: run a scratch program that touches 1 GiB page by page and read `minflt` from `/proc/<pid>/stat` (or `perf stat -e minor-faults`); then run it in a container with `--memory=256m` (and `--memory-swap=256m`) allocating until killed, and confirm exit code 137, `dmesg -T | grep -i oom`, and the `oom_kill` counter in the cgroup's `memory.events`.
