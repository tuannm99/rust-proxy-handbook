# Load Shedding

What to do when arriving work exceeds capacity. The short answer is
"reject some of it, immediately" — and the reasons why queueing instead is
actively worse are what this file is about.

## What to learn
### Queueing is the trap
When demand exceeds capacity, the two options are to queue the excess or
reject it. Queueing feels kinder and is the one that fails.

The queue grows, so latency grows with it. By the time a queued request
reaches the front, the client has already timed out and — worse — retried
(`06-proxy/05-retry.md`), which added *more* load. You are now spending your
remaining capacity computing answers that nobody is listening for, which
reduces effective capacity, which lengthens the queue. That loop is
self-sustaining: throughput of *useful* work collapses toward zero while
the system stays completely busy.

The recovery property matters too. A shedding system returns to normal the
moment load drops. A queueing system has to work through a backlog of
requests that are all already dead, so it stays degraded long after the
cause is gone.

### Shed early, shed cheap
A rejected request should cost as little as possible — that is the whole
point (`07-security/09-ddos.md`'s cost asymmetry). So shedding belongs early
in the pipeline (`09-architecture/01-components.md`): before auth, before
WAF body inspection, before the upstream call.

```rust
// at the front of the pipeline, before any expensive stage
if inflight.load(Ordering::Relaxed) > shed_threshold {
    return Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::RETRY_AFTER, "1")
        .body(Body::empty());
}
```

Gotcha: a 503 that costs a database lookup, a full structured log line
with request context, and a rendered error page gives an attacker a
*better* cost ratio than being served. Keep the shed path allocation-free
where you can, and sample the logging (`08-observability/01-logging.md`)
rather than writing a line per rejection.

Gotcha: make sure your own retry logic does not retry shed responses.
Retrying a 503 that means "I am overloaded" is precisely the amplification
the shedding exists to prevent.

### Bound by time, not by count
A fixed queue depth is the wrong limit, because the right depth depends on
how fast you're draining it — 100 queued requests is fine at 1ms each and
catastrophic at 500ms each.

The better rule, from CoDel, is to bound by *sojourn time*: drop requests
that have already waited longer than a target. That adapts automatically
as service time changes, and it directly expresses the thing you actually
care about — a request that has waited 5 seconds is worthless whether it's
first in line or hundredth.

Gotcha: measure the wait from when the request *arrived*, not from when
you started working on it. The whole point is to notice time spent
waiting, and a timer started at dequeue sees none of it.

### Shed the right requests
Once you accept that something must be dropped, which something is a
design decision:
- **By priority.** Health checks (`06-proxy/03-healthcheck.md`) and critical
  paths survive; bulk or batch traffic is dropped first. This requires a
  priority to exist on the request, which means classifying it at the
  edge — by route, by client tier, by an explicit header from trusted
  callers.
- **By cost.** Expensive routes shed sooner, so one costly endpoint
  can't consume the capacity of everything else
  (`07-security/09-ddos.md`'s expensive-endpoint section). Per-route
  concurrency caps are the simplest form of this.
- **Randomly.** The default, and fine when you have no better signal —
  but it means your most important traffic is dropped at the same rate as
  everything else.

Gotcha: never shed health checks. A proxy that sheds the very probes that
determine whether it is healthy gets marked down by its own load balancer
and removed from rotation — which moves its traffic onto the remaining
instances, pushing them into shedding too. That is how shedding, badly
scoped, takes down a whole fleet.

### Adaptive limits beat static ones
A hard-coded concurrency limit is a guess that ages badly: it's wrong
after a hardware change, a dependency slowdown, or a code change that
alters cost per request.

Adaptive concurrency limiting (Netflix's `concurrency-limits`, Envoy's
adaptive concurrency filter) treats it as a congestion-control problem —
observe latency, increase the limit while latency stays flat, decrease it
when latency rises, essentially TCP's AIMD applied to request admission.
The limit tracks real capacity without anyone tuning it.

Gotcha: adaptive limiting needs a stable latency signal to work from, so
it behaves badly when latency is naturally bimodal (cache hits at 1ms,
misses at 200ms — `05-http-stack/07-cache.md`). Apply it per route, or per
class of work with similar cost, rather than globally.

### Telling the client the truth
`503` with `Retry-After` is the correct shed response: it says "not now,
try again in N seconds" rather than "this failed." A well-behaved client
then backs off instead of retrying immediately.

Gotcha: distinguish shed 503s from upstream-failure 503s in your metrics
(`08-observability/02-metrics.md`). They have completely different causes and
remedies, and a single `status="503"` counter hides which one is
happening. This distinction also matters for your SLI — a 503 you emitted
because you were overloaded is your failure, and belongs in the error
budget (`08-observability/06-alerting.md`).

## Practice
Build these in order.

1. Add a concurrency counter and a static shed threshold at the front of
   `proxy`'s pipeline, returning 503 with `Retry-After`. **Done when**
   load beyond the threshold is rejected immediately rather than queueing.
2. Prove the queueing failure first, so the fix has a baseline. **Done
   when** you can show, with an unbounded queue, that p99 latency climbs
   without limit and useful throughput falls while the process stays 100%
   busy.
3. Measure the shed path's cost. **Done when** a rejected request is
   measurably cheaper than a served one — if it isn't, find what's
   allocating.
4. Replace the depth bound with a sojourn-time bound measured from
   arrival. **Done when** the threshold adapts correctly across two
   workloads with very different service times, with no retuning.
5. Add per-route concurrency caps. **Done when** saturating one expensive
   route leaves other routes serving normally.
6. Exempt health checks and add priority classes. **Done when** the proxy
   under heavy shedding still answers its own health probes and still
   passes high-priority traffic.
7. Stop retrying shed responses. **Done when** a load test at 3x capacity
   shows upstream request count staying flat rather than multiplying.
8. Separate shed 503s from upstream 503s in metrics, and attribute them
   correctly in your SLI. **Done when** a dashboard distinguishes "we are
   overloaded" from "the upstream is broken."
9. (Stretch) Replace the static limit with an adaptive one and compare.
   **Done when** the adaptive limit finds a threshold close to your
   measured capacity without being told it, and re-finds it after you
   inject a 3x upstream slowdown.
