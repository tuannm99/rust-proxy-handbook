# Service Discovery

## What to learn
### Static config vs dynamic discovery
`01-upstream.md` assumes a fixed upstream list. Real deployments change
upstream sets constantly (autoscaling, rolling deploys, node failures).
Static config (a list in a config file, reloaded per `09-architecture/03-config.md`)
is simplest and fine for small/stable fleets. Dynamic discovery — DNS SRV
records, Consul, or Kubernetes Endpoints/EndpointSlices — is needed once
upstream membership changes faster than you want to hand-edit config.

The honest framing: dynamic discovery trades a configuration problem for a
*distributed systems* problem. You gain automatic membership updates and
acquire a new dependency whose failure modes (stale data, partial views,
unreachable control plane) now determine whether your proxy can route at
all. Most of this file is about surviving that trade.

### DNS-based discovery
Resolve a DNS name (often an SRV record, which also carries port + weight,
unlike plain A/AAAA) on an interval and diff the result against the
current pool. Cheap, dependency-free, but bounded by DNS TTL — you cannot
react faster than the TTL, and stale resolver caches (see
`01-network/09-dns.md`) can leave you pointed at a decommissioned upstream
briefly after a change.

Gotcha, and this is the classic one: **resolving once at startup is not
discovery.** `tokio::net::TcpStream::connect("upstream:8080")` resolves
each time it's called, but any code that resolves into a `SocketAddr` and
stores it has frozen DNS at process-start time. The proxy then keeps
sending traffic to an IP that was recycled to a different workload hours
ago — and because connections to it *succeed* (something is listening),
health checks don't catch it. Long-lived processes must re-resolve on a
schedule; this bug is invisible in testing and obvious in production a
week later.

Gotcha: `getaddrinfo` (what `std`/`tokio` use by default) returns
addresses but not the TTL, so your poll interval is a guess disconnected
from what the zone actually published. If TTL-correct behavior matters,
use a resolver crate that exposes records properly (`hickory-resolver`)
rather than trying to infer it.

Gotcha: a plain A record lookup may return a *subset* of addresses — many
resolvers cap the response to what fits in a UDP packet, and some
round-robin which subset they return. Diffing "what I got this poll"
against "what I had" then produces phantom removals and re-additions of
upstreams that never went anywhere. Treat a shrunken result set with
suspicion (see the empty-result guard below) rather than as truth.

### Consul / Kubernetes (push-based)
Instead of polling, subscribe to a watch/stream API that pushes upstream
set changes as they happen (Consul blocking queries, Kubernetes watch on
Endpoints/EndpointSlices, or Envoy's xDS if you're speaking that protocol).
Lower latency to detect changes than polling DNS, at the cost of a
dependency on that control plane being reachable.

Gotcha: watches break. Connections drop, the server restarts, a resource
version expires and the API tells you to start over. A watch-based
implementation is not "subscribe once" — it is a supervised loop that
reconnects with backoff (`05-retry.md`), re-lists the full state on
reconnect, and reconciles that full state against what it currently holds.
Getting the re-list path right matters more than the happy path, because
the happy path is what you test and the re-list is what runs during an
incident.

### Never accept an empty result
This is the failure mode that turns a discovery blip into a total outage,
and it is worth building the guard before you build the feature:

```rust
// WRONG: one failed lookup, and the pool is now empty
pool.store(Arc::new(discovered));

// Right: an empty discovery result is far more likely to be a bug,
// a DNS hiccup, or an unreachable control plane than a real fleet of zero.
if discovered.is_empty() {
    tracing::warn!("discovery returned no upstreams; keeping previous set");
    metrics::increment("discovery_empty_result_total");
    return;
}
pool.store(Arc::new(discovered));
```

A real fleet genuinely scaling to zero is rare; a control plane returning
nothing during its own outage is common. Keeping the last known good set
means a discovery outage degrades you to "stale routing" instead of "no
routing" — and stale routing still serves users, while an empty pool
returns 503 for everything. Alert on the warning (it is a real problem)
but do not let it take traffic down.

