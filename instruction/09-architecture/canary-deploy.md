# Canary & Blue-Green Deployment

## What to learn
### Blue-green vs canary — different risk/rollback trade-offs
**Blue-green**: two full environments (old "blue," new "green"); switch 100% of traffic at once (e.g. flip which upstream pool is "active"). Rollback is instant (flip back) but you get zero signal about the new version's real-traffic behavior before it's fully live. **Canary**: shift a small percentage of traffic (1%, 5%, 25%...) to the new version while most traffic stays on the old one, watching error rate/latency before increasing the percentage. Slower, but catches bad releases while blast radius is small.

### Implementing weighted traffic splitting at the proxy layer
This builds directly on `06-proxy/load-balancer.md`: instead of one upstream pool, the router maintains two pools (stable, canary) with a weight, and picks per-request using weighted random selection or a deterministic hash (so the same client consistently lands on the same version — useful for session-sensitive testing).

```rust
fn pick_pool(stable_weight: u32, canary_weight: u32) -> Pool {
    let roll = rand::random::<u32>() % (stable_weight + canary_weight);
    if roll < canary_weight { Pool::Canary } else { Pool::Stable }
}
```
Gotcha: weighted-random splitting means a single client's requests can bounce between stable and canary across requests — fine for stateless APIs, broken for anything session-affine, where you need sticky routing (hash on a cookie/client-id) instead.

### Automated rollback triggers
A canary is only useful if something is watching it. Compare the canary pool's error rate / p99 latency (from `08-observability/metrics.md`) against the stable pool's, over the same time window, and automatically shift weight back to 0% if the canary's error rate exceeds a threshold (e.g. 2x stable's) for N consecutive intervals. Manual-only rollback is strictly worse — humans notice a canary regression slower than a metrics threshold does.

### Where this depends on service discovery
If upstream instances are registered dynamically (`06-proxy/service-discovery.md`), tag each instance with a version/pool label at registration time so the router can query "give me healthy stable instances" vs "give me healthy canary instances" instead of hardcoding addresses.

## Practice
1. Extend the load balancer in `labs/06-load-balancer` (or `proxy`) to support two named pools with configurable weights.
2. Implement weighted-random pool selection, verify with a test that traffic splits within a few % of the configured weight over many requests.
3. Add per-pool RED metrics (reuse `08-observability/metrics.md` work) so stable vs canary error rate/latency are comparable side by side.
4. Implement an automatic rollback: a background task that shifts canary weight to 0 if canary error rate exceeds stable's by a configurable multiplier for N consecutive checks.
5. Simulate a bad canary (deliberately return 500s from one upstream) under load and confirm automatic rollback fires before a human would normally notice.
