# Priority Queues at Scale: Timer Wheels

`13-algorithms/heap.md` covers the binary heap backing a small
priority queue (one entry per upstream). This file covers what happens
when the priority queue's job is scheduling *timeouts* — potentially one
per connection, at proxy scale — where a heap's O(log n) per-operation
cost and awkward cancellation stop being "good enough."

## What to learn

### Why a heap struggles as a timeout store
A proxy schedules a timeout on essentially every connection and request
(idle timeout, read timeout, retry deadline) and cancels most of them
early when the operation completes normally. A heap handles this as
insert-then-usually-cancel-before-firing, and canceling an arbitrary
entry in a binary heap (not just the root) needs the same decrease-key
machinery `heap.md` describes for updates — an O(log n) search-and-remove
per cancellation, at a rate of "once per request," is real overhead at
high connection counts.

### The timer wheel: bucket by when, not by exact order
A timer wheel trades exact ordering for O(1) insert and O(1) cancel by
bucketing deadlines into a fixed number of time slots (a circular array,
like `13-algorithms/ring-buffer.md`'s structure, but indexed by future
time rather than insertion order):

```rust
struct TimerWheel {
    slots: Vec<Vec<TimerId>>, // slots[i] = timers firing in tick i
    current_slot: usize,
    tick_duration: std::time::Duration,
}
// schedule(deadline): compute slot = (current_slot + ticks_until(deadline)) % slots.len()
//                     push the timer id into slots[slot]
// cancel(id): remove id from whichever slot holds it — O(1) with a side
//             index from id -> slot
// on each tick: advance current_slot, fire (and drain) everything in it
```

A single wheel only covers deadlines within `slots.len() * tick_duration`;
longer deadlines need a **hierarchical timer wheel** (multiple wheels at
increasing granularity — seconds, then minutes, then hours — where a
timer is re-inserted into a finer wheel as its deadline approaches). This
is exactly what the Linux kernel's own timer implementation and Netty's
`HashedWheelTimer` do, and it's the design `tokio::time` uses internally
for the driver behind every `tokio::time::sleep` and `timeout` call.

### Precision vs cost tradeoff
A timer wheel doesn't fire at the exact deadline — it fires on the tick
boundary the deadline falls into, so precision is bounded by
`tick_duration`. For connection/request timeouts (measured in tens of
milliseconds to seconds) a tick of 10-50ms is imperceptible; for anything
needing sub-millisecond precision, a heap's exact ordering is worth its
higher per-operation cost. Most proxy timeout use cases are firmly in the
"a wheel is fine" category — check what you're actually timing before
assuming you need heap precision.

### Gotcha: don't build your own if the runtime already has one
`tokio::time::sleep`/`timeout`/`interval` are backed by exactly this
mechanism inside tokio's runtime already. Building a second, separate
timer wheel in `proxy/` for something `tokio::time` already covers
duplicates a well-tested subsystem for no benefit — reach for this
file's content when you need to understand *why* `tokio::time::sleep` is
cheap to create and cancel by the thousand, not to replace it.

## Practice
1. Implement a single-level timer wheel and benchmark insert, fire, and
   cancel against your `heap.md` implementation at 100k scheduled
   timeouts with a 90% early-cancellation rate (simulating requests that
   complete before their timeout fires).
2. Extend it to a two-level hierarchical wheel (e.g. milliseconds and
   seconds) and confirm a timer scheduled beyond the first wheel's range
   correctly migrates into the fine wheel as its deadline approaches.
3. In `labs/05-reverse-proxy`, replace a naive per-connection
   `tokio::time::sleep`-per-timeout pattern with a design that reasons
   about batch cancellation (e.g. connection close cancels every pending
   timer for it), and explain in writing why `tokio::time` already
   avoids the cost you just measured in step 1.
