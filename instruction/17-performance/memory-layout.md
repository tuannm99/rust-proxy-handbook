# Memory Layout

Struct field ordering, padding, and `#[repr]` — how the same fields can
occupy very different amounts of memory, and why that reaches into
`13-algorithms/slab.md` and `14-memory/fragmentation.md`.

## What to learn

### Alignment forces padding
Every type has an alignment; a field must sit at an offset that is a
multiple of it. The compiler inserts padding to satisfy this, and field
*order* decides how much. A `u8` followed by a `u64` wastes 7 bytes of
padding to align the `u64`; the same fields reordered large-to-small waste
none.

```rust
struct Bad  { a: u8, b: u64, c: u8, d: u64 }  // 32 bytes (padding)
struct Good { b: u64, d: u64, a: u8, c: u8 }  // 24 bytes (packed tighter)
```

### Rust reorders by default — and that is good
Unlike C, Rust's default `repr(Rust)` is free to reorder fields, and it
does, to minimize padding automatically. So you usually do *not* hand-order
fields for size — the compiler already did. The reason to care is the
exceptions: `#[repr(C)]` (for FFI or a wire format) freezes declaration
order, and then the `Bad`/`Good` distinction above is back in your hands.
The eBPF map structs in `16-kernel/ebpf.md` and any struct you memcpy onto
the wire are `repr(C)` and must be ordered deliberately.

### Why size crosses into fragmentation and slabs
This is the payoff, not a micro-optimization. Allocators round to size
classes (`14-memory/fragmentation.md`): a 129-byte struct takes a 160-byte
slot. Shrinking that struct below 128 — by removing padding, or by the
hot/cold split below — moves it to the 128 class and saves 32 bytes *per
instance*. At 100k connections that is a step-function 3 MB, and it also
means more objects per slab page (`13-algorithms/slab.md`), i.e. better
cache density on the hot path (`17-performance/cpu-cache.md`). Size is
leverage on three subsystems at once.

### Hot/cold splitting
A per-connection struct often has a few fields touched every request
(state, buffer pointers) and many touched rarely (original client cert,
creation timestamp, debug counters). Packing them together drags the cold
fields through cache on every access. Split them: keep the hot fields in a
small struct and box the cold ones behind a pointer.

```rust
struct Conn {
    state: State,          // hot: touched per request
    buf: BytesMut,         // hot
    cold: Box<ConnCold>,   // rarely touched → one indirection, off the hot line
}
```

The trade is one pointer indirection to reach cold data; worth it only when
the hot struct then fits more per cache line and the cold data really is
cold.

### Enums and niches
Rust exploits "niches" — invalid bit patterns — to store enum
discriminants for free. `Option<&T>` is the same size as `&T` because null
is the `None` niche; `Option<NonZeroU32>` is 4 bytes, not 8. This means
using `NonZero*` and references instead of sentinel values (`u32::MAX`
means "none") can shrink a struct with zero code change. Reach for it in
the arena/index structures (`13-algorithms/lru.md`, `15-parser/ast.md`)
where a "none" link is common.

Gotcha: `#[repr(packed)]` (remove *all* padding) is almost never the
answer — it creates unaligned fields, and taking a reference to one is
undefined behavior, so it turns a size win into a soundness hazard
(`03-rust/unsafe.md`). Use field ordering and `NonZero` niches, not
`packed`.

## Practice
1. Use `std::mem::size_of` and `#[repr(C)]` to reproduce the `Bad`/`Good`
   difference, then remove `repr(C)` and confirm Rust already packs it —
   proving you rarely need to hand-order.
2. Print `size_of` for a real `proxy` per-connection struct; check whether
   it sits just past a size-class boundary (`14-memory/fragmentation.md`)
   and whether shrinking it crosses back under one.
3. Do a hot/cold split on that struct, measure the hot-path benchmark
   (`17-performance/cpu-cache.md`), and keep the change only if the number
   moves.
4. Replace a `u32::MAX`-means-none link in an arena structure with
   `Option<NonZeroU32>` and confirm the struct got smaller with no runtime
   change.
5. Verify the fragmentation link: allocate 100k of the struct before and
   after shrinking it under a size-class boundary and compare RSS, tying
   this back to `13-algorithms/slab.md`'s objects-per-page.
