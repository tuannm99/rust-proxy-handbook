# Queueing Theory

## What to learn

### Little's Law: the one formula that explains everything downstream of it
`L = λW` — the average number of items in a system (`L`) equals the average arrival rate (`λ`) times the average time an item spends in the system (`W`). It holds for *any* stable queueing system regardless of arrival distribution, service distribution, or number of servers, which is exactly why it's the single most load-bearing formula in capacity planning: knowing request rate and target latency tells you exactly how many in-flight requests your proxy must be able to hold.

```text
L = λ * W
example: λ = 1000 req/s, target W = 50ms = 0.05s  =>  L = 50 in-flight requests
```
This is the precise, quantitative version of [`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md)'s "bound by time, not by count" advice — Little's Law is what tells you *which* count corresponds to your actual time bound, instead of guessing a queue depth.

### The M/M/1 queue: the simplest model worth knowing by name
"M/M/1" means Markovian (memoryless, exponential) arrivals, Markovian service times, 1 server. Its closed-form results — average wait time, average queue length — are all expressed in terms of `ρ = λ/μ` (utilization: arrival rate over service rate), and every one of them blows up as `ρ → 1`.

```text
M/M/1 average wait in queue: Wq = ρ / (μ(1 - ρ))
```
Read that denominator: as utilization `ρ` approaches 1 (the server is "almost" at full capacity), wait time diverges toward infinity — not linearly, not gracefully, but as a vertical asymptote.

### Why "we're only at 80% CPU, we have headroom" is dangerously wrong
Plug `ρ = 0.8` versus `ρ = 0.95` into the M/M/1 formula: wait time doesn't grow by roughly the same ratio as utilization did — it grows explosively, because the formula's denominator is `(1-ρ)`, and `1-0.95` is five times smaller than `1-0.8`. This is the rigorous backing for [`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md)'s entire premise (queueing near saturation is a latency trap) and for why capacity planning targets utilization well below 100% — "well below," quantified, means staying far enough from the asymptote that small arrival-rate fluctuations don't produce huge latency swings.

### Variability makes it worse: the Pollaczek-Khinchine intuition
Real service times aren't exponential — they're often more variable (a cache hit takes 1ms, a cache miss takes 200ms, per [`05-http-stack/07-cache.md`](../05-http-stack/07-cache.md)'s bimodal example). The M/G/1 result (Pollaczek-Khinchine formula) shows wait time grows with the *variance* of service time, not just its mean — two systems with identical average service time but different variance have different queueing behavior, which is the formal reason [`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md) recommends applying adaptive concurrency limits per class of similarly-costed work rather than globally.

### Multiple servers change the shape, not the cliff
An M/M/c queue (c parallel servers, e.g. c worker threads) delays the same blow-up to higher utilization and smooths it somewhat (the basis of the Erlang-C formula used in call-center staffing), but the qualitative shape — wait time diverging as aggregate utilization approaches 1 — doesn't go away. Adding workers or threads buys headroom, not immunity.

## Practice
1. Using Little's Law, compute the in-flight request capacity [`proxy`](../../proxy) needs to hold for a target p99 latency and an expected request rate of your choosing — write the number down as an actual capacity-planning input.
2. Implement a tiny M/M/1 simulation (Poisson arrivals, exponential service time) and empirically measure average wait time at `ρ = 0.5, 0.8, 0.9, 0.95, 0.99` — plot it and confirm the blow-up shape matches the formula.
3. Repeat the simulation with a fixed (non-exponential) service time instead of exponential, holding the same mean, and compare wait times against the M/M/1 case at the same `ρ` — connect the difference to the Pollaczek-Khinchine intuition about variance.
4. Load-test [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) at increasing request rates and plot observed p50/p99 latency against measured utilization; identify where the curve starts bending toward the theoretical asymptote.
5. Revisit [`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md)'s adaptive concurrency limiting section and explain, using this file's formulas, why targeting a fixed utilization set-point (rather than a fixed request count) is the theoretically grounded choice.
