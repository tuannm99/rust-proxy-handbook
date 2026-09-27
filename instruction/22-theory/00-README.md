# CS Theory

A reference layer, like [`13-algorithms/`](../13-algorithms): the classical CS theory behind
what [`01-network/`](../01-network), [`02-linux/`](../02-linux), and [`03-rust/`](../03-rust) teach at a practical,
"what you need to build a proxy" level. No reading order — pulled in from
those directories as needed, not read start to finish. None of it is
required to finish [`proxy/`](../../proxy); it exists for readers who want the academic
grounding underneath the practical treatment — the deadlock theory behind
a real lock-ordering bug, the math behind a congestion-control knob you
already tuned by feel.

## Status: written

## Files

- [`01-deadlock.md`](01-deadlock.md) — the four Coffman conditions, resource allocation graphs, dining philosophers, prevention/avoidance/detection
- [`02-sync-classics.md`](02-sync-classics.md) — producer-consumer, readers-writers, semaphores vs mutexes formally, the barrier problem
- [`03-page-replacement.md`](03-page-replacement.md) — FIFO, LRU, clock/second-chance, optimal (Bélády's), thrashing and the working-set model
- [`04-cpu-scheduling.md`](04-cpu-scheduling.md) — FCFS, SJF/SRTF, round-robin, multi-level feedback queue, and the metrics that judge them
- [`05-congestion-control-math.md`](05-congestion-control-math.md) — slow start's exponential growth, AIMD, the throughput formula, why Cubic/BBR exist
- [`06-queueing-theory.md`](06-queueing-theory.md) — Little's Law, M/M/1, why "80% CPU" is not "20% headroom," the Pollaczek-Khinchine intuition
- [`07-crypto-math.md`](07-crypto-math.md) — AES's round structure, Diffie-Hellman, RSA, why hashes are one-way
- [`08-amdahls-law.md`](08-amdahls-law.md) — the serial-fraction ceiling on speedup, Gustafson's reframing, why tokio targets throughput over per-request parallelism
- [`09-cap-flp.md`](09-cap-flp.md) — CAP theorem's pick-two-under-partition, the FLP impossibility result, why every real consensus system relies on timeouts

## Where this connects back

- [`01-deadlock.md`](01-deadlock.md) and [`02-sync-classics.md`](02-sync-classics.md) → [`03-rust/04-sync.md`](../03-rust/04-sync.md), [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)
- [`03-page-replacement.md`](03-page-replacement.md) → [`02-linux/09-memory.md`](../02-linux/09-memory.md)
- [`04-cpu-scheduling.md`](04-cpu-scheduling.md) → [`16-kernel/07-scheduler.md`](../16-kernel/07-scheduler.md), [`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md)
- [`05-congestion-control-math.md`](05-congestion-control-math.md) → [`01-network/08-tcp.md`](../01-network/08-tcp.md)
- [`06-queueing-theory.md`](06-queueing-theory.md) → [`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md)
- [`07-crypto-math.md`](07-crypto-math.md) → [`01-network/06-crypto-basics.md`](../01-network/06-crypto-basics.md), [`01-network/13-tls.md`](../01-network/13-tls.md)
- [`08-amdahls-law.md`](08-amdahls-law.md) → [`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md), [`04-runtime/05-runtime-comparisons.md`](../04-runtime/05-runtime-comparisons.md), [`17-performance/`](../17-performance)
- [`09-cap-flp.md`](09-cap-flp.md) → [`18-distributed/`](../18-distributed) (all four files)

Each practical file above only points here for the reader who wants the
theory; it never assumes you've read this directory first.
