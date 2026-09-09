# Object Pools

`14-memory/arena.md` frees a whole request's allocations at once but
starts fresh next request. An object pool is for the opposite pattern:
the *same-shaped* object, reused across many requests, so it's never
freed at all — just checked out and returned.

## What to learn

### Checkout/return, usually via RAII
A pool holds a set of pre-constructed objects (connection structs, I/O
buffers) in a free list; a caller checks one out, uses it, and returns it
when done. Returning it manually is easy to forget on an error path, so
the idiomatic Rust version wraps the checkout in a guard whose `Drop`
returns it to the pool automatically:

```rust
struct PooledBuffer<'a> {
    buf: Vec<u8>,
    pool: &'a Pool,
}
impl Drop for PooledBuffer<'_> {
    fn drop(&mut self) {
        self.buf.clear();                 // reset before returning
        self.pool.push(std::mem::take(&mut self.buf));
    }
}
```
The `Drop` impl is what makes this safe under early returns and panics —
the object goes back to the pool (or is dropped for real, if the pool is
gone) no matter how the checkout's scope ends.

### Object pool vs arena: reuse across requests vs bulk-free within one
The distinction that matters: an arena's contents all die with the
request that allocated them and the arena itself is typically short-
lived (one per request, or reused after a full reset). A pool's objects
outlive any single request — a connection struct or buffer is checked
out, used for one request, returned, and checked out again by a
completely different request later. Reach for a pool when the object's
*identity* needs to persist and be reused (a `Vec<u8>` whose backing
allocation you want to keep); reach for an arena when you just want many
small allocations to die together.

### Gotcha: reset state before reuse, every time
The pool's entire benefit — skipping alloc/free — becomes a correctness
and potentially a security bug if a returned object isn't reset before
its next checkout. A buffer returned with a stale length or stale bytes
from the previous request's data, then partially overwritten by a
shorter next request, can leak the previous caller's bytes into the
current one's response if the code trusts the buffer's old length instead
of the new write's actual length. Reset (clear, zero, or at minimum
truncate to the new content's real length) in the `Drop` impl or at
checkout time — pick one place, and make it impossible to skip.

### Gotcha: an unbounded pool is a pool-shaped memory leak
A pool that only ever grows (checked out more than returned, or growing
to serve a traffic spike and never shrinking) reaches the same
uncontrolled high-water mark `fragmentation.md` describes for the
allocator itself — except now it's your code holding the memory instead
of the allocator. Cap the pool's maximum size, and when at capacity,
either block the checkout, fall back to a real allocation, or reject —
decide which, deliberately, rather than growing without bound by default.

## Practice
1. Implement an RAII-guarded pool of reusable `Vec<u8>` I/O buffers for
   `labs/05-reverse-proxy`'s per-connection copy path.
2. Add a test that a buffer, once returned to the pool and checked out
   again, never contains bytes from its previous use.
3. Benchmark alloc-per-request vs pooled buffers under concurrent load;
   measure both throughput and allocator pressure (allocation count).
4. Cap the pool size and add a load test that drives more concurrent
   checkouts than the cap; confirm your chosen overflow behavior (block,
   fall back to fresh allocation, or reject) is what actually happens.
