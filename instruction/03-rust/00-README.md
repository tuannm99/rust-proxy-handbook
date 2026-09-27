# Rust

Phase 3. Not a Rust tutorial — this assumes you know the syntax, and
covers the areas that actually decide whether proxy code is correct and
whether it holds up under the kind of scrutiny a principal-level systems/
network/backend engineer applies: ownership across tasks, lifetimes in
buffers, unsafe at FFI boundaries, shared state, async, pinning, the type
system's dispatch/generics story, error handling, iterators/closures,
smart pointers, concurrency patterns beyond raw locks, macros, API design,
FFI/ABI, and memory representation.

## Files

**Core language and memory model** (read in order first time through —
everything else in this directory and in `04-runtime/` assumes these):
- `01-ownership.md` — move semantics, `Copy` vs `Clone`, borrowing rules, drop order and RAII
- `02-lifetimes.md` — elision, structs holding borrowed data, lifetimes vs async, HRTB
- `03-unsafe.md` — what `unsafe` unlocks, the contract you take on, keeping it minimal and sound
- `04-sync.md` — `Arc`, `Mutex` vs `RwLock`, atomics and `Ordering`, shared state vs message passing
- `05-async.md` — the `Future` trait, async/await desugaring, cooperative scheduling, drop-is-cancel
- `06-pin.md` — why `Pin` exists, self-referential futures, `Unpin`

**Type system and idioms** (the day-to-day vocabulary of writing and reading proxy-grade Rust):
- `07-traits-and-generics.md` — static vs dynamic dispatch, monomorphization, associated types, object safety
- `08-error-handling.md` — `Result`/`?`, `thiserror` vs `anyhow`, panics vs errors, mutex poisoning
- `09-iterators-and-closures.md` — laziness, `Fn`/`FnMut`/`FnOnce`, `impl Trait` vs `Box<dyn Fn>`
- `10-smart-pointers-and-interior-mutability.md` — `Box`/`Rc`/`Arc`/`Cell`/`RefCell`/`Cow`, one spectrum

**Systems/backend engineering skills** (what separates "knows Rust syntax" from "builds production systems in it"):
- `11-concurrency-patterns.md` — channels (`mpsc`/`oneshot`/`broadcast`/`watch`), the actor pattern vs shared state
- `12-macros.md` — `macro_rules!`, hygiene, where derive/attribute proc-macros already run your code
- `13-api-design-and-modules.md` — visibility as contract, newtype, builder, sealed traits, semver
- `14-ffi-and-abi.md` — `repr(C)`, `extern "C"`, ownership across the FFI boundary, why plugins are `dyn Trait` not `dylib`
- `15-memory-layout.md` — `repr(Rust)` vs `repr(C)`, size/align/padding, niche optimization, enum layout

## Where it goes next

The core-language group underpins `04-runtime/`. `05-async.md`'s
cancellation semantics in particular come back as a real bug in
`06-proxy/01-upstream.md` (a dropped future skipping a counter decrement)
and in `08-observability/03-tracing.md` (a span guard held across
`.await`); `04-runtime/04-structured-concurrency.md` gives that same idea
its real tokio APIs (`JoinSet`, `select!` cancellation-safety). The type-
system and systems-engineering groups feed `05-http-stack/`,
`06-proxy/`, and `09-architecture/` directly — `07-traits-and-generics.md`
underpins every pluggable-strategy design (load balancer, plugin system),
`11-concurrency-patterns.md` underpins `06-proxy/01-upstream.md`'s
connection pool and `07-security/07-ratelimit.md`'s rate limiter,
`15-memory-layout.md` is the prerequisite for `14-memory/`'s allocator/
arena/slab designs and `17-performance/`'s cache-layout work.
