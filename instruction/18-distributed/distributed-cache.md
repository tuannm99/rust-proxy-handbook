# Distributed Cache

Spreading a cache across nodes once a single-node cache
(`05-http-stack/cache.md`, `13-algorithms/lru.md`) is not enough. The one
`18-distributed/` topic that connects most directly to a proxy — but still
beyond the single-instance `proxy/` deliverable.

## What to learn

### Why go distributed at all
A per-instance cache in a fleet of N proxies has two problems: the same
object is cached N times (N× the memory, N× the origin misses on cold
start), and hit rate is capped by one instance's memory. A distributed
cache makes the fleet share one logical cache — each object lives on one
(or a few) nodes, so aggregate capacity is the sum and each object is
fetched from origin roughly once. This is the CDN edge-cache problem.

### Placement: consistent hashing, not modulo
Which node owns a key must stay stable as nodes join and leave, or every
membership change reshuffles the whole cache and stampedes the origin. This
is exactly the problem `13-algorithms/consistent-hash.md` (and
`13-algorithms/maglev.md`, `13-algorithms/rendezvous-hash.md`) solve:
`hash(key)` maps to a point on a ring, the key belongs to the next node
clockwise, and adding a node only moves the keys in one arc rather than all
of them. Reuse that algorithm here directly — distributed caching is its
headline application.

```text
key "GET /img/x" → hash → ring position → owning node
add a node → only keys in its arc move → ~1/N reshuffled, not all
```

### The two topologies
- **Client-side sharding.** Each proxy knows the ring and routes a cache
  lookup straight to the owning peer (or to a shared store like a
  Memcached/Redis pool sharded by consistent hash). Simple, low-latency, no
  extra hop's worth of coordination. This is the common proxy design.
- **Server-side / peer forwarding.** A proxy that receives a request it
  does not own forwards it to the owner, which caches and serves. Groupcache
  and Nginx's shared-cache-cluster setups work this way.

Gotcha: every distributed-cache lookup is now a *network* operation, not a
memory read. A remote hit costs a round-trip; if that round-trip approaches
the origin fetch time, the distributed cache is not worth it. The usual
answer is two tiers — a small fast *local* cache (`13-algorithms/lru.md`)
in front of the distributed one — so hot objects never leave the process
and only the long tail goes over the network.

### Consistency and invalidation are the hard part
Caches across nodes drift. When an object is purged or updated, every node
holding it must learn — and there is no cheap strong-consistency answer.
The practical tools: short TTLs so staleness self-heals, versioned keys so
an update writes a *new* key rather than mutating, and a purge broadcast
(often over gossip, `18-distributed/gossip.md`) accepting brief
inconsistency. A proxy cache almost always chooses eventual consistency
here — strong consistency (`18-distributed/raft.md`) on a data-path cache
would cost more than it saves.

### Thundering herd across the fleet
When a popular object expires, every proxy that gets a request for it can
hit the origin simultaneously — a fleet-wide stampede far worse than the
single-node version (`05-http-stack/cache.md`'s request coalescing). The
distributed fix is that only the *owning* node fetches from origin and the
others coalesce onto it, plus request-coalescing/single-flight on that
owner. Placement (consistent hashing) is what makes "only the owner
fetches" possible.

## Practice
1. Extend the single-node cache from `labs/10-cache` with consistent-hash
   placement (`13-algorithms/consistent-hash.md`) across a simulated set of
   3 nodes; verify a key always resolves to the same node.
2. Add a node and measure the fraction of keys that move — confirm it is
   ~1/N, not everything, and contrast with `hash % N` placement which
   reshuffles all of them.
3. Build the two-tier shape: a small local LRU in front of the distributed
   lookup, and measure how much network traffic the local tier absorbs on
   a hot-object workload.
4. Reproduce a fleet-wide thundering herd on expiry, then fix it with
   owner-only origin fetch plus single-flight coalescing on the owner.
5. Implement purge with versioned keys and a gossiped invalidation
   (`18-distributed/gossip.md`); reason explicitly about the staleness
   window and why eventual consistency is the right call for a proxy cache
   versus `18-distributed/raft.md`.
