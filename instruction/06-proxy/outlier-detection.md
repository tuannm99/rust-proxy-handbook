# Outlier Detection

Deciding an upstream is bad from the traffic you're already sending it,
rather than from a dedicated probe. `06-proxy/healthcheck.md` covers
active probing; this file covers the passive half and the damping that
keeps either of them from causing more harm than the failure did.

## What to learn
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
upstream is deliberately shedding load"
(`07-security/load-shedding.md`) rather than "broken".

Gotcha: this is the same failure-classification question as
`06-proxy/circuit-breaker.md`'s trip condition, and the two should agree.
An upstream whose circuit is open but which passive health checking still
considers healthy produces contradictory routing decisions.

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

### Ejection needs a floor, too
Passive detection at scale can eject faster than you intend: a shared
dependency hiccups, every upstream fails a few requests at once, and the
thresholds trip across the whole pool simultaneously. That's the same
correlated-failure shape as a deep health check
(`06-proxy/healthcheck.md`), reached by a different route.

Envoy's answer is `max_ejection_percent` — never eject more than a
configured fraction of the pool, no matter what the signal says. Below
that fraction the signal is credible; above it, the more likely
explanation is that the problem isn't the hosts.

### Slow start: the recovery stampede
An upstream that just came back healthy has zero active connections, which
makes it the most attractive candidate for least-connection balancing
(`06-proxy/load-balancer.md`) and for any consistent-hash ring that just
re-added it. It receives a disproportionate burst of traffic in the first
seconds after recovery — into a process with cold caches, an empty
connection pool, and a JIT/page cache that hasn't warmed — and frequently
falls over again, producing a flap that the threshold logic above can't
prevent because the upstream really is failing.

The fix is a ramp: for the first `T` seconds after an upstream becomes
healthy, scale its effective weight from near-zero up to its configured
weight, so it receives a growing trickle rather than a flood. nginx
(`slow_start=`) and Envoy (`slow_start_config`) both implement exactly
this.

Gotcha: the same ramp applies to a *newly added* upstream from service
discovery (`06-proxy/service-discovery.md`), not just a recovered one —
a freshly-scaled-out instance is cold in exactly the same way, and
least-connection routing finds it just as attractive.

### Integrating with the load balancer
The load balancer must never consider an unhealthy upstream a candidate,
but marking-down must be O(1) and lock-free from the balancer's read path
— it just checks `upstream.healthy.load(Relaxed)` before/while picking.

Gotcha: an upstream in slow start is neither "healthy" (full weight) nor
"unhealthy" (excluded) — it needs a third state, or a weight the balancer
reads rather than a boolean. Modelling health as a `bool` is what makes
slow start awkward to add later; a small enum or an effective-weight value
costs nothing now.

## Practice
Build these in order.

1. In `labs/05-reverse-proxy`, add passive health checking on real request
   failures, counting only connection errors, timeouts, and 5xx. **Done
   when** a load test against a killed upstream marks it down in under one
   active probe interval, and a test sending 1000 requests for a
   nonexistent path (all 404s) leaves it healthy.
2. Add consecutive-failure/-success thresholds (3 down, 2 up) with a
   minimum time in the down state, and log every transition. **Done when**
   killing and restarting an upstream shows exactly two transitions —
   not a burst — and recovery takes at least your minimum down-time.
3. Make the counter and state transition atomic. **Done when** concurrent
   probe results cannot double-count a single failure — test with many
   threads reporting failures simultaneously.
4. Add a maximum ejection percentage. **Done when** a fault that makes
   every upstream fail simultaneously ejects only up to your configured
   fraction, leaving the rest serving.
5. Model health as an effective weight rather than a boolean, and add slow
   start. **Done when** you can chart requests-per-second to a recovering
   upstream over the first 30s and see a ramp rather than a step.
6. Verify the flap is gone. **Done when** the recovery flap from step 2
   disappears under a load test that pushes the upstream near its
   capacity limit.
7. Apply the ramp to newly discovered upstreams too
   (`06-proxy/service-discovery.md`). **Done when** adding an instance
   mid-load-test gives it a ramp rather than an immediate full share.
