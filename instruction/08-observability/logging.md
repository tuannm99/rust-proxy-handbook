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

Gotcha: the useful test for `ERROR` is "would I want to be paged for this?"
A proxy that logs every failed upstream connection at `ERROR` produces
thousands of them during a routine upstream restart, and the level stops
carrying information — which means the one genuinely novel error is
invisible. Failures the proxy *handled* (a retry that succeeded, a circuit
that opened as designed) are `WARN` at most; the aggregate rate belongs in
metrics (`08-observability/metrics.md`), not in a log line per occurrence.

### Redact by allowlist, not denylist
"Don't log the `Authorization` header" is a rule you will violate by
accident, because secrets arrive in places you didn't enumerate: `Cookie`,
`Proxy-Authorization`, a token in a query string (`?api_key=...`), a
custom `X-Api-Key`, a signed URL, a JWT in a path segment.

Invert it: log an explicit *allowlist* of headers and query parameters,
and drop everything else. The list is short (`content-type`,
`user-agent`, `content-length`, your own request ID) and the failure mode
becomes "a field is missing from logs" rather than "a credential is in
the log aggregator forever."

Gotcha: `#[derive(Debug)]` on a config or request struct will happily
print a secret field when someone logs it with `{:?}` during debugging.
Wrap secrets in a newtype whose `Debug`/`Display` prints `[redacted]`, so
the compiler's default is safe rather than leaky. `secrecy`'s `Secret<T>`
does this.

Gotcha: logs are subject to data-protection rules (GDPR and equivalents)
the moment they contain an IP address or user identifier. That makes
retention a compliance decision, not just a cost one, and "we keep all
logs for two years" may be a liability rather than an asset.

### Log injection: fields are attacker-controlled
The path, the `User-Agent`, the `Referer`, and any header you log are
written by whoever sent the request. If your formatter emits plain text, a
`User-Agent` containing `\n` lets an attacker inject a *whole fake log
line* — forging an entry that looks like a successful admin login, or
splitting one event into two to break a parser.

Structured JSON logging handles this correctly by construction, since the
serializer escapes control characters inside string values — which is one
of the better arguments for JSON over a hand-rolled text format. If you
do emit text, escape or strip control characters from every
attacker-controlled field, and cap field lengths (a 1 MB `User-Agent` is a
cheap way to fill your disk).

Gotcha: the same applies downstream. A log line that is valid JSON can
still carry a payload that attacks whatever *reads* it — a dashboard that
renders log fields as HTML has an XSS hole fed by your proxy's traffic.

### Correlation IDs / request IDs
A reverse proxy is often the first hop, so it should mint a request ID if the client didn't send one (`X-Request-Id`), thread it through every log line via a `tracing::Span`, and forward it to the upstream so logs across services can be joined on it. This is the low-tech precursor to full distributed tracing (see `08-observability/tracing.md`).

```rust
let span = tracing::info_span!("request", request_id = %request_id);
let _enter = span.enter(); // all logs inside inherit request_id
```

Gotcha: the snippet above is correct in a synchronous function and
**wrong in an async one** — holding an `Entered` guard across an `.await`
attaches the span to whatever task the executor runs next. Use
`.instrument(span)` on the future instead; `08-observability/tracing.md`
covers why in detail. This is the most common instrumentation bug in
async Rust and it silently corrupts the correlation you built the ID for.

Gotcha: a client-supplied `X-Request-Id` is untrusted input. Validate its
length and character set (a UUID-shaped string, say) before accepting it,
or you've handed attackers a channel into every log line — and into any
system that indexes on it.

### Log volume as a cost, not a free side effect
At 10k req/s, one log line per request is 10k lines/s to ship, parse, index, and store. Disk I/O and log-shipper backpressure can itself become the bottleneck that slows down the proxy (a classic self-inflicted incident). Plan retention, sampling (e.g. log 100% of errors, 1% of 200s), and async/non-blocking writers (`tracing-appender`'s `non_blocking`) from day one.

Gotcha: a synchronous write to a log file is a blocking syscall on the
request path. When the disk is slow — or the log volume itself has filled
the page cache with dirty pages (`16-kernel/page-cache.md`) — that write
blocks a tokio worker thread and stalls every connection multiplexed on
it. `tracing_appender::non_blocking` moves writes to a dedicated thread
behind a bounded queue; that queue is a ring buffer
(`13-algorithms/ring-buffer.md`), and you must know what it does when
full. Dropping log lines under pressure is the right default for a proxy —
but only if you *count* the drops, or you'll trust an incomplete log
without knowing it.

### Sampling that preserves signal
Naive sampling ("log 1% of requests") is worse than it looks in a
distributed system: each hop samples independently, so a request logged at
the proxy is probably *not* logged at the upstream, and you can never
reconstruct a full path.

Sample **deterministically on the request ID or trace ID** — hash it and
keep the request if the hash falls below your rate — so every hop makes
the same decision and a sampled request is logged everywhere. Then layer
the rules that actually matter: keep 100% of 5xx, 100% of slow requests
(over some latency threshold), and a small deterministic fraction of the
rest.

## Practice
Build these in order.

1. In `proxy`, wire `tracing` + `tracing-subscriber` with a JSON
   formatter configurable via `RUST_LOG`. **Done when** one request emits
   one parseable JSON line containing method, path, status, and latency.
2. Add a per-request span carrying `request_id`, attached with
   `.instrument()` rather than `enter()`. **Done when** a load test with
   concurrent requests shows every log line carrying the *correct* request
   ID — build the `enter()`-across-`await` version first and watch IDs
   cross-contaminate under concurrency.
3. Accept or mint `X-Request-Id`, validating client-supplied values.
   **Done when** a 10 KB or newline-containing `X-Request-Id` is rejected
   or replaced, and a valid one is forwarded upstream unchanged.
4. Switch to allowlist-based field logging and wrap config secrets in a
   redacting newtype. **Done when** a request with `Authorization`,
   `Cookie`, and `?api_key=` produces log lines containing none of them,
   and `{:?}` on your config prints `[redacted]` for the secret fields.
5. Attempt log injection: send a `User-Agent` containing newlines and
   forged JSON. **Done when** the output is still exactly one well-formed
   log record per request, with the payload contained in a string field.
6. Switch to `tracing_appender::non_blocking` and count dropped lines.
   **Done when** a load test against a deliberately slow writer shows
   request latency unaffected and a nonzero, *visible* drop counter.
7. Measure logging's own cost. **Done when** you have p50/p99 request
   latency with logging fully on, sampled, and off, under
   `12-testing/load-testing.md` — the gap is your logging budget.
8. Add deterministic sampling plus always-log rules for 5xx and slow
   requests. **Done when** the same request is either logged at every hop
   or none, and every error in a load test appears in full.
