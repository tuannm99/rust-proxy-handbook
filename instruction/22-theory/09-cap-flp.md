# CAP Theorem and the FLP Impossibility Result

## What to learn

### CAP: pick two, under partition
For a distributed data store, CAP theorem says it is impossible to simultaneously guarantee all three of Consistency (every read sees the most recent write), Availability (every request gets a non-error response), and Partition tolerance (the system keeps working when network communication between nodes fails) — and since real networks *do* partition, the actual choice in practice is between C and A *during* a partition, not a free choice among all three. This is the formal justification behind every "eventual consistency here" decision [`18-distributed/`](../18-distributed) makes.

```text
Partition happens (network split): a choice must be made
-> refuse some requests until the partition heals  (choosing C over A)
-> serve requests on both sides, risk stale/conflicting reads  (choosing A over C)
```

### Where [`18-distributed/`](../18-distributed) already made this choice, made explicit
[`18-distributed/04-distributed-cache.md`](../18-distributed/04-distributed-cache.md)'s purge/invalidation design (short TTLs, versioned keys, gossiped invalidation, "accepting brief inconsistency") chooses Availability over Consistency during a partition — a cache serving slightly stale data is far better than a cache that stops serving. [`18-distributed/01-raft.md`](../18-distributed/01-raft.md)'s consensus, in contrast, chooses Consistency over Availability by design: a Raft cluster that cannot reach a majority refuses to commit writes at all rather than risk two sides disagreeing — precisely why Raft is reserved for control-plane state (config, leadership) and never for the data-plane request path.

### FLP: why consensus cannot be solved without something extra
The Fischer-Lynch-Paterson (FLP) impossibility result proves something stronger than CAP for a narrower setting: in a fully *asynchronous* network (no bound on message delay) where even one node might crash, no deterministic algorithm can guarantee consensus terminates — it is impossible to reliably distinguish "that node is slow" from "that node is dead" without some timing assumption. This is not an engineering limitation waiting to be optimized away; it is a proof.

### How every real consensus system quietly sidesteps FLP
Since FLP's asynchronous, fully-adversarial model is unsolvable in general, every real system adds one extra assumption FLP disallows: Raft and Paxos use *timeouts* (randomized election timeouts in [`18-distributed/03-leader-election.md`](../18-distributed/03-leader-election.md) — an explicit, partial-synchrony assumption that "long enough silence probably means dead, not just slow"), trading perfect correctness in the worst case for termination in the common case. This is exactly why [`18-distributed/03-leader-election.md`](../18-distributed/03-leader-election.md)'s lease-TTL discussion calls the TTL choice "a real trade-off": too short falsely declares a live node dead (an FLP-flavored false positive), too long delays recovery. No TTL eliminates the trade-off; FLP says none can exist.

### Why this matters for a proxy that mostly avoids distributed state
[`proxy`](../../proxy) is a single instance by design ([`18-distributed/00-README.md`](../18-distributed/00-README.md) says so directly), which sidesteps both of these results entirely — CAP and FLP are theorems about systems that must coordinate *state* across unreliable networks, and a single-instance proxy with no coordinated state has neither problem. The value of knowing them is recognizing the exact moment a design decision crosses that line: the instant [`proxy`](../../proxy) needs two instances to agree on anything — a shared rate-limit counter, a leader-elected job, a replicated cache — CAP and FLP stop being theoretical and start being the actual constraints on what is achievable, no matter how good the implementation is.

## Practice
1. For [`18-distributed/04-distributed-cache.md`](../18-distributed/04-distributed-cache.md)'s purge design, write down explicitly which of C or A it sacrifices during a network partition between cache nodes, and why that is the right choice for a cache specifically.
2. For [`18-distributed/01-raft.md`](../18-distributed/01-raft.md)'s consensus, do the same — identify what it sacrifices, and explain why that is the right choice for control-plane config instead.
3. Simulate FLP's core problem directly: build a toy 3-node "leader election" with no timeouts at all (pure message-passing, no clock), and demonstrate it can hang forever if one node's messages are merely delayed rather than lost — confirm no fixed protocol logic alone can distinguish "slow" from "dead" without a timing assumption.
4. Add randomized election timeouts to the same toy system (as real Raft does) and show it now reliably terminates — but construct a pathological message-delay pattern that still causes a false "dead" verdict on a live node, and connect this back to the TTL trade-off in [`18-distributed/03-leader-election.md`](../18-distributed/03-leader-election.md).
5. Pick one real or hypothetical feature that would require [`proxy`](../../proxy) to run as more than one coordinating instance (a shared rate limiter, a cluster-wide cache) and write one paragraph on which side of CAP would be chosen and why, citing the specific [`18-distributed/`](../18-distributed) file that already made this call for a similar problem.
