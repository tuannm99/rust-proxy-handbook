# Deadlock

## What to learn

### The four Coffman conditions
Deadlock requires all four simultaneously: mutual exclusion (a resource held exclusively), hold-and-wait (a process holds one resource while waiting for another), no preemption (a resource can't be forcibly taken away), and circular wait (a cycle of processes each waiting on a resource the next one holds). Break any single condition and deadlock becomes impossible — this is the entire strategic menu for solving it.

### Resource allocation graphs
Model processes and resources as a directed graph: an edge from process to resource means "waiting for," resource to process means "holds." With single-instance resources, a cycle in this graph is both necessary and sufficient for deadlock — detecting deadlock is literally cycle detection.

```text
A -> R1   (A waits for R1)
R1 -> B   (B holds R1)
B -> R2   (B waits for R2)
R2 -> A   (A holds R2)   => cycle => deadlock
```

### Dining philosophers
The canonical illustration: N philosophers, N forks, each philosopher needs both adjacent forks to eat. Naive "pick up left fork, then right fork" gives every philosopher hold-and-wait simultaneously — classic circular wait. Fixes: asymmetric ordering (one philosopher picks up right-then-left, breaking the cycle), a resource hierarchy (always acquire the lower-numbered fork first), or a waiter/arbitrator granting permission.

```rust
// lock ordering fix: always acquire the lower-indexed mutex first
fn eat(left: &Mutex<Fork>, right: &Mutex<Fork>, left_idx: usize, right_idx: usize) {
    let (first, second) = if left_idx < right_idx { (left, right) } else { (right, left) };
    let _a = first.lock().unwrap();
    let _b = second.lock().unwrap();
}
```
This is not a toy problem — it's the exact shape of a real bug: two locks in a connection pool ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)'s free-list lock and a stats-counter lock) acquired in different orders by two code paths deadlocks a proxy in production, and it looks exactly like this.

### Prevention, avoidance, detection+recovery
Three strategies, in order of how much they cost you. **Prevention** removes one Coffman condition structurally — a global lock ordering, or requesting all resources upfront. **Avoidance** (Banker's algorithm) grants a resource request only if the resulting state is still "safe" (some ordering exists where every process can still finish), computed dynamically — theoretically elegant, rarely implemented because it needs maximum resource needs known upfront. **Detection+recovery** lets deadlock happen, periodically checks for cycles, and kills or rolls back a process to break one. Most real systems, including Rust's own standard library, do none of these formally — they rely on prevention by convention (lock ordering discipline) because avoidance and detection are too expensive to run on a hot path.

### Why Rust doesn't save you here
Rust's borrow checker prevents data races (two threads mutating the same memory unsynchronized) at compile time, but deadlock is a liveness bug, not a memory-safety bug — the code is perfectly safe, it simply never makes progress. Nothing in the type system stops you from acquiring two `Mutex`es in inconsistent order across two code paths. `parking_lot`'s deadlock-detection feature (a debug-only cycle detector across held locks) is the closest thing to automated help, and it's opt-in, not default.

## Practice
1. Implement dining philosophers with `std::sync::Mutex` naively (always left-then-right) and reproduce a real deadlock under load — confirm all threads are stuck, not just slow.
2. Fix it with lock ordering (always acquire the lower-indexed fork first) and confirm the same load no longer deadlocks.
3. Draw the resource allocation graph, by hand, for a two-lock scenario in [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)'s connection pool where two code paths acquire a free-list lock and a stats lock in opposite order; identify the cycle.
4. Enable `parking_lot`'s `deadlock_detection` feature in a small example, deliberately deadlock two threads, and observe it report the cycle.
5. Write down your own lock-ordering rule for [`proxy`](../../proxy)'s actual locks (if it has more than one) and add it as a code comment at each lock's definition site — this is "prevention by convention" in practice.
