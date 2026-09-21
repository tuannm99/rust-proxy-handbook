# Metrics
Prometheus/OpenTelemetry.

## What to learn
### Counter, gauge, histogram
A **counter** only goes up (total requests, total errors) — useful with `rate()` in PromQL to get requests/sec. A **gauge** can go up or down (open connections, upstreams currently healthy). A **histogram** buckets observations (request duration) so you can derive p50/p95/p99 server-side without shipping raw samples. Proxies live and die by histograms — averages hide the slow 1% that a load balancer's users actually feel.

```rust
use prometheus::{register_histogram_vec, HistogramVec};

static REQUEST_DURATION: once_cell::sync::Lazy<HistogramVec> = once_cell::sync::Lazy::new(|| {
    register_histogram_vec!(
        "proxy_request_duration_seconds",
        "Request duration",
        &["method", "status"]
    ).unwrap()
});
```

Gotcha: counters reset to zero when the process restarts, which is why you
almost never read a counter's raw value — `rate()` and `increase()`
understand resets and compute across them. A dashboard panel showing a raw
`_total` is showing "time since last deploy" as much as anything real.

### Percentiles do not average — this is the one to get right
A histogram gives you percentiles *because* the buckets are additive. Ten
proxy instances each export bucket counts; you sum the buckets across
instances and then compute the quantile:

```promql
histogram_quantile(0.99, sum by (le) (rate(proxy_request_duration_seconds_bucket[5m])))
```

What you must **not** do is compute p99 per instance and average those
numbers. The average of ten p99s is not the fleet's p99 and has no
statistical meaning at all — it is systematically wrong, usually
optimistic, and it looks entirely plausible on a dashboard. The same
applies across time: you cannot average a p99 over an hour to get the
hour's p99.

Gotcha: this is exactly why Prometheus's **Summary** type is a trap for a
multi-instance service. A Summary computes quantiles *client-side*, inside
each process, so what it exports is already-collapsed numbers that cannot
be re-aggregated across instances. Use histograms for anything you'll have
more than one copy of — which, for a proxy, is everything.

### Bucket boundaries are a design decision
A histogram's resolution is entirely determined by its buckets, chosen in
advance. Prometheus's default buckets top out around 10 seconds and are
spaced for generic web work — if your proxy's p99 is 3ms, every request
lands in the first bucket and `histogram_quantile` interpolates within it,
producing a confident number that is essentially made up.

Choose buckets around the latencies you actually care about, and put a
boundary exactly at your SLO threshold (`08-observability/06-alerting.md`):
with a bucket edge at 250ms, "fraction of requests under 250ms" becomes an
exact count rather than an interpolation.

Gotcha: buckets cost cardinality. A histogram with 20 buckets and 3 labels
of 10 values each is 20 × 1000 = 20,000 series from one metric. Native
(exponential) histograms in newer Prometheus versions avoid the tradeoff,
but until you're on those, "add more buckets" is not free.

### The RED method for a proxy
For every hop (client-facing and per-upstream) track: **R**ate (requests/sec), **E**rrors (rate of non-2xx or connection failures), **D**uration (latency histogram). This is the minimum dashboard to answer "is the proxy healthy" and "is upstream X healthy" without guessing. Pair with USE (Utilization/Saturation/Errors) for the machine itself (CPU, fd count, conn pool saturation).

For a proxy specifically, measure duration at **both** ends and export the
difference. Total client-observed latency minus upstream response time is
the proxy's own overhead — queueing, TLS, WAF inspection
(`07-security/06-waf.md`), connection acquisition from the pool. Without that
split, every latency investigation starts with "is it us or them?" and no
data to answer it.

The proxy-specific signals worth having from day one, none of which are in
a generic RED dashboard:
- **Connection pool saturation** per upstream (`06-proxy/01-upstream.md`) —
  waiting for a pooled connection is invisible in upstream response time.
- **Retry and circuit-breaker state** (`06-proxy/05-retry.md`) — retry rate,
  budget exhaustion, circuit transitions.
- **Healthy upstream count** as a gauge (`06-proxy/03-healthcheck.md`).
- **Cache hit ratio** (`05-http-stack/07-cache.md`), which explains upstream
  load changes that have nothing to do with client traffic.
- **Queue depth / shed count** (`07-security/09-ddos.md`), the earliest
  signal of overload.

