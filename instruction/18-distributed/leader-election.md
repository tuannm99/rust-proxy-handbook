# Leader Election

Picking exactly one node to hold a role — the sub-problem underneath Raft
(`18-distributed/raft.md`), and often solvable far more cheaply on its own.
Optional/advanced relative to the single-instance `proxy/`.

## What to learn

### Why a proxy fleet would want a leader
Some jobs must run on exactly one instance even though many are deployed:
polling a service registry and pushing config to the others, running a
periodic cache-warm or cleanup, being the single writer to a shared store.
Running them on every instance duplicates work or corrupts shared state;
running them on none means they never happen. Leader election is how the
fleet designates one, and re-designates automatically when it dies.

### The core difficulty: split-brain
The failure that makes this hard is two nodes both believing they are
leader — split-brain — which happens under a network partition where each
side cannot see the other and assumes it died. Two leaders writing shared
state is exactly the corruption you were trying to prevent. Every real
solution is fundamentally about preventing or bounding split-brain, and
the answer is always a *majority*: a node can only be leader if a majority
agrees, and a partition can have at most one majority side.

### The pragmatic answer: a lease from a store you already run
You almost never implement election from scratch. The standard pattern is a
short-lived lease (lock) in a store that itself solves consensus:

```text
leader = whoever holds key "leader" with a TTL;
  holder renews it every TTL/3;
  if the holder dies, the TTL expires and another node acquires it.
```

etcd, Consul, and ZooKeeper expose exactly this; Kubernetes' own
`leader-election` (the `Lease` object) is this pattern and is how most
Go/Rust services in a k8s environment elect a leader. Redis `SET NX PX` is
the poor-man's version. You are borrowing the store's already-correct
consensus (`18-distributed/raft.md`) instead of re-deriving it.

Gotcha: the lease TTL is a real trade-off. Too long and a dead leader's
work stalls for the whole TTL before failover; too short and a brief GC
pause or network blip makes a healthy leader lose its lease and causes
needless churn. And critically — the *old* leader must stop acting the
instant it fails to renew, before the TTL lets someone else in, or you get
two active leaders during the overlap. Fence writes (include the lease's
version/epoch on every write to the shared store, so the store rejects a
stale leader's writes) rather than trusting timing.

### When you do not need election at all
Often the cleaner design is to need no leader: make the periodic job
idempotent and let every instance run it (harmless duplication), or shard
the work by consistent hashing (`13-algorithms/consistent-hash.md`) so each
key has a natural owner without a global leader, or push the
single-writer requirement down into a store that serializes writes itself.
Electing a leader adds a failure mode (the election); avoiding the need for
one removes it. Prefer that when the job allows.

## Practice
1. Implement lease-based election with Redis `SET key id NX PX <ttl>` plus
   a renewal loop; run three processes and confirm exactly one holds the
   lease at a time.
2. Kill the leader and measure failover time; relate it directly to the TTL
   and renewal interval you chose.
3. Reproduce the danger: pause the leader process (SIGSTOP) past its TTL so
   another acquires the lease, then resume it and show the old leader
   briefly still thinks it leads — this is split-brain in miniature.
4. Add fencing: stamp each write to the shared store with the lease epoch
   and have the store reject stale epochs; show it neutralizes the step-3
   overlap.
5. For one real fleet job (config polling, cache cleanup), decide whether
   to elect a leader or make it idempotent/sharded
   (`13-algorithms/consistent-hash.md`) instead, and justify which is
   simpler for `proxy`'s actual needs.