Gotcha: the same reasoning applies to *large* changes, not just empty
ones. A poll that removes 90% of upstreams at once is more likely a
partial view than a real event. A maximum-churn guard ("never remove more
than X% of the pool in one update without a second confirming poll") is
cheap insurance; Envoy's equivalent is its panic threshold
(`03-healthcheck.md`), applied at the membership layer instead.

### Fail static
Generalizing the above: when the control plane is unreachable, keep
serving the last known good configuration *indefinitely*, and do not
expire it on a timer. The instinct to add "if discovery data is older than
5 minutes, stop using it" feels safe and is precisely backwards — it
converts a control-plane outage (which users would not otherwise notice)
into a data-plane outage (which they certainly will). The data plane
should be able to run for days on stale membership; that property is what
makes the control plane a non-critical dependency instead of a single
point of failure.

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
half-updated one.

Note what the `Arc<Upstream>` inside the `Vec` buys you beyond atomicity:
a request that grabbed an `Arc<Upstream>` before the swap holds it alive
through its own completion, even though the new `Vec` no longer references
it. Removal becomes refcount-driven — the `Upstream` drops when the last
in-flight request using it finishes — which is exactly the drain semantics
you want, for free, instead of a manual "is anyone still using this"
check.

### Draining a removed upstream
"Remove" is three steps, not one, and doing them in the wrong order drops
traffic:
1. **Stop selecting it** for new requests (remove from the balancer's
   candidate set).
2. **Let in-flight requests finish** — the refcount behavior above handles
   this if you hold `Arc<Upstream>` per request, with a deadline for the
   ones that never finish.
3. **Close its pooled idle connections** (`01-upstream.md`) last. Skipping
   this step is the common bug: the connection pool keeps warm sockets to
   a host that discovery removed, and if your pool is keyed by address
   rather than by `Arc<Upstream>` identity, a later host at the same
   address inherits them.

Gotcha: a graceful-shutdown path (`09-architecture/04-graceful-shutdown.md`)
and a discovery-removal path are the same drain logic at different scopes.
Write it once.

## Practice
Build these in order.

1. In `labs/05-reverse-proxy`, move the hardcoded upstream list behind a
   `Discovery` trait with a static implementation. **Done when** the proxy
   behaves identically to before and nothing outside the trait knows where
   the list came from.
2. Add a DNS-poll implementation using a resolver that re-resolves on each
   tick, diffing against the current set and logging adds/removals.
   **Done when** changing a local `/etc/hosts` entry (or a local DNS
   server's zone) is picked up within one poll interval — and write the
   resolve-once-at-startup version first so you can watch it *not* pick
   it up.
3. Swap pool storage to `arc_swap::ArcSwap<Vec<Arc<Upstream>>>`. **Done
   when** a concurrent load test running during repeated pool swaps
   produces zero errors and zero torn reads.
4. Add the empty-result guard and a maximum-churn guard. **Done when**
   pointing discovery at a name that resolves to nothing leaves the
   previous pool serving traffic, emits a warning, and increments a
   metric — with zero client-visible 503s.
5. Implement the three-step drain. **Done when** removing an upstream
   mid-load-test completes every in-flight request to it (no resets), and
   `ss -tan` shows its pooled idle connections closed afterward rather
   than lingering.
6. Verify fail-static. **Done when** the discovery source is made
   permanently unreachable and the proxy keeps routing correctly on the
   last known good set for as long as you care to leave it running.
7. (Stretch) Add a Consul or Kubernetes watch-based implementation behind
   the same trait. **Done when** change-detection latency is measurably
   lower than the DNS poller's, *and* killing the watch connection
   mid-test triggers a reconnect-and-relist that reconciles correctly
   rather than duplicating or dropping upstreams.
