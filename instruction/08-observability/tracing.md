# Distributed Tracing
OpenTelemetry spans/traces vs metrics vs logs.

## What to learn
### The three pillars, and what each is actually for
**Logs** answer "what happened" for one event. **Metrics** answer "what's the aggregate trend" cheaply at scale. **Traces** answer "where did *this specific request* spend its time across every hop it touched" — a tree of spans (proxy → auth service → upstream A → upstream B) with timing for each. A proxy is usually the entry point of a trace, so it's responsible for either starting a new trace or continuing one the client already started.

### Trace context propagation (W3C `traceparent`)
Distributed tracing only works if every hop passes the same trace ID forward. The W3C Trace Context spec defines a `traceparent` header: `00-<32 hex trace-id>-<16 hex parent-span-id>-<flags>`. If the incoming request has one, the proxy creates a *child* span under that trace ID and forwards a new `traceparent` (with its own span ID as parent) to the upstream. If not, it mints a new trace ID.

```rust
// pseudo: extracting/propagating W3C traceparent
fn child_traceparent(incoming: Option<&str>, new_span_id: &str) -> String {
    match incoming {
        Some(tp) => {
            let parts: Vec<&str> = tp.split('-').collect(); // [version, trace_id, parent_id, flags]
            format!("00-{}-{}-{}", parts[1], new_span_id, parts[3])
        }
        None => format!("00-{}-{}-01", generate_trace_id(), new_span_id),
    }
}
```
Gotcha: if any single hop in the chain drops or fails to forward `traceparent`, the trace fragments into disconnected pieces and you lose the ability to see the full request path — this is the most common distributed tracing bug in practice.

Gotcha: the snippet above indexes `parts[1]` and `parts[3]` on
attacker-supplied input — a malformed `traceparent` panics the request
task. Validate strictly: exactly four dash-separated fields, correct
lengths, hex-only, and an all-zero trace ID or span ID is invalid per
spec. Reject-and-mint-new is the right response to anything malformed,
never "use it anyway."

Gotcha: `tracestate` (the companion header carrying vendor-specific
key-value data) must be forwarded too, and it has its own size limits.
Dropping it doesn't break the trace tree but does lose sampling decisions
and vendor context that downstream systems rely on.

### The async Rust bug that breaks everything: `enter()` across `.await`
This is the Rust-specific failure mode, and it silently produces traces
that are *wrong* rather than missing — which is worse.

```rust
// WRONG in async code
let span = tracing::info_span!("request", request_id = %id);
let _guard = span.enter();        // sets the current span for THIS THREAD
upstream.send(req).await;          // task yields; another task resumes on this thread
                                   // ...and inherits our span
```

`Span::enter()` returns a guard that sets the thread-local current span.
When the future yields at an `.await`, the guard is still held, so whatever
task the executor schedules next on that thread logs and spans *inside
your request's span*. You get requests nested under unrelated requests,
attributes attached to the wrong trace, and correlation IDs that point at
someone else's request.

The fix is to attach the span to the *future* rather than the thread:

```rust
use tracing::Instrument;
async move { upstream.send(req).await }
    .instrument(tracing::info_span!("request", request_id = %id))
    .await;
```

`#[tracing::instrument]` on an async fn does the right thing
automatically. The rule: in async code, never hold an `Entered` guard
across an `.await`; `clippy::await_holding_span_guard` catches it, so turn
it on.

### What to put on a proxy's spans
A span that records only a name and a duration tells you a request was
slow, which you already knew from metrics. The value is in the attributes
that explain *why*, and for a proxy the interesting ones are all about
decisions it made:

- which upstream was selected, and by which algorithm
  (`06-proxy/load-balancer.md`)
- how long connection acquisition took vs the upstream's own response time
  (`06-proxy/upstream.md`) — these are frequently confused in incident
  reviews and the split settles it
- retry count and whether a circuit was open (`06-proxy/retry.md`)
- cache hit/miss/stale (`05-http-stack/cache.md`)
- whether the request was rate-limited or shed, and by which rule
- time spent in WAF inspection (`07-security/waf.md`)

Child spans for the phases (TLS handshake, WAF, upstream call) make the
waterfall self-explanatory — someone reading the trace should be able to
see where the time went without knowing your code.

Gotcha: span attributes are subject to the same rules as log fields
(`08-observability/logging.md`) — no credentials, no raw bodies, no
unbounded attacker-controlled strings. Traces are usually more widely
readable than logs, not less.

