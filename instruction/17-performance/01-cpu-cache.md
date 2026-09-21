# CPU Cache

Why data layout, not instruction count, usually decides hot-path latency.
Read this **after** `08-observability/04-profiling.md` has pointed a
flamegraph at a specific hot path — not before.

## What to learn

### The numbers that make the case
A load from L1 is ~1 ns; from main memory it is ~100 ns. The CPU does not
fetch bytes, it fetches 64-byte cache lines. So the question that decides
hot-path speed is rarely "how many instructions" but "how many cache
misses" — one miss costs as much as ~100 arithmetic operations. Code that
looks efficient (few instructions) can be slow because it chases pointers
that each miss cache, and code that looks wasteful (touching more bytes) can
be fast because it streams sequentially and every line is prefetched.

### Locality: the two kinds
- **Spatial** — data used together should sit together, so one line fetch
  brings several useful values. Iterating a `Vec<Struct>` has it; chasing a
  linked list of `Box`ed nodes does not, which is a core reason
  `13-algorithms/lru.md` and `15-parser/03-ast.md` push arenas (`Vec` +
  indices) over `Box`ed pointer structures.
- **Temporal** — data used now will be used again soon, so keep it hot.

```rust
// Struct-of-Arrays: iterating just `weight` touches only weight's lines
struct Backends { weights: Vec<u32>, conns: Vec<u32>, addrs: Vec<SocketAddr> }
// vs Array-of-Structs: iterating `weight` drags addr+conns through cache too
struct Backend { weight: u32, conns: u32, addr: SocketAddr }
```

For a load balancer scanning weights across thousands of backends
(`06-proxy/02-load-balancer.md`), the struct-of-arrays layout can be several
times faster purely from not fetching the fields it does not read.

### Prefetching rewards predictable access
The hardware prefetcher watches your access pattern and pulls the next
lines *before* you ask, but only when the pattern is predictable —
sequential or fixed-stride. Sequential array traversal runs near memory
bandwidth; random pointer chasing defeats the prefetcher entirely and pays
the full miss on every hop. This is the concrete, measurable reason "flat
array beats linked structure" keeps coming up.

### Pointer chasing is the proxy's recurring tax
A request often walks: connection → session map entry → route → upstream →
backend. If each hop is a separate heap object, each is a likely cache
miss, and the chain runs on every single request. You cannot remove the
logical indirection, but you can make the *hot* fields (the ones touched
every request) contiguous and leave cold fields elsewhere — a "hot/cold
split" of the struct.

### Measure, because intuition is wrong here
Cache behavior is invisible in source. Use `perf stat` to read
`cache-misses` and `L1-dcache-load-misses`, and `perf record` /
`cachegrind` to attribute them to lines of code. The whole discipline is
counterintuitive enough that changing layout without measuring both before
and after is guessing — and the guess is often backwards.

Gotcha: never do this work speculatively. A layout change that shaves cache
misses off code that runs 0.1% of the time is invisible in production and
adds complexity forever. The trigger is a flamegraph
(`08-observability/04-profiling.md`) showing a specific hot loop, under real
load (`12-testing/01-load-testing.md`) — see also `17-performance/02-false-sharing.md`
for the concurrent version of this problem.

## Practice
1. Benchmark array-of-structs vs struct-of-arrays for scanning one field
   across 100k backends (the `06-proxy/02-load-balancer.md` weight scan);
   record `perf stat cache-misses` for both, not just wall time.
2. Build a linked list of `Box`ed nodes and an arena (`Vec` + index) of the
   same data, traverse both, and compare cache-miss counts to see the
   prefetcher effect.
3. Take a hot struct from your `proxy` request path and split it hot/cold;
   measure whether the request-path benchmark moves at all — and be
   honest if it does not.
4. Use `perf record` to attribute cache misses to specific lines in a
   `proxy` hot path under load, rather than guessing which access is
   costly.
5. Read `L1-dcache-load-misses` before and after one layout change and
   write down whether the change was worth its complexity — practice
   rejecting changes that do not move the number.
