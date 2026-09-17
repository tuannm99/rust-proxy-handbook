# Health Check

## What to learn
### Active health checks
The proxy periodically sends its own probe request (TCP connect, or an
HTTP GET to `/healthz`) to each upstream on a timer, independent of real
traffic. This is the only way to detect a dead upstream that simply isn't
receiving live requests right now.

```rust
async fn probe_loop(upstream: std::sync::Arc<Upstream>, interval: std::time::Duration) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let ok = tokio::time::timeout(std::time::Duration::from_secs(2), tcp_probe(&upstream.addr))
            .await
            .is_ok();
        upstream.healthy.store(ok, std::sync::atomic::Ordering::Relaxed);
    }
}
```
Gotcha: the probe timeout must be shorter than the probe interval, or slow
probes pile up. Always bound the probe with `tokio::time::timeout`.

A second, subtler gotcha with `tokio::time::interval`: its default
`MissedTickBehavior::Burst` fires immediately and repeatedly to "catch up"
if a tick was missed (because a probe ran long). That converts one slow
probe into a burst of back-to-back probes against an upstream that is
already struggling. `ticker.set_missed_tick_behavior(MissedTickBehavior::Delay)`
is almost always what a health checker wants.

### What each probe type actually proves
Probe depth is a real design decision, not a detail:
- **TCP connect** proves the kernel accepted a connection. It does *not*
  prove any application is behind the socket — a process stuck in an
  infinite loop, or one whose accept backlog is being drained by the
  kernel alone, still passes (`16-kernel/tcp-stack.md`).
- **HTTP GET `/healthz` returning 200** proves the HTTP server loop is
  alive and scheduling work. It does not prove the upstream can serve
  *real* requests if `/healthz` is a static handler that touches nothing.
- **A deep check** (the handler verifies its database, cache, or
  downstream dependency) proves the upstream can do real work — and
  introduces the failure mode below.

Gotcha: the most common health check in the wild is a static 200 handler,
which detects exactly one class of failure (process dead) that TCP connect
already detects more cheaply. If your `/healthz` doesn't touch anything the
real request path touches, you have a liveness check, not a health check —
know which one you built.

### Deep checks and correlated failure
A deep check makes each upstream's health depend on a *shared* dependency,
so when that dependency has a blip, every upstream in the fleet fails its
probe at the same instant. The proxy dutifully marks all of them unhealthy
and now has zero candidates — converting a degraded database into a total
outage, with the proxy as the mechanism that did it.

Every serious proxy has a guard for this. Envoy calls it the **panic
threshold**: if the fraction of healthy hosts falls below a threshold
(50% by default), Envoy *ignores health status entirely* and load balances
across all hosts, on the reasoning that if most of the fleet looks dead,
the more likely explanation is that the health signal is wrong. Implement
the equivalent before you ship deep checks:

```rust
// pick among healthy upstreams, but fall back to the whole pool
// once too few are healthy to trust the signal
let healthy: Vec<_> = pool.iter().filter(|u| u.is_healthy()).collect();
let candidates = if (healthy.len() as f64) < 0.5 * pool.len() as f64 {
    pool.all()      // panic mode: health signal is not credible
} else {
    healthy
};
```
Gotcha: fail-open is right for a *shared* dependency and wrong for a
per-upstream one. If upstreams fail independently (bad deploy on one host,
one disk full), panic mode sends traffic to genuinely dead hosts. Deep
checks should test dependencies the upstream owns exclusively; shared
dependencies belong in an alert (`08-observability/alerting.md`), not in a
per-host health verdict.

### Passive detection, damping, and slow start
Active probes are only half the picture. Real request failures detect a
bad upstream faster than any probe interval, thresholds keep a single blip
from draining a host, and a ramp keeps a recovered host from being
stampeded the instant it comes back.

All three live in `06-proxy/outlier-detection.md`. The division: this file
is "we went and asked"; that one is "we noticed from the traffic we were
already sending, and we damped our reaction."

### Probe cost at fleet scale
Probe traffic is `proxies × upstreams × (1 / interval)` requests per
second, and it is paid whether or not any real traffic exists. Twenty
proxy instances probing 100 upstreams every second is 2000 req/s of pure
overhead hitting the fleet, and every one of those probes lands at the
same instant if every proxy started from a config deploy that rolled out
together.

Jitter the interval per upstream (a random offset on the first tick is
enough) so probes spread across the window instead of hammering in
lockstep — the same synchronization problem as retry storms in
`retry.md`, with the same fix.

## Practice
Build these in order.

1. In `labs/05-reverse-proxy`, add an active TCP-connect probe loop per
   upstream with `tokio::time::interval`, bounded by `tokio::time::timeout`
   and using `MissedTickBehavior::Delay`. **Done when** a dummy upstream
   you `kill -STOP` (not kill — stopped, so the socket stays open) is
   still reported healthy, and you can explain why from the probe-depth
   section above.
2. Replace it with an HTTP `/healthz` probe. **Done when** the `kill -STOP`
   case now correctly reports unhealthy.
3. Add the panic-threshold fallback. **Done when** a test that makes
   *every* upstream fail its probe still routes traffic (rather than
   returning 503 to everything), and a test that fails only one upstream
   still excludes exactly that one.
4. Add per-upstream jitter to the probe schedule. **Done when** probe
   arrival timestamps at one upstream, from 3 concurrently running proxy
   instances, are spread across the interval instead of clustered.
5. Measure probe cost. **Done when** you can state the requests per second
   your probes generate at your fleet size, and it is a number you are
   willing to pay.
6. Work through `06-proxy/outlier-detection.md` for passive detection,
   flap damping, and slow start. **Done when** an upstream that fails real
   requests is ejected before the next probe fires, and a recovered one
   ramps back rather than being stampeded.
