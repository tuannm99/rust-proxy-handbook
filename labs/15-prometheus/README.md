# 15-prometheus

## Goal

Export request count, latency histogram, and upstream error count as
Prometheus metrics on a `/metrics` endpoint.

## Done when

- [ ] `/metrics` exposes a request counter labeled by route and status class, a latency histogram, and an upstream error counter.
- [ ] `promtool check metrics` passes on the output, and a local Prometheus scrapes it.
- [ ] Label cardinality is bounded: no raw path, user ID, or client IP as a label value ([`instruction/08-observability/02-metrics.md`](../../instruction/08-observability/02-metrics.md)).
- [ ] Histogram buckets were chosen for your latency range, and `histogram_quantile(0.99, ...)` roughly matches the p99 your load-test tool reports.
- [ ] Recording a metric on the request path doesn't take a global lock (checked with a benchmark or by reading the library's code).
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/08-observability/02-metrics.md`](../../instruction/08-observability/02-metrics.md)
- [`instruction/08-observability/05-slo.md`](../../instruction/08-observability/05-slo.md) — what these metrics are ultimately for
- [`instruction/12-testing/06-lab-environment.md`](../../instruction/12-testing/06-lab-environment.md) — running Prometheus and `promtool` locally

## Run

```
cargo run -p prometheus-lab
```
