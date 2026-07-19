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

### The RED method for a proxy
For every hop (client-facing and per-upstream) track: **R**ate (requests/sec), **E**rrors (rate of non-2xx or connection failures), **D**uration (latency histogram). This is the minimum dashboard to answer "is the proxy healthy" and "is upstream X healthy" without guessing. Pair with USE (Utilization/Saturation/Errors) for the machine itself (CPU, fd count, conn pool saturation).

### Cardinality explosion
Every unique combination of label values creates a new time series. A label like `path` on a proxy that forwards arbitrary user-supplied paths (`/users/12345`, `/users/12346`, ...) can create unbounded cardinality and take down your metrics backend (Prometheus OOMs, or your bill explodes). Normalize dynamic path segments into route templates (`/users/:id`) before using them as a label, and never put raw user input, request IDs, or IPs in a label.
Gotcha: this is the single most common way a "just add a metric" PR turns into a 3am page for the observability team.

### Push vs pull, and where OpenTelemetry fits
Prometheus pulls (scrapes `/metrics` on an interval); OTel metrics can push to a collector, which then exports to Prometheus/Datadog/etc. A proxy typically exposes a `/metrics` endpoint for scraping — simple, no extra network dependency, and it survives the proxy being temporarily unreachable from the collector (data is just missed, not queued and lost).

## Practice
1. In `proxy`, expose a `/metrics` endpoint (`prometheus` crate's `TextEncoder`) alongside the proxy's normal listener.
2. Add RED metrics for the client-facing side: `proxy_requests_total{status}`, `proxy_request_duration_seconds` histogram.
3. Add per-upstream RED metrics keyed by upstream *name* (from `06-proxy/upstream.md`), not by raw address/path, to avoid cardinality blowup.
4. Add a gauge for currently-healthy upstream count, updated from the health checker in `06-proxy/healthcheck.md`.
5. Deliberately add a high-cardinality label (raw path) to one metric, scrape it under `12-testing/load-testing.md` traffic, and observe the series count grow — then fix it with route templating.
