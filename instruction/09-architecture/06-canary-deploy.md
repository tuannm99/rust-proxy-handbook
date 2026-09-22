# Canary & Blue-Green Deployment

## What to learn
### Blue-green vs canary — different risk/rollback trade-offs
**Blue-green**: two full environments (old "blue," new "green"); switch 100% of traffic at once (e.g. flip which upstream pool is "active"). Rollback is instant (flip back) but you get zero signal about the new version's real-traffic behavior before it's fully live. **Canary**: shift a small percentage of traffic (1%, 5%, 25%...) to the new version while most traffic stays on the old one, watching error rate/latency before increasing the percentage. Slower, but catches bad releases while blast radius is small.

Gotcha: blue-green's "instant rollback" is only true for *stateless*
changes. If the green environment wrote to a shared database with a new
schema, flipping back leaves blue reading data it doesn't understand —
which is not a rollback, it's a second incident. Both strategies require
that the two versions can run **simultaneously against shared state**,
which is a constraint on the application (expand-migrate-contract schema
changes, backward-compatible message formats), not something the proxy
can provide.

### Implementing weighted traffic splitting at the proxy layer
This builds directly on `06-proxy/02-load-balancer.md`: instead of one upstream pool, the router maintains two pools (stable, canary) with a weight, and picks per-request using weighted random selection or a deterministic hash (so the same client consistently lands on the same version — useful for session-sensitive testing).

```rust
fn pick_pool(stable_weight: u32, canary_weight: u32) -> Pool {
    let roll = rand::random::<u32>() % (stable_weight + canary_weight);
    if roll < canary_weight { Pool::Canary } else { Pool::Stable }
}
```
Gotcha: weighted-random splitting means a single client's requests can bounce between stable and canary across requests — fine for stateless APIs, broken for anything session-affine, where you need sticky routing (hash on a cookie/client-id) instead.

Gotcha: sticky-by-hash only works if every proxy instance computes the
same hash. A per-process random seed (`13-algorithms/hashmap.md`'s
`DefaultHasher` warning) means instance A sends a user to canary and
instance B sends the same user to stable — producing exactly the
version-flapping that stickiness was supposed to prevent. Use a
fixed-seed hash, and include the pool weights in nothing but the
threshold comparison, so a weight change moves the minimum number of
users.

Gotcha: a client bouncing between versions is worse than it sounds when
the versions differ in behavior — a browser that loads `index.html` from
canary and its hashed JS bundle from stable gets a 404
(`05-http-stack/05-static.md`'s immutable assets), and the user sees a broken
page rather than a clean error.

### Automated rollback triggers
A canary is only useful if something is watching it. Compare the canary pool's error rate / p99 latency (from `08-observability/02-metrics.md`) against the stable pool's, over the same time window, and automatically shift weight back to 0% if the canary's error rate exceeds a threshold (e.g. 2x stable's) for N consecutive intervals. Manual-only rollback is strictly worse — humans notice a canary regression slower than a metrics threshold does.

### The statistics problem nobody warns you about
A 1% canary receives 1% of the traffic, and therefore 1% of the *samples*.
At 1000 req/s overall, the canary sees 10 req/s — so in a 1-minute window
it has 600 requests, and a single burst of 12 errors is "2% error rate,
double stable's 1%." That comparison is noise, and a rollback automation
that trusts it will roll back healthy releases roughly forever, training
everyone to ignore it.

Two guards, both required:
- **Minimum sample count** before any comparison is evaluated — the same
  floor as the alerting ratio in `08-observability/06-alerting.md` and the
  rate-based circuit breaker in `06-proxy/05-retry.md`. Below it, the correct
  verdict is "not enough data," not "healthy" and not "failing."
- **Compare like with like.** Canary and stable must be measured over the
  same window, and ideally over the same traffic mix — if your canary
  happens to receive a disproportionate share of one expensive endpoint
  (because of hash stickiness, or because it's in one region), the
  difference you measure is the traffic, not the code.

Gotcha: p99 is especially noisy at low sample counts — the 99th percentile
of 600 requests is the 6th-worst request. Prefer error rate and p50 for
early, small canaries; wait for meaningful volume before trusting tail
latency comparisons.

### What a canary cannot catch
Worth knowing so a green canary isn't mistaken for proof:
- **Resource leaks.** A memory leak (`14-memory/06-fragmentation.md`) or fd
  leak takes 100x longer to manifest at 1% traffic. A canary that runs for
  an hour tells you nothing about a leak that kills a full-traffic
  instance in a day.
- **Load-dependent failures.** Lock contention, connection pool exhaustion
  (`06-proxy/01-upstream.md`), and thundering herds only appear near capacity
  — which a 1% canary is nowhere near.
- **Time-dependent bugs.** A daily batch, a certificate expiry
  (`01-network/13-tls.md`), a month-boundary calculation.
- **Anything downstream.** If the canary shares upstreams and a database
  with stable, it can't reveal a problem in the shared dependency — and
  can *cause* one that harms stable traffic too.

The mitigations are to hold at a meaningful percentage (25-50%) for a
meaningful duration before going to 100%, and to keep watching the
release after it's fully rolled out — most releases that fail, fail after
the deploy is declared complete.

### Where this depends on service discovery
If upstream instances are registered dynamically (`06-proxy/07-service-discovery.md`), tag each instance with a version/pool label at registration time so the router can query "give me healthy stable instances" vs "give me healthy canary instances" instead of hardcoding addresses.

Gotcha: canary pools are small — often a single instance — so the
panic-threshold and health-check logic from `06-proxy/03-healthcheck.md`
behaves differently there. One unhealthy instance in a 20-instance stable
pool is a non-event; one unhealthy instance in a 1-instance canary pool is
100% of that pool, and your fail-open panic mode may route *stable*
traffic to it. Evaluate health per pool, not across the merged set.

## Practice
Build these in order.

1. Extend `labs/06-load-balancer` (or `proxy`) to support two named pools
   with configurable weights, sourced from config
   (`09-architecture/03-config.md`). **Done when** weights can change without
   a restart.
2. Implement weighted-random selection. **Done when** a test over 100k
   requests shows the split within a percent of the configured weight.
3. Add sticky-by-hash selection with a fixed-seed hash. **Done when** the
   same client ID lands on the same pool across a process restart *and*
   across three concurrently running proxy instances — test the
   multi-instance case explicitly, since that's where the seed bug hides.
4. Add per-pool RED metrics with pool as a label. **Done when** stable and
   canary error rate and latency are directly comparable on one dashboard.
5. Implement automatic rollback with a minimum sample floor. **Done when**
   a canary at 1% weight with 3 errors in a minute does *not* roll back,
   and a canary genuinely returning 50% errors does — the first case is
   the one that proves the guard works.
6. Simulate a bad canary under load (`12-testing/01-load-testing.md`) with
   one upstream returning 500s. **Done when** automatic rollback fires
   within your target window and the total number of client-visible errors
   is bounded by the canary weight, not by the time a human took to react.
7. Evaluate health per pool. **Done when** the single canary instance
   going unhealthy takes the canary pool to zero weight without triggering
   panic-mode routing of stable traffic to it.
8. Write down what your canary process cannot catch, and what you do
   instead. **Done when** the deployment runbook names a hold duration at
   a meaningful percentage and a post-rollout watch period, with the
   reasoning attached.
