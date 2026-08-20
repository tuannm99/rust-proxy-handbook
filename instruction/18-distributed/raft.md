# Raft

Leader-based consensus — how a set of nodes agree on an ordered log of
changes even as some fail. Out of scope for the single-instance `proxy/`;
read this only if you extend toward a multi-node control plane or cache
(`18-distributed/distributed-cache.md`).

## What to learn

### The problem it solves
Consensus is getting N nodes to agree on a sequence of values despite
crashes and network delays, such that they never disagree. It is the
foundation under every system that stores config or coordination state
reliably — etcd, Consul, ZooKeeper (via ZAB), CockroachDB. If your proxy
ever needs a *shared, consistent* control plane (all instances agreeing on
the current routing config, or on which instance owns a shard), Raft is the
mechanism you would reach for rather than inventing your own.

### The three pieces
Raft deliberately factors consensus into parts you can reason about
separately:

- **Leader election.** One node is leader per *term* (a monotonically
  increasing number). Followers that hear nothing from a leader before a
  randomized timeout become candidates and request votes; a candidate with
  a majority becomes leader. Randomized timeouts are what break symmetry so
  two candidates rarely tie — see `18-distributed/leader-election.md`.
- **Log replication.** Clients send changes to the leader, which appends
  them to its log and replicates to followers. An entry is *committed* once
  a majority have stored it; only then is it applied to the state machine
  and acknowledged. This is why Raft needs an odd cluster size (3, 5) — a
  majority must survive.
- **Safety.** A leader only ever appends; it never overwrites its log. The
  election rules guarantee a node missing committed entries cannot win, so
  committed history is never lost.

### The quorum arithmetic that governs everything
A cluster of `2f+1` nodes tolerates `f` failures, because commit requires a
majority (`f+1`). Three nodes survive one failure; five survive two. Even
numbers buy nothing — four nodes still only tolerate one failure but cost
more coordination, so cluster sizes are odd.

```text
3 nodes → majority 2 → tolerates 1 failure
5 nodes → majority 3 → tolerates 2 failures
```

Gotcha: this majority requirement is also Raft's cost. Every committed
write waits for a round-trip to a majority, so consensus is *slow* relative
to a local operation — single-digit-thousands of writes/sec on a WAN
cluster, not millions. Never put a per-request hot path through Raft. It is
for control-plane state that changes rarely (config, membership,
leadership), not for data-plane traffic.

### Do not implement it yourself
Raft is famous for being "understandable" relative to Paxos, and still
notoriously subtle to implement correctly — leader completeness, log
compaction/snapshotting, membership changes, and the exact commit rules
each hide bugs that only surface under partition. Use a proven library
(`openraft`, `raft-rs`) if you genuinely need consensus. The value of
understanding it is knowing *when* you need it — and, far more often,
recognizing when you do not.

### When you almost certainly do not need it
Most "distributed" needs a proxy has are weaker than consensus and have
cheaper solutions: shared rate-limit counters use Redis
(`07-security/ratelimit.md`), not Raft; membership/discovery uses gossip
(`18-distributed/gossip.md`) or an existing registry
(`06-proxy/service-discovery.md`); a shared cache uses consistent hashing
(`13-algorithms/consistent-hash.md`) and tolerates inconsistency. Reach for
consensus only when nodes must *never* disagree on an ordered history.

## Practice
1. Trace a single committed write through a 3-node Raft on paper: client →
   leader append → replicate → majority ack → commit → apply. Identify the
   exact point it becomes durable.
2. Work out the failure tolerance for 3, 4, 5, and 7 nodes and articulate
   why even sizes are wasteful.
3. Reason about a partition: a 5-node cluster splits 3/2. Which side can
   commit, which cannot, and why does the minority side refuse rather than
   fork history?
4. For each "distributed" need in this handbook — rate limiting
   (`07-security/ratelimit.md`), service discovery
   (`06-proxy/service-discovery.md`), shared cache
   (`18-distributed/distributed-cache.md`) — decide whether it truly needs
   consensus or a weaker mechanism, and justify each.
5. If and only if you extend `proxy/` to a multi-node control plane: stand
   up a 3-node cluster with `openraft` storing the routing config, and kill
   the leader under load to watch election and continuity — do not
   hand-roll the algorithm.
