# Memory Layout and Representation

## What to learn

### `repr(Rust)` vs. `repr(C)`: the compiler may reorder your fields
Without an explicit `#[repr(...)]`, the compiler is free to reorder struct fields to minimize padding — two structs with identical field lists can end up with different layouts, and field order in memory need not match declaration order. This mostly doesn't matter for pure-Rust code (fields are accessed by name), but matters the moment you need a stable layout: FFI ([`03-rust/14-ffi-and-abi.md`](14-ffi-and-abi.md)), or reasoning about cache-line packing for a hot struct ([`17-performance/01-cpu-cache.md`](../17-performance/01-cpu-cache.md)).

```rust
struct A { a: u8, b: u64, c: u8 }              // repr(Rust): compiler may pack tighter than declared order
#[repr(C)] struct B { a: u8, b: u64, c: u8 }   // fields stay in declared order, C-style padding rules
```

### Size and alignment: `size_of`, `align_of`, and padding
Every type has a size and an alignment requirement; a struct's total size is generally rounded up to a multiple of its largest field's alignment, which is why field *order* affects total size even under `repr(C)` — grouping same-sized fields together minimizes wasted padding.

```rust
use std::mem::size_of;
#[repr(C)] struct Bad { a: u8, b: u64, c: u8 }  // u64's 8-byte alignment forces padding around both u8s
#[repr(C)] struct Good { b: u64, a: u8, c: u8 } // both u8s pack together after the u64
assert!(size_of::<Good>() < size_of::<Bad>());
```
For a struct allocated per-connection at scale — tens of thousands of live connections in a proxy — this kind of padding difference is real memory, not a micro-optimization. See [`17-performance/01-cpu-cache.md`](../17-performance/01-cpu-cache.md) and [`17-performance/04-memory-layout.md`](../17-performance/04-memory-layout.md) for the deeper cache-line-packing treatment this file is a prerequisite for.

### Niche optimization: why `Option<&T>` is the same size as `&T`
The compiler exploits invalid bit patterns of a type — a reference can never be null — to represent `None` as that otherwise-impossible pattern, so `Option<&T>`, `Option<Box<T>>`, and `Option<NonZeroU32>` all cost zero extra bytes over the non-`Option` type. This is a load-bearing optimization, not a curiosity: it's the concrete reason idiomatic Rust reaches for `Option<NonZeroU32>` over `Option<u32>` for an id that's never legitimately zero.

```rust
assert_eq!(size_of::<Option<&u8>>(), size_of::<&u8>()); // niche-optimized: zero extra cost
assert_eq!(size_of::<Option<std::num::NonZeroU32>>(), size_of::<u32>());
```

### Enum layout: tags cost space too
A plain enum with data-carrying variants is sized to fit its largest variant plus a discriminant tag; a `Result<SmallOk, HugeErr>` pays for `HugeErr`'s size even on the success path. If a hot-path `Result`'s error variant carries something large (a whole request struct), boxing it (`Result<T, Box<HugeErr>>`) shrinks the type back down for the common case.

```rust
enum Small { A, B(u8) }              // small
enum Mixed { A, B([u8; 256]) }       // sized for the 256-byte variant even when it's A
```

### Why this connects to buffer pools and arenas
[`14-memory/`](../14-memory)'s arena/slab/object-pool designs all assume you know the exact size and alignment of what you're storing — a slab allocator ([`14-memory/05-slab-allocator.md`](../14-memory/05-slab-allocator.md)) can't be designed without knowing precisely how big each slot needs to be, which is `size_of`/`align_of` applied to your actual connection-state struct.

## Practice
1. Take a per-connection state struct from [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), print `std::mem::size_of::<T>()`, reorder fields by descending size, and measure whether it shrinks.
2. Confirm the niche-optimization claim yourself: compare `size_of::<Option<u32>>()` against `size_of::<Option<std::num::NonZeroU32>>()` and against `size_of::<Option<&u32>>()`.
3. Build a `Result<T, E>` where `E` is a large struct, measure `size_of::<Result<T, E>>()`, then box `E` and measure again.
4. Read [`14-memory/05-slab-allocator.md`](../14-memory/05-slab-allocator.md) and connect its slot-sizing logic back to this file's `size_of`/`align_of` discussion, in your own words.
5. Add `#[repr(C)]` to a struct that's currently `repr(Rust)` and use `size_of` (or `std::mem::offset_of!` if your toolchain has it) to check whether the layout actually changed.
