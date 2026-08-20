# Gossip

Epidemic-style state propagation — how a large fleet learns membership and
health without any central coordinator. Optional/advanced relative to
`proxy/`; relevant if you run many proxy instances that need to know about
each other.

## What to learn

### The idea, and how it differs from consensus
In a gossip protocol each node periodically picks a few random peers and
exchanges state; information spreads like an infection, reaching the whole
fleet in `O(log N)` rounds. Crucially it provides *eventual* consistency,
not the strong agreement of Raft (`18-distributed/raft.md`) — nodes may
briefly disagree, and that is the deliberate trade for scale and
partition-tolerance. Consensus is for state that must never diverge;
gossip is for state where "everyone converges within a few seconds" is
fine, which is exactly right for membership and health.

### What it is used for
The canonical use is cluster membership and failure detection — which nodes
exist and which are alive. This is what Consul, Cassandra, and Serf use
(all built on SWIM or a variant). For a proxy fleet the payoff is
decentralized health awareness: instead of every proxy independently
polling every backend (`06-proxy/healthcheck.md`), nodes gossip health
observations, so the fleet converges on "backend X is down" with far less
total probe traffic.

### SWIM: the failure detector worth knowing
SWIM (Scalable Weakly-consistent Infection-style Membership) is the modern
standard because it separates two concerns naive gossip conflates:

- **Failure detection** by direct ping, with *indirect* probing as a
  second chance — if A cannot reach B, A asks a few other nodes to ping B
  on its behalf before declaring B dead. This is what suppresses false
  positives from a single bad link.
- **Dissemination** of membership changes piggybacked on those ping
  messages, so detection and propagation share traffic.

```text
A ──ping──▶ B      (no reply)
A ──"ping B for me"──▶ C, D   (indirect probe before declaring B dead)
```

Gotcha: the indirect-probe step is the whole reason SWIM is usable in
production. Without it, one flaky network path between two nodes marks a
healthy node dead and the misinformation gossips fleet-wide. Any gossip
system you build or configure must have this second-opinion mechanism, or
it will flap.

### Anti-entropy vs rumor-mongering
Two propagation styles, usually combined: *rumor-mongering* spreads a new
fact aggressively for a while then stops (fast, but a node that missed it
stays stale), and *anti-entropy* periodically reconciles full state between
peers (slow, but guarantees eventual convergence and repairs missed
rumors). Production systems run both — rumors for speed, anti-entropy as
the backstop.

### The cost model
Gossip trades latency and consistency for scalability and resilience: no
single point of failure, load spread evenly, but convergence takes several
rounds and nodes are transiently inconsistent. Tune the fanout (peers per
round) and period against fleet size — too aggressive wastes bandwidth, too
lazy slows convergence and failure detection.

### Do not hand-roll it
Like Raft, correct gossip is subtle (false-positive suppression, message
amplification, incarnation numbers to resolve stale-vs-fresh state). Use
`memberlist` (Go, via Serf) as the reference, or a Rust SWIM crate, rather
than inventing your own. Understanding it tells you when decentralized
health beats centralized polling — which for a small proxy fleet it often
does not.

## Practice
1. Simulate rumor spread: N nodes, each round every "infected" node tells
   `k` random peers; measure rounds to full coverage and confirm the
   `O(log N)` shape as you scale N.
2. Add a flaky link between two nodes and show naive direct-only detection
   produces a false "dead" verdict; then add SWIM-style indirect probing
   and show it suppresses the false positive.
3. Compare total probe traffic for fleet health two ways: every proxy
   polling every backend (`06-proxy/healthcheck.md`) vs gossiped health
   observations, as fleet and backend counts grow.
4. Reason about convergence vs consistency: construct a moment where two
   proxies disagree on a backend's health and decide whether that
   transient disagreement is acceptable for *your* traffic (it usually is
   for load-balancing, not for billing).
5. Only if you run a multi-instance fleet: wire a SWIM library into `proxy`
   for instance membership and observe a killed instance being detected and
   removed — do not implement the protocol yourself.
