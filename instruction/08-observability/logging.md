# Logging

## What to learn
### Structured logging over string logging
Plain `println!`/`log::info!` text is fine for a toy, but a proxy handles thousands of req/s across many connections — you need machine-parseable fields (method, path, status, upstream, latency), not sentences to grep. Use the `tracing` crate with a JSON formatter so every log line is a structured event you can filter/aggregate downstream (Loki, ELK, CloudWatch Insights).

```rust
use tracing_subscriber::fmt;

fn init_logging() {
    fmt()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .init();
}

// per-request:
tracing::info!(method = %req.method(), path = %req.uri().path(), status = 200, latency_ms = 4, "request completed");
```
Gotcha: `tracing`'s JSON layer allocates per field per event — under real load this becomes measurable CPU. Sample or batch logs at high QPS instead of logging every request unconditionally.

### Log levels and what belongs where
`ERROR` = the proxy itself failed to do its job (upstream unreachable, panic caught, config invalid). `WARN` = degraded but handled (retry succeeded on 2nd attempt, circuit breaker opened). `INFO` = one line per request in production, or per notable lifecycle event (config reloaded, listener bound). `DEBUG`/`TRACE` = header dumps, connection pool internals — compiled in but filtered out by default via `RUST_LOG`/`EnvFilter`, since even *evaluating* whether to log has a cost if arguments aren't lazy.
Gotcha: never log full request/response bodies or Authorization headers at INFO — that's how proxies leak credentials into log aggregators that a wider team can read.

### Correlation IDs / request IDs
A reverse proxy is often the first hop, so it should mint a request ID if the client didn't send one (`X-Request-Id`), thread it through every log line via a `tracing::Span`, and forward it to the upstream so logs across services can be joined on it. This is the low-tech precursor to full distributed tracing (see `08-observability/tracing.md`).

```rust
let span = tracing::info_span!("request", request_id = %request_id);
let _enter = span.enter(); // all logs inside inherit request_id
```

### Log volume as a cost, not a free side effect
At 10k req/s, one log line per request is 10k lines/s to ship, parse, index, and store. Disk I/O and log-shipper backpressure can itself become the bottleneck that slows down the proxy (a classic self-inflicted incident). Plan retention, sampling (e.g. log 100% of errors, 1% of 200s), and async/non-blocking writers (`tracing-appender`'s `non_blocking`) from day one.

## Practice
1. In `proxy`, wire up `tracing` + `tracing-subscriber` with a JSON formatter, configurable via `RUST_LOG`.
2. Add a `tracing::Span` per request carrying `request_id`, `method`, `path`; generate a UUID request ID if the client sends none.
3. Log one structured line per completed request with status code and latency; log upstream failures at `WARN` with the retry count.
4. Switch the writer to `tracing_appender::non_blocking` and measure request latency with/without it under a small load test (see `12-testing/load-testing.md`) to see logging's own overhead.
5. Add a sampling rule: always log non-2xx, but only 1-in-N successful requests.
