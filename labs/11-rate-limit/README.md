# 11-rate-limit

## Goal

Per-client rate limiting using token bucket, sliding window, and leaky
bucket, returning a proper `429` with `Retry-After` when exceeded.

## Done when

- [ ] Exceeding the limit returns `429` with a `Retry-After` that is accurate: retrying after that many seconds succeeds.
- [ ] Over a 60-second test, the admitted rate matches the configured rate within a few percent.
- [ ] Token bucket admits a burst up to its capacity, then settles to the steady rate.
- [ ] You reproduce the fixed-window boundary burst (up to 2x the limit across a window edge) and show your sliding window doesn't allow it ([`instruction/13-algorithms/sliding-window.md`](../../instruction/13-algorithms/sliding-window.md)).
- [ ] Hundreds of concurrent tasks hitting one key never over-admit (no race between check and update).
- [ ] 1,000,000 distinct client keys don't grow memory without bound — idle keys are evicted or expire.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/07-security/07-ratelimit.md`](../../instruction/07-security/07-ratelimit.md)
- [`instruction/07-security/11-load-shedding.md`](../../instruction/07-security/11-load-shedding.md) — what to do once limits aren't enough
- [`instruction/13-algorithms/token-bucket.md`](../../instruction/13-algorithms/token-bucket.md), [`instruction/13-algorithms/sliding-window.md`](../../instruction/13-algorithms/sliding-window.md), [`instruction/13-algorithms/leaky-bucket.md`](../../instruction/13-algorithms/leaky-bucket.md)
- [`instruction/03-rust/16-testing-idioms.md`](../../instruction/03-rust/16-testing-idioms.md) — injecting a fake clock so rate tests aren't flaky

## Run

```
cargo run -p rate-limit
```
