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

### Passive health checks
Piggyback on real traffic: if N consecutive real requests to an upstream
fail or time out, mark it down without waiting for the next active probe.
Cheaper (no extra traffic) and much faster to react — a 5s active probe
interval means up to 5s of requests sent into a hole, while passive
detection catches it on the first few failures.

The two are complementary in a specific way: passive checks can only mark
an upstream *down* (you learn nothing about an upstream getting no
traffic), while active checks are the only thing that can bring it back
*up*. A pool with only passive checking is a one-way door — once an
upstream is marked down it stops receiving the traffic that would prove
it recovered.

Gotcha: don't count client-caused errors as upstream failures. A 404 or a
400 means the upstream is working correctly and the *request* was bad;
counting 4xx toward a failure threshold lets one client with a broken URL
scheme mark your entire fleet unhealthy. Count connection errors,
timeouts, and 5xx — and be deliberate about 503, which often means "this
upstream is deliberately shedding load" rather than "broken".

### Thresholds and flapping
Never flip health state on a single failed probe — use a threshold (e.g. 3
consecutive failures to go down, 2 consecutive successes to come back up).
Without this, an upstream near its capacity limit will flap up/down every
few seconds, which is worse for the fleet than staying marked down. This
"flap damping" is exactly what nginx's `max_fails`/`fail_timeout` and
Envoy's outlier detection implement.

Make the thresholds asymmetric, and in the direction that may surprise
you: go down slowly (3+ failures, so a single blip doesn't drain an
upstream) but come back up *even more* slowly (more successes, plus a
minimum time in the down state). Coming back too eagerly is what creates
flapping, because the upstream that just recovered is immediately handed a
full share of traffic and falls over again.

Gotcha: consecutive-failure counters must be reset atomically with the
state transition, or two concurrent probe results can both observe
"2 failures" and both increment to 3, double-counting a single failure.
Keep the counter and the state under one atomic operation, or one small
mutex per upstream — this is per-upstream, off the request hot path, so a
mutex here is fine.

### Slow start: the recovery stampede
An upstream that just came back healthy has zero active connections, which
makes it the most attractive candidate for least-connection balancing
(`load-balancer.md`) and for any consistent-hash ring that just re-added
it. It receives a disproportionate burst of traffic in the first seconds
after recovery — into a process with cold caches, an empty connection
pool, and a JIT/page cache that hasn't warmed — and frequently falls over
again, producing a flap that the threshold logic above can't prevent
because the upstream really is failing.

The fix is a ramp: for the first `T` seconds after an upstream becomes
healthy, scale its effective weight from near-zero up to its configured
weight, so it receives a growing trickle rather than a flood. nginx
(`slow_start=`) and Envoy (`slow_start_config`) both implement exactly
this.

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

### Integrating with the load balancer
The load balancer must never consider an unhealthy upstream a candidate,
but marking-down must be O(1) and lock-free from the balancer's read path
— it just checks `upstream.healthy.load(Relaxed)` before/while picking.

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
3. Add consecutive-failure/-success thresholds (3 down, 2 up) with a
   minimum time in the down state; log every transition. **Done when**
   killing and restarting an upstream shows exactly two transitions in the
   logs — not a burst of them — and recovery takes at least your minimum
   down-time.
4. Add passive health checking on real request failures, counting only
   connection errors, timeouts, and 5xx. **Done when** a load test against
   a killed upstream marks it down in under one probe interval, and a test
   that sends 1000 requests for a nonexistent path (all 404s) leaves it
   healthy.
5. Add the panic-threshold fallback. **Done when** a test that makes
   *every* upstream fail its probe still routes traffic (rather than
   returning 503 to everything), and a test that fails only one upstream
   still excludes exactly that one.
6. Add slow start. **Done when** you can chart requests-per-second to a
   recovering upstream over the first 30s and see a ramp rather than a
   step — and confirm the recovery flap from step 3 disappears under a
   load test that pushes the upstream near its capacity limit.
7. Add per-upstream jitter to the probe schedule. **Done when** probe
   arrival timestamps at one upstream, from 3 concurrently running proxy
   instances, are spread across the interval instead of clustered.
