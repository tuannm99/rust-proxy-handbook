# Service Discovery

## What to learn
### Static config vs dynamic discovery
`upstream.md` assumes a fixed upstream list. Real deployments change
upstream sets constantly (autoscaling, rolling deploys, node failures).
Static config (a list in a config file, reloaded per `09-architecture/config.md`)
is simplest and fine for small/stable fleets. Dynamic discovery — DNS SRV
records, Consul, or Kubernetes Endpoints/EndpointSlices — is needed once
upstream membership changes faster than you want to hand-edit config.

### DNS-based discovery
Resolve a DNS name (often an SRV record, which also carries port + weight,
unlike plain A/AAAA) on an interval and diff the result against the
current pool. Cheap, dependency-free, but bounded by DNS TTL — you cannot
react faster than the TTL, and stale resolver caches (see
`01-network/dns.md`) can leave you pointed at a decommissioned upstream
briefly after a change.

### Consul / Kubernetes (push-based)
Instead of polling, subscribe to a watch/stream API that pushes upstream
set changes as they happen (Consul blocking queries, Kubernetes watch on
Endpoints/EndpointSlices). Lower latency to detect changes than polling
DNS, at the cost of a dependency on that control plane being reachable.

### Reacting to membership changes without dropping traffic
The critical invariant: updating the upstream set must never interrupt
in-flight requests to an upstream that's being removed, and must never
route new requests to a stale reference.

```rust
struct UpstreamPool {
    upstreams: arc_swap::ArcSwap<Vec<std::sync::Arc<Upstream>>>,
}
```
Swap the whole `Vec` atomically with `arc-swap` (or a `RwLock` if you don't
want the extra dependency) rather than mutating in place — readers picking
an upstream always see either the old or new complete list, never a
half-updated one. An upstream being removed should be drained (stop
sending it *new* requests, let in-flight ones finish) rather than
force-closed.

## Practice
1. In `proxy`, add a DNS-poll-based discovery loop
   (interval + `tokio::net::lookup_host` or an SRV-aware resolver crate)
   that diffs against the current pool and logs adds/removals.
2. Swap the pool's storage to `arc_swap::ArcSwap` and confirm (with a
   concurrent load test) that in-flight requests never see a torn/partial
   upstream list.
3. Simulate a removal: stop sending a soon-to-be-removed upstream new
   requests but let its current in-flight ones complete before dropping it
   from the pool entirely.
4. (Stretch) Swap the DNS poller for a Consul or Kubernetes watch-based
   source and compare change-detection latency.
