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

### Passive health checks
Piggyback on real traffic: if N consecutive real requests to an upstream
fail or time out, mark it down without waiting for the next active probe.
Cheaper (no extra traffic) but only reacts when live traffic is already
failing — combine both in production.

### Thresholds and flapping
Never flip health state on a single failed probe — use a threshold (e.g. 3
consecutive failures to go down, 2 consecutive successes to come back up).
Without this, an upstream near its capacity limit will flap up/down every
few seconds, which is worse for the fleet than staying marked down. This
"flap damping" is exactly what nginx's `max_fails`/`fail_timeout` and
Envoy's outlier detection implement.

### Integrating with the load balancer
The load balancer must never consider an unhealthy upstream a candidate,
but marking-down must be O(1) and lock-free from the balancer's read path
— it just checks `upstream.healthy.load(Relaxed)` before/while picking.

## Practice
1. In `milestones/03-reverse-proxy`, add an active TCP-connect probe
   loop per upstream using `tokio::time::interval`.
2. Add a consecutive-failure/-success counter and threshold before
   flipping `healthy`; log every state transition.
3. Kill and restart a dummy upstream process and confirm the pool marks it
   down then back up within roughly 2 probe intervals, not flapping in
   between.
4. Make the load balancer (from `load-balancer.md`) skip unhealthy
   upstreams and verify traffic drains off a downed one immediately.
