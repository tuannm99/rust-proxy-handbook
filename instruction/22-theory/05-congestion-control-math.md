# Congestion Control: The Math Behind the Knob

## What to learn

### The congestion window as the actual state variable
TCP's congestion window (`cwnd`) bounds how many unacknowledged bytes may be in flight; throughput is approximately `cwnd / RTT`. Every congestion control algorithm is, mechanically, a rule for growing and shrinking `cwnd` in response to signals — loss, delay, explicit congestion notification. [`01-network/08-tcp.md`](../01-network/08-tcp.md)'s practical treatment names this; this file derives the actual growth/shrink math.

### Slow start: exponential, on purpose
`cwnd` starts at a small initial value (historically 1-2 segments; RFC 6928 raised typical defaults to around 10) and doubles every RTT until the first loss or until hitting a threshold — exponential growth, because linear growth from a tiny starting window would take far too many round trips to reach a link's real capacity.

```text
RTT 1: cwnd = 10 segments
RTT 2: cwnd = 20
RTT 3: cwnd = 40
... doubling continues until loss or ssthresh
```
This exponential ramp-up is the concrete, quantitative reason a brand-new TCP connection is slower than a reused one even on an otherwise-idle network — [`01-network/08-tcp.md`](../01-network/08-tcp.md)'s connection-reuse argument has a precise shape here.

### Congestion avoidance: AIMD (additive increase, multiplicative decrease)
After slow start, `cwnd` grows by roughly one segment per RTT (additive increase) and halves on a loss event (multiplicative decrease) — classic Reno-style AIMD. AIMD is provably fair among competing flows in steady state (it converges toward equal shares of bottleneck bandwidth), which is precisely why it was chosen over faster-converging but less fair alternatives.

```text
additive increase:      cwnd += 1 segment per RTT (no loss)
multiplicative decrease: cwnd = cwnd / 2 (on loss)
```

### The throughput formula, and what it says about long fat networks
For loss-based AIMD, steady-state throughput is approximately proportional to `MSS / (RTT * sqrt(loss_rate))` — the "square-root formula." The direct, uncomfortable consequence: on a high-RTT, low-loss link, throughput scales with `1/RTT`, so the same loss rate that's negligible on a data-center-local connection can cap throughput hard on a cross-continent one — a quantitative reason a proxy's upstream-to-origin latency matters for throughput, not only for tail latency.

### Why Cubic and BBR exist: two different critiques of AIMD's math
**Cubic** (Linux's default) grows `cwnd` as a cubic function of time since the last loss rather than linearly, spending more time near the previous ceiling before probing higher — better suited to high-bandwidth, high-RTT ("long fat") networks where AIMD's slow additive climb wastes capacity. **BBR** abandons loss as the primary signal entirely and instead models the path's actual bottleneck bandwidth and minimum RTT directly, reacting to measured delay rather than waiting for a router to drop a packet — a direct response to the observation that loss-based control fills buffers (bufferbloat) long before it ever signals congestion.

### Why a proxy doesn't implement any of this — but should know it
This is entirely the kernel's TCP stack's job ([`01-network/08-tcp.md`](../01-network/08-tcp.md) already says so); the value of the math here is being able to reason quantitatively about a real production question — "why did throughput to this one region drop after nothing on our end changed" — instead of only qualitatively.

## Practice
1. By hand, compute `cwnd` after each of the first 6 RTTs of slow start starting from `cwnd = 10`, then compute it for another 6 RTTs of AIMD additive increase after one loss event halves it.
2. Using the square-root throughput formula, compute the throughput difference between a 1ms-RTT/0.01%-loss link and a 150ms-RTT/0.01%-loss link for the same MSS — quantify how much RTT alone costs you.
3. Capture a real file transfer with `tcpdump` or `ss -i` and identify the slow-start phase (exponential `cwnd` growth) transitioning into congestion avoidance (linear growth) from the observed throughput curve.
4. Check `sysctl net.ipv4.tcp_congestion_control` on a Linux machine you control, switch between `cubic` and `bbr` (if available), and compare throughput/latency on a deliberately lossy or high-RTT simulated link (`tc netem`).
5. Write one paragraph connecting this file's math to [`01-network/08-tcp.md`](../01-network/08-tcp.md)'s claim that connection reuse matters for throughput "not just latency" — show the quantitative reason a fresh connection's slow start actually costs you.
