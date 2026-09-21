# Arena Allocation

`14-memory/01-allocator.md` covers general-purpose allocation. An arena is
the opposite strategy for a specific shape of workload — one a proxy has
constantly: many small allocations that all die together at the end of
one request.

## What to learn

### Bump allocation: one pointer, no free list
An arena hands out memory by advancing a single pointer through a
pre-allocated block; there is no per-object free — the entire block is
released (or reset and reused) as one operation when the arena's scope
ends:

```rust
// what bumpalo does conceptually
struct Arena {
    chunk: Vec<u8>,
    offset: usize,
}
impl Arena {
    fn alloc<T>(&mut self, value: T) -> &mut T {
        // bump self.offset up by size_of::<T>() (aligned), write value there
        // grow to a new chunk if the current one is full
        unimplemented!()
    }
}
```
In practice you reach for the `bumpalo` or `typed-arena` crate rather
than hand-rolling this — the point is the *shape*: allocation is a
pointer bump (as cheap as allocation gets), and N objects that would
otherwise be N separate `Box`/`Vec` allocations-and-frees become one
allocation and one bulk release.

### The fit: request-scoped data
A single HTTP request typically allocates many short-lived pieces —
parsed headers, a routing decision's intermediate values, buffers for
transformation — all of which are only needed until the response is
sent. Allocating every one of them into a per-request arena and dropping
the whole arena when the request completes turns what would be dozens of
individual frees into one. This is also exactly the fix
`14-memory/06-fragmentation.md` recommends for the "mixed lifetimes"
fragmentation trigger: request-scoped data never gets a chance to
interleave with longer-lived connection state if it lives in its own
arena.

### Gotcha: arena-allocated data's lifetime is the arena's lifetime
Values allocated from an arena are references borrowed from it — they
cannot outlive the arena without being copied out first. In an async
handler, this means the arena (or a reference into it) must live at
least as long as every `.await` point that touches data borrowed from
it, which is exactly the kind of self-referential-across-await-points
situation `03-rust/06-pin.md` describes. Concretely: don't allocate into an
arena that's a local variable and then try to hold a reference into it
across a suspended future that outlives the function — either own the
arena inside the future's state, or copy the data out before the
suspension point.

### Gotcha: unbounded per-request arenas are a memory bomb
An arena that grows without limit (e.g. one processing an attacker-
controlled, unbounded request body into arena-allocated pieces) removes
the natural backpressure a per-allocation limit would have provided.
Cap the arena's total size per request and reject/error past it, the same
way you'd cap any other per-request resource (`07-security/09-ddos.md`).

## Practice
1. Use `bumpalo` to arena-allocate the parsed headers for one request in
   `labs/01-http-parser`, replacing per-header `String`/`Vec` allocations.
2. Measure allocation count and total time for parsing a request with 20
   headers, with and without the arena, at high concurrency.
3. Deliberately try to return a reference borrowed from a function-local
   arena from that function, observe the compile error, and rewrite it to
   either own the arena in the caller or copy the needed data out.
4. Add a maximum arena size per request and confirm a request whose
   headers would exceed it is rejected cleanly rather than growing the
   arena unbounded.
