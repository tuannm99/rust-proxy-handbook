# 16-opentelemetry

## Goal

Export `tracing` spans as OpenTelemetry traces so a single trace ID
connects spans across a chain of proxied requests.

## Done when

- [ ] Spans for the request lifecycle (accept, route, upstream call, response) appear in a local trace viewer such as Jaeger.
- [ ] The trace context (`traceparent`) is propagated to the upstream: a chain of two proxy instances shows up as one trace.
- [ ] Span timings are correct across `.await` points — no span entered with a guard held across an await ([`instruction/08-observability/03-tracing.md`](../../instruction/08-observability/03-tracing.md)).
- [ ] Sampling is configurable, and at a low sampling rate export overhead is not measurable in a load test.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/08-observability/03-tracing.md`](../../instruction/08-observability/03-tracing.md)
- [`instruction/04-runtime/04-structured-concurrency.md`](../../instruction/04-runtime/04-structured-concurrency.md) — `task_local!` for per-request context

## Run

```
cargo run -p opentelemetry-lab
```