### Sampling
Tracing every request at high QPS is expensive to store and mostly redundant (thousands of identical fast 200s tell you nothing new). Head-based sampling decides at the entry point (e.g. 1% of requests, or all requests with `traceparent` flag already set to sampled by an upstream client). Tail-based sampling defers the decision until the trace finishes, so you can keep 100% of *slow or erroring* traces and drop most fast ones — better signal, but requires buffering spans before deciding.

The decision must propagate, and it must be consistent. The sampled flag
in `traceparent`'s last byte is how downstream hops learn what was already
decided — a hop that re-decides independently produces traces with holes
in them, since some spans of one trace are kept and others dropped. Honor
the incoming flag; only decide when you're the entry point.

Gotcha: tail sampling can't be done in the proxy alone. Deciding "keep
this trace because it was slow" requires seeing *all* spans of the trace,
which only a collector positioned after every service can do. The proxy's
part is to export everything (or a generous head sample) and let the
collector make the real decision — a design choice with real cost
implications, so make it deliberately.

Gotcha: an attacker who controls `traceparent` controls your sampling
decision, and can force 100% sampling by setting the sampled flag on every
request — turning your tracing pipeline into an amplification target
(`07-security/ddos.md`). Rate-limit the honoring of client-set sampling
from untrusted clients, or ignore the flag except from trusted peers
(`07-security/ip-filtering.md`'s trust boundary again).

### Don't let telemetry take down the proxy
The exporter is a network client in your request path's shadow, and it
fails like one. Three properties to verify rather than assume:
- **The export must be asynchronous and bounded.** A batch exporter with
  an unbounded queue turns a collector outage into an OOM; a synchronous
  export turns collector latency into request latency.
- **Dropping spans must be counted.** A queue that silently discards under
  pressure gives you a trace pipeline that is quietly lying, usually
  during exactly the incident you're trying to investigate.
- **Shutdown must flush.** Spans buffered when the process exits are lost
  unless the shutdown path drains them — tie this into
  `09-architecture/graceful-shutdown.md` rather than leaving it to a
  destructor that may not run.

### Spans vs tracing crate `Span`s
Confusingly, Rust's `tracing` crate calls its structured logging scopes "spans" too — and they compose well with OpenTelemetry: `tracing-opentelemetry` bridges `tracing::Span`s into OTel spans that get exported to a collector (Jaeger/Tempo/Honeycomb), so the same instrumentation you added for `08-observability/logging.md` doubles as trace data.

## Practice
Build these in order.

1. In `labs/16-opentelemetry`, get `tracing-opentelemetry` plus an OTLP
   exporter emitting one span to a local collector (Jaeger is enough).
   **Done when** the span appears in the UI.
2. Reproduce the `enter()`-across-`.await` bug on purpose in `proxy`:
   instrument a request handler with a held guard and run concurrent
   requests. **Done when** you can see spans nested under the wrong
   parent — then switch to `.instrument()` / `#[instrument]`, enable
   `clippy::await_holding_span_guard`, and confirm the tree is correct.
3. Extract and propagate `traceparent` with strict validation. **Done
   when** a malformed header (wrong field count, non-hex, all-zero trace
   ID) is rejected and replaced rather than panicking or propagating, and
   a valid one produces a correct parent/child relationship.
4. Run the proxy in front of two chained instances of
   `labs/02-http-server` and verify end-to-end. **Done when** one trace ID
   connects spans from all three processes in the UI.
5. Add proxy-decision attributes and phase child spans. **Done when** a
   single trace shows connection-acquisition time separately from upstream
   response time, plus upstream name, retry count, and cache outcome.
6. Add head-based sampling that honors the incoming sampled flag. **Done
   when** the decision propagates through all three hops — verify that
   traces are never partially sampled — and an untrusted client cannot
   force 100% sampling.
7. Break propagation deliberately on one hop. **Done when** you've seen
   the fragmented trace in the UI, since that is what the bug looks like
   in production.
8. Kill the collector mid-load-test. **Done when** the proxy keeps serving
   with unchanged latency, memory stays flat, and a dropped-span counter
   rises — then restart it and confirm export resumes.
9. Wire exporter flush into shutdown. **Done when** spans from requests
   completed just before `SIGTERM` still arrive at the collector.
