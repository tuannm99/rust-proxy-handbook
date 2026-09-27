# Smart Pointers and Interior Mutability

## What to learn

### `Box<T>`: single ownership, on the heap
`Box<T>` is a heap allocation with the same move/borrow rules as any owned value, just backed by heap memory instead of the stack. It shows up for: values too large to move cheaply, recursive types (a type can't contain itself by value, but can contain a `Box` of itself), and trait objects (`Box<dyn Trait>`), since `dyn Trait` has no compile-time-known size.

```rust
struct Recursive { child: Box<Recursive> } // needs Box: without it, the type has infinite size
```

### `Rc<T>` vs `Arc<T>`: shared ownership, single- vs. multi-thread
`Rc<T>` is reference-counted shared ownership for single-threaded code — the count itself isn't atomic, so it's cheaper but not `Send`/`Sync`. `Arc<T>` is the same idea with an atomic count, safe to share across threads. In an async proxy almost everything crosses thread boundaries via `tokio::spawn`, so `Rc` shows up rarely — mostly inside a deliberately single-threaded future on a `current_thread` runtime or `LocalSet` (`04-runtime/03-runtime-config.md`). Defaulting to `Arc` and dropping to `Rc` only when a value is deliberately pinned to one thread is the safer habit.

### `Cell<T>` and `RefCell<T>`: interior mutability, single-threaded
Both let you mutate through a shared reference (`&T`), which the borrow checker otherwise forbids. `Cell<T>` works for `Copy` types via `get`/`set` — no runtime bookkeeping needed, just a plain memory swap. `RefCell<T>` works for anything via `.borrow()`/`.borrow_mut()`, enforcing Rust's aliasing rule (one mutable *xor* many immutable borrows) at runtime instead of compile time, panicking on violation instead of failing to compile.

```rust
use std::cell::RefCell;
let cache: RefCell<std::collections::HashMap<String, String>> = RefCell::new(Default::default());
cache.borrow_mut().insert("k".into(), "v".into()); // fine
// let a = cache.borrow(); let b = cache.borrow_mut(); // panics at runtime: already borrowed
```
Gotcha: holding a `RefCell` borrow across an `.await` point is a common source of "already borrowed" panics in async code, because another task can run — and also try to borrow — while the first is suspended. This is exactly the hazard `Mutex`/`Arc<Mutex<_>>` (`03-rust/04-sync.md`) is designed to make impossible rather than runtime-panicking.

### `Mutex<T>`/`RwLock<T>`: the same spectrum, across threads
Where `RefCell` enforces aliasing at runtime for single-threaded code, `Mutex<T>`/`RwLock<T>` enforce the multi-thread equivalent, blocking instead of panicking on contention — `03-rust/04-sync.md` covers them fully; this file's point is that all four types sit on one spectrum: compile-time-checked (plain `&`/`&mut`), single-thread-runtime-checked (`Cell`/`RefCell`), and multi-thread-blocking (`Mutex`/`RwLock`).

### `Cow<'a, T>`: avoid cloning until you must
`Cow` ("clone on write") holds either a borrowed reference or an owned value, cloning only when mutation is actually required. On a header-normalization path (`07-security/04-normalization.md`), most requests need zero modification — returning `Cow::Borrowed` for the common case and allocating (`Cow::Owned`) only when a header actually needs rewriting avoids an allocation on the hot path for the overwhelming majority of requests.

```rust
fn normalize_header(v: &str) -> std::borrow::Cow<str> {
    if v.bytes().all(|b| !b.is_ascii_uppercase()) {
        std::borrow::Cow::Borrowed(v) // already normalized, zero allocation
    } else {
        std::borrow::Cow::Owned(v.to_ascii_lowercase())
    }
}
```

## Practice
1. Build a small recursive `enum`/`struct` (e.g. a config AST — see `15-parser/03-ast.md`) that needs `Box` to compile; remove the `Box` and read the compiler's "infinite size" error.
2. Try sharing an `Rc<RefCell<T>>` across a `tokio::spawn` boundary and read the resulting `Send` error; fix it by switching to `Arc<Mutex<T>>` (or `Arc<parking_lot::Mutex<T>>`).
3. Deliberately hold a `RefCell` borrow across an `.await` in a small async example to reproduce the "already borrowed" panic, then explain why the equivalent `Mutex` version would have deadlocked instead — and why that isn't actually better. Connect to `03-rust/04-sync.md`.
4. Write a header-normalization function returning `Cow<str>` (in `labs/01-http-parser` or the `07-security` normalization exercise), and assert the already-normalized path never allocates (e.g. `matches!(result, Cow::Borrowed(_))`).
5. Read `Arc::get_mut`'s docs and explain when it succeeds (it needs exclusive access) — connect this to why it's rare to use in a proxy holding `Arc`s shared across many tasks.
