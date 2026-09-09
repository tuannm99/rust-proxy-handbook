# Ring Buffer

A fixed-size circular buffer. `08-observability/logging.md` needs one so
that logging never allocates on the request hot path and never blocks it
waiting for a slow writer.

## What to learn

### The structure
A `Vec<T>` of fixed capacity plus a head and tail index, both taken modulo
capacity:

```rust
struct RingBuffer<T> {
    buf: Vec<Option<T>>,
    head: usize, // next write position
    tail: usize, // next read position
    cap: usize,
}
// push: buf[head] = Some(item); head = (head + 1) % cap
// pop:  let item = buf[tail].take(); tail = (tail + 1) % cap
```

No allocation after construction, no shifting of elements — every push
and pop is O(1), which is the entire point for a structure sitting on a
hot path.

### SPSC: lock-free between exactly one producer and one consumer
When there's exactly one writer thread and one reader thread — the common
shape for "request handlers produce log lines, one background task
writes them to disk" — the ring buffer needs no lock at all: the producer
only ever writes `head` and reads `tail`, the consumer only ever writes
`tail` and reads `head`, and each side's own index only needs `Ordering::Release`
on write / `Acquire` on read to be visible to the other side correctly.
This is what `tracing_appender::non_blocking` (referenced in
`08-observability/logging.md`) and most SPSC channel crates (`crossbeam`,
`ringbuf`) implement.

### Full-buffer policy: never block the hot path
A logging ring buffer must decide what happens when full, and "block the
producer" is the wrong default for a request-handling thread — it turns a
disk-write slowdown into request latency. The standard choices are
**drop the newest** (reject the incoming log line, cheap, loses the most
recent event) or **overwrite the oldest** (advance `tail` along with
`head`, loses history but never rejects). Pick based on whether "we know
we dropped something" (with a dropped-count metric,
`08-observability/metrics.md`) matters more than keeping the latest event.

### Gotcha: false sharing between head and tail
`head` and `tail` are written by different threads (in the SPSC case) but
if they sit on the same cache line, every write to one invalidates the
other core's cached copy of the line — the two threads end up serializing
on cache traffic despite touching logically independent data. This is
`17-performance/false-sharing.md`'s exact failure mode; pad `head` and
`tail` onto separate cache lines (`#[repr(align(64))]` on a wrapper, or
interleave with padding fields) to fix it.

## Practice
1. Implement the SPSC ring buffer above with correct `Acquire`/`Release`
   ordering; write a test with one producer and one consumer thread that
   asserts no item is ever lost or duplicated under load.
2. Implement both full-buffer policies (drop-newest, overwrite-oldest)
   behind a flag, and add a dropped-line counter exported as a metric.
3. Wire the ring buffer in as the buffer behind an async logging path in
   `proxy` (or a standalone test harness modeling it), and load-test with
   a deliberately slow "disk writer" consumer to confirm request handling
   never blocks on it.
4. Reproduce the false-sharing cost: benchmark throughput with `head` and
   `tail` adjacent in memory, then with them padded onto separate cache
   lines, and measure the difference under concurrent access.
