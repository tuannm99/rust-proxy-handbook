# Retry & Circuit Breaker

## What to learn
### Idempotency: the rule that comes before any retry logic
Never retry a request whose method/semantics aren't safe to repeat unless
you know the upstream is idempotent for it. GET/HEAD/PUT/DELETE are
generally safe to retry; a bare POST usually is not (it might create a
resource twice). A production proxy either only auto-retries
idempotent methods, or requires an explicit idempotency key from the
client for POST retries.

### Retry budgets
Retrying blindly on every failure can turn a small upstream blip into a
retry storm that takes the upstream down completely (each failed request
now costs 2-3x). A retry budget caps total retries as a percentage of
total requests over a rolling window (e.g. "retries may not exceed 10% of
requests in the last 10s") — if the budget is exhausted, stop retrying and
fail fast instead.

### Exponential backoff with jitter
Fixed-delay retries from many clients synchronize into retry storms.
Exponential backoff with random jitter spreads retries out in time.

```rust
fn backoff(attempt: u32, base: std::time::Duration) -> std::time::Duration {
    let exp = base * 2u32.pow(attempt.min(6));
    let jitter_ms = rand_range(0..exp.as_millis() as u64 / 2);
    exp + std::time::Duration::from_millis(jitter_ms)
}
```
Gotcha: cap the exponent (as above) — `2u32.pow(attempt)` overflows fast if
`attempt` is unbounded.

### Circuit breaker states
A circuit breaker stops calling a consistently-failing upstream entirely
for a cool-down period, instead of retrying into it forever.

```rust
enum CircuitState {
    Closed,                                 // normal, calls pass through
    Open { until: std::time::Instant },     // failing fast, no calls made
    HalfOpen,                               // one trial call allowed
}
```
Transitions: `Closed -> Open` after N consecutive failures; `Open ->
HalfOpen` once `until` elapses; `HalfOpen -> Closed` on a successful trial
call, `HalfOpen -> Open` (with `until` reset, usually backed off further)
on a failed one.

Gotcha: don't let concurrent requests all become the "one trial call" in
`HalfOpen` — gate it with a `compare_exchange` or a semaphore of size 1, or
you get a thundering herd against a barely-recovered upstream.

## Practice
1. In `labs/05-reverse-proxy`, add a retry wrapper around the
   upstream call that only retries GET/HEAD and uses the backoff+jitter
   function above, capped at 3 attempts.
2. Add a rolling retry budget counter; force upstream failures and confirm
   retries stop once the budget is exhausted instead of retrying forever.
3. Implement the `CircuitState` enum and wire it per-upstream; verify with
   logs that a consistently-failing upstream moves Closed -> Open -> HalfOpen
   -> Closed (or back to Open) as expected.
4. Load-test with one upstream deliberately slow and confirm the circuit
   breaker isolates it without you needing to restart the proxy.
