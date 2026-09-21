# Observability

Phase 8. Answering "what is the proxy doing right now" and "why was that
request slow" — for a component that sits between everything, where the
usual answer is "it's the other side's fault" and you need data to say so.

## Files

- `01-logging.md` — structured events, redaction, log injection, sampling that preserves signal
- `02-metrics.md` — counters/gauges/histograms, RED for a proxy, cardinality, why percentiles can't be averaged
- `03-tracing.md` — spans, `traceparent` propagation, sampling, and the `enter()`-across-`await` bug
- `04-profiling.md` — perf and flamegraphs, why they lie about async causality, `tokio-console`, off-CPU time
- `05-slo.md` — defining SLIs precisely, targets and windows, error budgets
- `06-alerting.md` — symptom alerting, burn-rate ladders, alerting on absence of data, routing

## Reading order

`01-logging.md` → `02-metrics.md` → `03-tracing.md` builds the three signals in
increasing cost order, and they share instrumentation (the request span
you add for logging becomes the trace span). `05-slo.md` before
`06-alerting.md`, always — an alert threshold without an SLO behind it is a
guess. `04-profiling.md` is for when something is already slow and you need
to find out where.

These back `labs/15-prometheus` and `labs/16-opentelemetry`. The thing to
carry into `proxy/`: measure latency at both ends, because the difference
between client-observed and upstream-observed time is the only number that
says whether a slowdown is yours.
