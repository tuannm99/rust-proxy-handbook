# Leaky Bucket

`07-security/ratelimit.md` introduces leaky bucket as "a queue that drains
at a constant rate." This file covers the two ways that sentence actually
gets implemented, because they behave very differently under load.

## What to learn

### As a meter: leaky bucket is token bucket upside down
The "leaky bucket algorithm as meter" formulation tracks a water level
that fills on each request and drains (leaks) at a constant rate; a
request is rejected if it would overflow the bucket's capacity.

```rust
struct LeakyBucketMeter {
    level: f64,       // current "water" level
    capacity: f64,
    leak_rate: f64,   // units drained per second
    last_leak: std::time::Instant,
}

impl LeakyBucketMeter {
    fn try_add(&mut self, now: std::time::Instant, cost: f64) -> bool {
        let elapsed = now.duration_since(self.last_leak).as_secs_f64();
        self.level = (self.level - elapsed * self.leak_rate).max(0.0);
        self.last_leak = now;
        if self.level + cost <= self.capacity {
            self.level += cost;
            true
        } else {
            false
        }
    }
}
```

This is mathematically the mirror image of token bucket (fill vs drain,
reject-on-full vs reject-on-empty) and produces the *same* accept/reject
decisions as token bucket for the same capacity and rate — see
`13-algorithms/token-bucket.md`'s GCRA. If you already have a correct
token bucket, you don't need to separately implement this form; it exists
mostly because vendor docs (AWS, GCP quota docs, some API gateways)
describe their limiter this way.

### As a queue: leaky bucket is a rate-limited buffer, not a meter
The other formulation is a literal bounded FIFO queue that a background
process drains at a fixed rate, one request at a time:

```rust
struct LeakyBucketQueue<T> {
    queue: std::collections::VecDeque<T>,
    capacity: usize,
    drain_interval: std::time::Duration,
}
// enqueue: reject if queue.len() == capacity, else push_back
// a task on a fixed interval pops one item and forwards it downstream
```

This is a fundamentally different guarantee from the meter form: it
**smooths output to a strictly constant rate** — downstream never sees
more than one request per `drain_interval`, no matter how bursty the
input, as long as the queue has room. Token bucket and the meter form of
leaky bucket both *allow* a burst through immediately up to capacity; the
queue form never does. Reach for it specifically when the thing being
protected genuinely cannot absorb a burst (a fixed-capacity legacy
backend, a hardware device with a single in-flight request) rather than
for "fair API usage," where bursts are usually fine.

### The queue form's real failure mode: latency, not rejection
A full meter/token-bucket rejects immediately with a 429. A full leaky
bucket *queue* can be configured to either reject when full (bounded
queue) or block the caller until there's room — and the blocking version
turns a rate-limit problem into a latency problem: at `capacity = 1000`
and `drain_rate = 10/sec`, a request that finds the queue full waits up to
100 seconds before being served or timing out. This is often worse for
the caller than an immediate 429, because the caller's own timeout may
fire first, and it now holds a connection/thread open for the whole wait.

Gotcha: an *unbounded* queue removes the rejection case entirely and turns
sustained overload into unbounded memory growth *and* unbounded latency
simultaneously — the queue is not "leaking" fast enough to keep up, and
nothing stops it from growing. Always bound the queue, and prefer
rejecting on a full queue over blocking the caller in a proxy context,
where every blocked task holds resources (a connection, a task) for the
wait.

## Practice
1. In `labs/11-rate-limit`, implement the meter form and confirm, on a
   scripted request timeline, that it accepts/rejects identically to your
   `token-bucket.md` implementation at matching capacity/rate.
2. Implement the queue form with a bounded `VecDeque` and a background
   task draining at a fixed interval; add a `try_enqueue` that rejects
   immediately when full.
3. Demonstrate the smoothing difference directly: fire a burst of
   `2 * capacity` requests at both your token bucket and your leaky-bucket
   queue, and plot (or just print, timestamped) when each one lets
   requests through — token bucket admits the first `capacity` instantly,
   the queue trickles all of them out at `drain_rate`.
4. Change the queue form to block-until-room instead of reject-on-full,
   measure the worst-case wait at your chosen capacity/rate, and write
   down why you would or wouldn't ship that behavior in `proxy/`.
