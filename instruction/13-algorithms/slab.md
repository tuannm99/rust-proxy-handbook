# Slab

Fixed-size object storage with O(1) insert and remove, addressed by
integer index instead of pointer. The data structure behind connection
tables, LRU nodes, and arena-based graphs.
`14-memory/slab-allocator.md` covers the allocator-level view; this file
covers the data structure you actually use in Rust.

## What to learn

### Why indices beat pointers in Rust
A proxy needs a collection of long-lived, individually-removable objects —
one entry per active connection, one node per LRU entry. The obvious
shapes all have problems: `HashMap<Id, T>` hashes on every access and
scatters allocations; `Vec<T>` invalidates every index on removal;
`Rc<RefCell<T>>` graphs leak on cycles; raw pointers mean `unsafe`
(`03-rust/unsafe.md`).

A slab is a `Vec` of slots where **removal does not shift anything** — the
freed slot joins a free list, so every outstanding index stays valid. You
get array-speed access, stable handles, one contiguous allocation, and no
`unsafe`. This is why `Slab<T>` shows up under connection registries and
why an arena-backed LRU (`13-algorithms/lru.md`) is the idiomatic Rust
implementation.

### The free list lives inside the slots
The elegant part: the free list needs no separate storage. A vacant slot
holds the index of the *next* vacant slot, so the whole free list is
threaded through the unused slots at zero extra memory cost.

```rust
enum Slot<T> {
    Occupied(T),
    Vacant { next_free: Option<usize> },
}

struct Slab<T> {
    slots: Vec<Slot<T>>,
    next_free: Option<usize>, // head of the free list
    len: usize,
}
```

Insert pops the head of the free list (or pushes a new slot if empty);
remove writes `Vacant` into the slot and pushes it onto the head. Both are
O(1) with no allocation in the steady state.

Gotcha: `Slot<T>` is as large as the larger of `T` and a `usize`, plus the
enum discriminant. For small `T` that overhead is proportional; production
slab implementations pack the discriminant into a spare bit or keep a
separate occupancy bitmap.

### The ABA / stale-handle problem
This is the bug that bites. Connection 7 closes, freeing slot 7; a new
connection immediately reuses slot 7. Any code still holding index 7 —
a queued timer, an in-flight response, a metrics callback — now reads or
mutates the *wrong connection*. There is no type error and no panic; it is
silent cross-talk between unrelated clients, which in a proxy means one
user's response reaching another.

The fix is a **generation counter**: pair each slot with a counter
incremented on every removal, and make the public handle `(index,
generation)`. Access checks that the slot's generation matches the
handle's and returns `None` on mismatch, turning a silent aliasing bug into
an ordinary lookup miss.

Gotcha: the generation must increment on *removal*, not insertion, and it
must be wide enough not to wrap during the lifetime of any outstanding
handle — a `u32` at high connection churn is worth thinking about rather
than assuming.

### Capacity and the shrink problem
A slab never shrinks on its own: after a traffic spike creates 100k
connection slots, the `Vec` stays 100k slots wide even at 100 active
connections. For a long-running proxy that is a permanent memory
high-water mark set by your worst spike
(see `14-memory/fragmentation.md`).

Compacting means moving occupied entries into low slots, which invalidates
their indices — precisely the property the slab exists to provide. The
practical options are to bound the slab and reject beyond capacity
(reasonable: it doubles as a connection limit), or to accept the
high-water mark as the cost. Pre-sizing with `with_capacity` for expected
peak load also avoids repeated reallocation during a ramp-up, when the
proxy is already under stress.

## Practice
1. Implement `Slab<T>` with the intrusive free list above — `insert`,
   `remove`, `get`, `get_mut` — and assert that removing a low index leaves
   all other indices valid.
2. Reproduce the stale-handle bug: hold an index across a remove+insert
   cycle and observe it reading the new occupant. Then add generation
   counters and confirm the same access now returns `None`.
3. Use it in `labs/05-reverse-proxy` as the connection registry, keyed by a
   generational handle rather than an address.
4. Compare against `HashMap<u64, T>` for 100k insert/remove/lookup cycles —
   measure both time and peak memory.
5. Demonstrate the shrink problem: grow to 100k entries, remove all but
   100, and show the memory does not return. Add a capacity bound and the
   test that rejects insertion past it.
6. Back the LRU from `13-algorithms/lru.md` with your slab instead of a
   bare `Vec`.
