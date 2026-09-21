# Rust

Phase 3. Not a Rust tutorial — this assumes you know the syntax, and
covers the six areas that actually decide whether proxy code is correct:
ownership across tasks, lifetimes in buffers, unsafe at FFI boundaries,
shared state, async, and pinning.

## Files

- `01-ownership.md` — move semantics, `Copy` vs `Clone`, borrowing rules, drop order and RAII
- `02-lifetimes.md` — elision, structs holding borrowed data, lifetimes vs async, HRTB
- `03-unsafe.md` — what `unsafe` unlocks, the contract you take on, keeping it minimal and sound
- `04-sync.md` — `Arc`, `Mutex` vs `RwLock`, atomics and `Ordering`, shared state vs message passing
- `05-async.md` — the `Future` trait, async/await desugaring, cooperative scheduling, drop-is-cancel
- `06-pin.md` — why `Pin` exists, self-referential futures, `Unpin`

## Where it goes next

All of it underpins `04-runtime/`. `05-async.md`'s cancellation semantics in
particular come back as a real bug in `06-proxy/01-upstream.md` (a dropped
future skipping a counter decrement) and in
`08-observability/03-tracing.md` (a span guard held across `.await`).
