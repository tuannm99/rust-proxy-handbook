# Load Testing

## What to learn
### Tools: wrk, vegeta, k6
`wrk` is a small C tool good for raw HTTP/1.1 throughput with Lua scripting
for custom request generation. `vegeta` (Go) reports latency as a proper
histogram and is easier to script from CI (`echo "GET http://..." | vegeta
attack -rate=500 | vegeta report`). `k6` is heavier but lets you script
realistic multi-step user flows in JS. For this handbook, start with `wrk`
or `vegeta` against `labs/05-reverse-proxy` — you don't need
scripted flows to find basic bottlenecks.

### Throughput vs latency percentiles
Requests/sec alone hides tail latency — a proxy can hit high RPS while p99
is terrible because a few slow upstream calls queue up behind fast ones.
Always report p50/p90/p99/p999, not just mean or RPS. A load balancer bug
(e.g. one upstream getting 10x traffic) often shows up as a fat p99 tail
long before it shows up in the average.

### The load generator is not free
`wrk`/`vegeta` running on the same machine as the proxy competes for CPU
and can bottleneck *itself* — the numbers then measure the generator, not
your proxy. Watch the generator's own CPU/network usage, and prefer running
it on a separate machine (or at least a separate CPU affinity) for numbers
you trust. A generator with too few connections/threads will also
under-drive the proxy and report artificially "good" latency.

### Closed-loop vs open-loop load
Most simple tools (default `wrk`) are closed-loop: they wait for a response
before sending the next request, which means a slow server automatically
throttles the offered load and hides the real backlog. `vegeta attack
-rate=N` and `wrk2` are open-loop: they send at a fixed rate regardless of
response time, which is what real traffic (and real incidents) look like.
Prefer open-loop when you want to know "what happens at 500 req/s" rather
than "how fast can this go end to end."

## Practice
1. Run `wrk -t4 -c100 -d30s` against `labs/02-http-server` serving a
   static file; record RPS and p50/p99.
2. Run the same test against `labs/05-reverse-proxy` with 2
   upstreams and compare — the proxy hop should add latency, quantify how
   much.
3. Switch to `vegeta attack -rate=200` (open-loop) against the same target
   and compare p99 to the closed-loop `wrk` run at similar throughput.
4. Introduce an artificial slow path in one upstream (e.g. `sleep` before
   responding to 10% of requests) and confirm it shows up in p99 well
   before it moves the mean — cross-reference `08-observability/02-metrics.md`
   for how you'd alert on this in production.
5. Watch `top`/`htop` on the load generator itself during a high-rate run
   and confirm it isn't CPU-saturated (which would invalidate the results).
