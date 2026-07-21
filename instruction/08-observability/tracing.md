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

### Sampling
Tracing every request at high QPS is expensive to store and mostly redundant (thousands of identical fast 200s tell you nothing new). Head-based sampling decides at the entry point (e.g. 1% of requests, or all requests with `traceparent` flag already set to sampled by an upstream client). Tail-based sampling defers the decision until the trace finishes, so you can keep 100% of *slow or erroring* traces and drop most fast ones — better signal, but requires buffering spans before deciding.

### Spans vs tracing crate `Span`s
Confusingly, Rust's `tracing` crate calls its structured logging scopes "spans" too — and they compose well with OpenTelemetry: `tracing-opentelemetry` bridges `tracing::Span`s into OTel spans that get exported to a collector (Jaeger/Tempo/Honeycomb), so the same instrumentation you added for `08-observability/logging.md` doubles as trace data.

## Practice
1. In `proxy`, add `tracing-opentelemetry` + an OTLP exporter, and export the request span already created for `08-observability/logging.md`.
2. Implement `traceparent` extraction from the inbound request and propagation to the outbound upstream call.
3. Verify end-to-end: run the proxy in front of two chained instances of `labs/02-http-server`, and confirm a single trace ID connects both spans in your tracing backend (Jaeger locally is enough).
4. Add head-based sampling (fixed percentage) and confirm the sampling decision itself propagates via the `traceparent` flags byte so downstream hops don't re-decide independently.
5. Break propagation on purpose (drop the header on one hop) and observe the trace fragment in the UI — this is what a real propagation bug looks like.