### Cardinality explosion
Every unique combination of label values creates a new time series. A label like `path` on a proxy that forwards arbitrary user-supplied paths (`/users/12345`, `/users/12346`, ...) can create unbounded cardinality and take down your metrics backend (Prometheus OOMs, or your bill explodes). Normalize dynamic path segments into route templates (`/users/:id`) before using them as a label, and never put raw user input, request IDs, or IPs in a label.
Gotcha: this is the single most common way a "just add a metric" PR turns into a 3am page for the observability team.

Gotcha: cardinality is *multiplicative*. Four labels with 10, 20, 5, and
50 values is 50,000 series per metric — each of which costs memory in the
proxy as well as in the backend. Before adding a label, multiply it out;
and treat "labels come from the route table" (bounded, known at config
load) as the rule, "labels come from the request" as the exception that
needs justification.

Gotcha: status code as a label is fine (bounded), but status code *text*
or an error message string is not. `status="500"` is one series;
`error="connection refused to 10.0.0.7:8080"` is one per upstream, per
port, per phrasing.

### Metrics cost something on the hot path
Every metric update happens on every request. A counter is an atomic
increment, which is cheap but not free — a single global counter touched
by 16 worker threads is a contended cache line
(`17-performance/02-false-sharing.md`), and at high request rates that shows
up in a profile.

The bigger cost is usually the *label lookup*: `with_label_values(&["GET",
"200"])` hashes the label strings to find the right child metric on every
call. Resolve label sets once (per route, at config load, or cached per
connection) and hold the resulting handle, rather than looking it up per
request.

Gotcha: measure this rather than assuming. Metrics overhead is usually
small enough to ignore and occasionally 5% of CPU — and you cannot tell
which without a flamegraph (`08-observability/04-profiling.md`).

### Push vs pull, and where OpenTelemetry fits
Prometheus pulls (scrapes `/metrics` on an interval); OTel metrics can push to a collector, which then exports to Prometheus/Datadog/etc. A proxy typically exposes a `/metrics` endpoint for scraping — simple, no extra network dependency, and it survives the proxy being temporarily unreachable from the collector (data is just missed, not queued and lost).

Gotcha: expose `/metrics` on a **separate listener** from production
traffic, bound to an internal interface. On the main listener it's
reachable by anyone, and it leaks a detailed map of your upstreams, route
names, and traffic volumes — plus scraping it becomes a cheap way to
consume proxy CPU (`07-security/09-ddos.md`). It also means metrics stay
scrapeable when the main listener is saturated or shedding, which is
exactly when you need them.

Gotcha: rendering `/metrics` is O(number of series). At high cardinality
this becomes a slow, allocation-heavy response — and a scrape interval
shorter than the render time means the proxy is permanently busy
serializing metrics. Watch scrape duration as its own metric.

## Practice
Build these in order.

1. In `labs/15-prometheus`, expose `/metrics` with the `prometheus`
   crate's `TextEncoder` and scrape it by hand. **Done when** `curl`
   returns a valid exposition-format response.
2. Add client-side RED metrics to `proxy` on a separate internal listener.
   **Done when** `/metrics` is unreachable from the public listener and
   still served while the main listener is saturated.
3. Add per-upstream RED keyed by upstream *name*, plus the proxy-specific
   signals (pool saturation, retry rate, healthy count, cache hit ratio,
   shed count). **Done when** you can answer "is this latency ours or the
   upstream's" from the dashboard alone.
4. Choose histogram buckets for your actual latency range with an edge at
   your SLO threshold. **Done when** p50/p99 computed from buckets match
   a direct measurement (compare against latencies recorded by your load
   generator) to within a bucket width — with default buckets first, so
   you see them disagree.
5. Aggregate across instances correctly. **Done when** running three proxy
   instances and computing fleet p99 with `histogram_quantile(sum by (le)
   ...)` gives a defensible number — and you've also computed the average
   of per-instance p99s and can state how far off it is.
6. Deliberately add a raw-path label, run a load test, and watch series
   count grow. **Done when** you've seen it climb into the thousands,
   then fixed it with route templating and confirmed it's bounded by the
   route table size.
7. Pre-resolve label handles on the hot path and profile
   (`08-observability/04-profiling.md`). **Done when** you can state metrics'
   CPU cost as a percentage, before and after.
8. Watch `/metrics` render time as its own metric. **Done when** you know
   how long a scrape takes at your current series count and how that
   compares to your scrape interval.
