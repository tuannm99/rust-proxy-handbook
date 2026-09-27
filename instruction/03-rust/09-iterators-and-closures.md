# Iterators and Closures

## What to learn

### Iterators are lazy — that's the zero-cost part
`.iter().map(f).filter(g)` builds a chain of adapter structs that do nothing until something drives them (`.collect()`, a `for` loop, a direct `.next()`). The compiler typically inlines and fuses the whole chain into a loop with no allocation for the adapters themselves — this is what "zero-cost abstraction" means in practice: the chained, declarative code compiles down to roughly what a hand-written loop would.

```rust
let hot_upstreams: Vec<_> = pool.iter()
    .filter(|u| u.healthy())
    .map(|u| u.addr)
    .collect(); // filter + map run in one pass; only .collect() allocates
```
Gotcha: `.collect::<Vec<_>>()` allocates; chaining several adapters and collecting once at the end is both more idiomatic and cheaper than collecting after each step.

### `Fn` / `FnMut` / `FnOnce` — closures are structs
A closure desugars to an anonymous struct capturing its environment, plus a generated call method. `Fn` is callable via `&self` (repeatable, non-mutating capture), `FnMut` via `&mut self` (repeatable, mutating), `FnOnce` via `self` (consumes captures — callable exactly once). Because `Fn: FnMut: FnOnce`, any `Fn` closure also satisfies an `FnMut`/`FnOnce` bound, but not the reverse.

```rust
let counter = std::sync::atomic::AtomicUsize::new(0);
let record_hit = || counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed); // Fn: shared-ref capture, callable many times
```
Gotcha: a closure that must run more than once (a per-connection handler mapped over many connections) needs `Fn`/`FnMut`, but if it *moves* a non-`Clone` resource into itself it degrades to `FnOnce` and won't compile where `Fn` is required — the usual fix is cloning an `Arc` into the closure instead of moving the original value.

### `impl Trait` return position vs `Box<dyn Fn>`
Returning `impl Fn(...) -> ...` gives a concrete, unnamed, monomorphized closure type at zero cost — but every function returning `impl Fn` with an identical signature still returns its *own* distinct type. `Box<dyn Fn(...)>` erases the type, which is what you need to store heterogeneous closures in one collection (a `Vec` of route handlers — see `05-http-stack/03-router.md`).

```rust
fn make_key_extractor(header: &'static str) -> impl Fn(&Request) -> Option<&str> {
    move |req| req.headers().get(header)?.to_str().ok()
}
```

### Iterator adapters that matter on a proxy's hot path
`.peekable()` for lookahead without consuming (useful in a hand-written parser — `01-http-parser`), `.windows()`/`.chunks()` on slices for framing logic, `.try_fold()` for early-exit accumulation under a `Result`, `.zip()` for pairing header name/value iterators. Avoid a `.collect::<Vec<_>>()` immediately followed by `.iter()` again — that round trip through an allocation is usually avoidable by keeping the chain lazy one step longer.

## Practice
1. Rewrite a hand-written `for` loop with manual index tracking in your `labs/01-http-parser` (e.g. scanning for `\r\n`) using iterator adapters (`.position()`, `.windows()`, `.split()`), and compare readability against the loop version.
2. Write a closure that captures a cloned `Arc<Mutex<Stats>>` and mutates it on every call from multiple spawned tasks; explain why it needs `Fn` rather than `FnOnce` to be usable that way.
3. Implement `Iterator` by hand for a custom type (e.g. a struct walking chunks of a `Bytes` buffer) and drive it with a plain `for` loop to confirm the `IntoIterator` wiring works.
4. In `labs/03-router`, store route handlers as `Box<dyn Fn(&Request) -> Response + Send + Sync>` in a `Vec`, and explain why `impl Fn` could not be used for that field's type instead.
5. Compare a `.filter().map().collect()` chain against a hand-rolled loop doing the same work by inspecting `--release` codegen (`cargo asm`, or just timing both under load) and confirm they're close.
