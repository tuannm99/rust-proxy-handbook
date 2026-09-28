# Runtime Landscape: Work-Stealing vs. Thread-per-Core

## What to learn

### Tokio's model, restated as a design choice
Tokio's multi-threaded runtime is work-stealing: any task can run on any worker thread, and idle workers steal from busy ones ([`04-runtime/01-tokio.md`](01-tokio.md)). This optimizes for load balancing across cores with minimal manual tuning — you don't think about which core a connection lands on. The cost: a task migrating between cores means its data (an `Arc<Mutex<_>>`'s cache line, a NUMA-local allocation) may now be touched from a different core than where it was allocated — real, if usually small, overhead ([`17-performance/03-numa.md`](../17-performance/03-numa.md)).

### The thread-per-core alternative: `glommio`, `monoio`
A thread-per-core ("shard-per-core") runtime pins one thread to each core, gives it its own event loop and often its own `io_uring` instance ([`02-linux/08-io_uring.md`](../02-linux/08-io_uring.md)), and never migrates a task off the core it started on. Data structures can be `Rc<RefCell<_>>` instead of `Arc<Mutex<_>>` within a shard, because nothing else ever touches that shard's memory — no atomics, no cross-core cache-line bouncing. The cost moves elsewhere: load imbalance between shards must be solved architecturally (`SO_REUSEPORT` plus the kernel's connection distribution, [`16-kernel/05-rss.md`](../16-kernel/05-rss.md)/[`16-kernel/06-rps.md`](../16-kernel/06-rps.md)) instead of by a scheduler stealing work automatically.

```rust
// Conceptual shape, not tokio: each shard owns its connections exclusively.
// glommio::LocalExecutorBuilder::new(Placement::Fixed(core_id)).spawn(|| async move {
//     // this shard's event loop, io_uring instance, and Rc<RefCell<_>> state all live here
// });
```

### Why nginx and Envoy already made this choice, and why tokio didn't have to
nginx's worker-process-per-core model and Envoy's thread-per-core model are the C++/systems-level version of the same shard-per-core idea, predating io_uring — they distribute listening sockets via `SO_REUSEPORT` and never share connection state across workers by design. Tokio's work-stealing model exists partly *because* Rust's ownership and `Send`/`Sync` system makes safe cross-thread sharing (`Arc<Mutex<_>>`) cheap to write correctly, which C++ doesn't give for free — the safety net thread-per-core sidesteps in C++ is one Rust already has, which weakens (without eliminating) thread-per-core's traditional advantage.

### When the difference actually shows up
For most HTTP/1.1 or HTTP/2 proxy workloads bottlenecked on TLS handshakes, header parsing, or upstream round-trip time, tokio's work-stealing model isn't the bottleneck — cross-core migration overhead is small next to those costs. Thread-per-core architectures earn their complexity at very high connection-churn, very short-request workloads (millions of tiny ops/sec, think a cache or KV proxy) where per-request overhead itself, not I/O wait, dominates — exactly where [`13-algorithms/`](../13-algorithms)-style data-structure choices and [`17-performance/`](../17-performance)'s cache/NUMA concerns start mattering more than which async model was picked.

### Where [`proxy`](../../proxy) sits
[`proxy`](../../proxy) is built on tokio's work-stealing model deliberately ([`04-runtime/00-README.md`](00-README.md)) — not because thread-per-core is wrong, but because L7 HTTP proxying doesn't clearly need it, and tokio's ecosystem (hyper, rustls, h2, quinn) is what the rest of this handbook assumes. Understanding the alternative is what lets you recognize, later, whether a specific hot path inside [`proxy`](../../proxy) (a very hot cache lookup, [`05-http-stack/07-cache.md`](../05-http-stack/07-cache.md)) would benefit from being pulled into its own shard-per-core component instead of assuming tokio's default is always the right tool.

## Practice
1. Read `glommio`'s or `monoio`'s README and identify concretely which tokio APIs (`Arc<Mutex<_>>`-based ones especially) have no direct equivalent in their programming model, and write down why.
2. Benchmark [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) under high connection churn (many short-lived connections) and use `perf top` or `tokio-console` to find where the bottleneck actually sits — confirm whether it's scheduler overhead or something else (TLS handshake cost, accept-loop cost).
3. Explain, in your own words, why `SO_REUSEPORT` ([`01-network/07-socket.md`](../01-network/07-socket.md)) is load-bearing for a thread-per-core design in a way it isn't for tokio's work-stealing one.
4. Read [`16-kernel/05-rss.md`](../16-kernel/05-rss.md) and [`16-kernel/06-rps.md`](../16-kernel/06-rps.md), and connect NIC-level packet steering to why a thread-per-core proxy cares which core a connection's packets land on, while a work-stealing one mostly doesn't.
5. Write one paragraph defending [`proxy`](../../proxy)'s choice of tokio over a thread-per-core runtime, specific to what [`proxy`](../../proxy) actually does (L7 HTTP termination and proxying) rather than a generic "tokio is popular" argument.
