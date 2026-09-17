# Circuit Breaker

Stop calling an upstream that is consistently failing, instead of retrying
into it forever. `06-proxy/retry.md` handles individual request failures;
this is the layer above, which decides an upstream should not be called at
all for a while.

## What to learn
### The three states
```rust
enum CircuitState {
    Closed,                                 // normal, calls pass through
    Open { until: std::time::Instant },     // failing fast, no calls made
    HalfOpen,                               // one trial call allowed
}
```
Transitions: `Closed -> Open` after the failure condition trips;
`Open -> HalfOpen` once `until` elapses; `HalfOpen -> Closed` on a
successful trial call, `HalfOpen -> Open` (with `until` reset, usually
backed off further) on a failed one.

The value is in what `Open` buys: requests fail *immediately* rather than
after a connect timeout. For the client that's the difference between a
fast error it can act on and a multi-second hang; for the upstream it's
the difference between being left alone to recover and being held down by
continuing traffic.

### Trip on a failure rate, not a consecutive count
A consecutive-failure trigger is the simplest and it is noisy at low
traffic: an upstream receiving 3 requests per minute trips on 3 unlucky
failures spread over a minute, and one receiving 10,000 per second may
never see 5 consecutive failures even at a 30% error rate, because
successes keep interleaving.

Use a failure *rate* over a rolling window with a **minimum request count**
floor: "open only if ≥20 requests in the window and >50% failed." The
floor is the same guard that appears in rate-based alerting
(`08-observability/alerting.md`) and canary analysis
(`09-architecture/canary-deploy.md`) — below it, the correct verdict is
"not enough data", not "healthy" and not "failing".

Gotcha: decide what counts as a failure, and do not include client errors.
A 404 or 400 means the upstream worked correctly and the request was bad
(`06-proxy/healthcheck.md` makes the same point for passive health
checks). Counting 4xx lets one client with a broken URL scheme open the
circuit for everyone.

Gotcha: count *timeouts* as failures, and make sure the timeout is shorter
than the window you're measuring over — otherwise failures arrive too
slowly to ever constitute a rate.

### Only one trial call in HalfOpen
When `until` elapses, every in-flight request wants to be the trial call.
Letting them all through sends a thundering herd at an upstream that has,
by definition, just been failing.

Gate it with a `compare_exchange` on the state or a semaphore of size one,
so exactly one request probes and the rest continue to fail fast until it
reports back.

```rust
// only the thread that wins the CAS gets to be the trial call
if state.compare_exchange(OPEN, HALF_OPEN, AcqRel, Acquire).is_ok() {
    // this request probes
} else {
    // everyone else still fails fast
}
```

Gotcha: a single trial call is a one-sample experiment. An upstream that
is 50% broken has even odds of closing the circuit, at which point full
traffic returns and it reopens — a flap. Requiring several consecutive
successes before closing, or ramping traffic back gradually (the slow
start in `06-proxy/outlier-detection.md`), is what converts a coin flip
into a measurement.

### Back off the open duration
A fixed 30-second open period means an upstream that is down for an hour
gets probed 120 times, each probe costing a real request its latency. Back
the open duration off on each failed trial (30s, 60s, 120s, capped), and
reset it after a successful close — the same exponential-with-a-cap shape
as retry backoff (`06-proxy/retry.md`).

### Scope: per upstream, not per pool
A circuit breaker on the *pool* fails fast for everything the moment one
host misbehaves, which throws away the healthy hosts you have. Keep the
circuit per upstream instance, and let the load balancer
(`06-proxy/load-balancer.md`) route around open circuits by treating them
like unhealthy hosts.

Gotcha: that means the panic-threshold reasoning applies here too — if
every upstream's circuit is open, failing fast for 100% of traffic may be
worse than trying anyway. Decide what "all circuits open" does, exactly as
`06-proxy/healthcheck.md` decides what "all hosts unhealthy" does.

Gotcha: circuit state is per process. With N proxy instances, an upstream
must fail enough for *each* instance to trip independently, and a
restarted instance starts with every circuit closed and re-learns by
sending real traffic into a known-bad upstream
(`09-architecture/rolling-restart.md`).

### Where it sits relative to retries
Retry first, circuit second: a retry handles the individual blip, and the
circuit notices that blips have become the norm. The ordering that matters
is that **retries must respect the circuit** — retrying into an open
circuit is exactly the traffic the circuit exists to stop, so the retry's
upstream selection must skip hosts with open circuits rather than treating
"circuit open" as just another failure to retry past.

## Practice
Build these in order.

1. In `labs/05-reverse-proxy`, implement `CircuitState` per upstream with
   a consecutive-failure trigger. **Done when** logs show
   `Closed -> Open -> HalfOpen -> Closed` against an upstream you kill and
   restart.
2. Show the low-traffic false trip. **Done when** an upstream receiving
   one request per minute opens on three scattered failures — then switch
   to a rate-based trigger with a minimum request count and confirm it
   does not.
3. Exclude 4xx from the failure count and include timeouts. **Done when**
   1000 requests for a nonexistent path leave the circuit closed, and an
   upstream that accepts connections but never responds opens it.
4. Gate `HalfOpen` to a single trial call. **Done when** a concurrent load
   test at the moment the circuit half-opens shows exactly one request
   reaching the upstream.
5. Require several consecutive successes to close, and back off the open
   duration on repeated failures. **Done when** a 50%-broken upstream
   settles into a stable open state rather than flapping.
6. Make retries circuit-aware. **Done when** a retry skips hosts with open
   circuits instead of counting them as another failure.
7. Decide and implement the all-circuits-open policy. **Done when**
   killing every upstream produces your chosen behavior deliberately —
   fail fast, or try anyway — rather than whatever falls out.
