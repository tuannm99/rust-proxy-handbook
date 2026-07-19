# Chaos Testing

## What to learn
### Fault injection: latency, packet loss, upstream failures
Chaos testing means deliberately breaking things your proxy depends on
while it's under load, instead of only testing the happy path. The three
faults worth starting with: added latency (does a slow upstream cause
unbounded queueing?), packet loss/connection resets (does the client-facing
side degrade gracefully?), and full upstream failures (does the proxy
actually fail over?). These map directly to the failure modes
`06-proxy/healthcheck.md` and `06-proxy/retry.md` are meant to handle — this
is where you find out if that code actually works.

### toxiproxy
`toxiproxy` (Shopify) sits between your proxy and its upstreams as a
programmable TCP proxy: you can inject latency, bandwidth limits, and
connection resets on a live connection via its HTTP API, and toggle them
mid-test. It's the easiest way to test `06-proxy/retry.md`'s circuit
breaker without touching kernel-level tools — point
`milestones/03-reverse-proxy`'s upstream config at a toxiproxy instance
instead of the real upstream.

### tc netem
`tc qdisc add dev lo root netem delay 200ms loss 5%` injects latency/loss at
the kernel network-interface level — more realistic than toxiproxy (real
packet loss, not just connection resets) but coarser-grained (affects all
traffic on that interface, harder to toggle per-connection mid-test). Good
for a final "does this survive a bad network" pass once toxiproxy-driven
unit-level chaos tests already pass.

### Verifying retry/circuit-breaker behavior under real faults
The point of chaos testing here isn't finding new bugs blindly — it's
confirming a specific claim: "the circuit breaker opens after N failures
and the proxy stops sending traffic to a dead upstream." Write the chaos
test as an assertion against that claim (e.g. "after killing upstream A,
95%+ of requests within 2s are served by upstream B"), not just "run chaos
and see what happens."

## Practice
1. Put `milestones/03-reverse-proxy` in front of two upstreams routed
   through `toxiproxy`; inject 500ms latency on one and confirm your load
   balancer/health check notices and shifts traffic (or at minimum that p99
   reflects it if you haven't built adaptive routing yet).
2. Use toxiproxy to fully cut one upstream's connection mid-load-test and
   assert client-visible error rate stays near 0% (retries + circuit
   breaker absorb it) rather than spiking to ~50%.
3. Use `tc netem` to add 5% packet loss on `lo` and re-run the same
   load test; compare error rate/latency to the toxiproxy-only run.
4. Kill and restart an upstream process repeatedly during a sustained load
   test ("flapping") and confirm `06-proxy/healthcheck.md`'s hysteresis
   prevents the proxy from thrashing its rotation decision every second.
5. Send `SIGTERM` to `proxy` mid-chaos-test and
   confirm graceful shutdown (`09-architecture/graceful-shutdown.md`) still
   drains in-flight requests correctly even while upstreams are unhealthy.
