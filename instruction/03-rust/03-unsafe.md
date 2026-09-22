# Unsafe Rust

## What to learn

### What `unsafe` actually unlocks
`unsafe` does not disable the borrow checker or type checker — it unlocks
exactly five extra abilities: dereferencing raw pointers, calling `unsafe`
fns (including FFI), implementing `unsafe` traits, mutating a `static`, and
accessing union fields. Everything else about Rust's rules still applies
inside an `unsafe` block. Writing `unsafe` is a promise to the compiler that
*you* have manually verified an invariant it can't check.

```rust
let x = 5;
let p = &x as *const i32;
unsafe {
    println!("{}", *p); // dereferencing a raw pointer requires unsafe
}
```

### The unsafe contract
Every `unsafe fn` has implicit preconditions ("safety invariants") that the
caller must uphold — these belong in a `// SAFETY:` comment at the call site
explaining *why* the invariant holds, not what the code does. A proxy built
for throughput will eventually reach for `unsafe` (custom buffer pools,
FFI to `epoll`/`io_uring` via `libc`) — the discipline of writing the safety
argument down is what keeps it sound as the code changes around it.

```rust
// SAFETY: `len` bytes were just initialized by the read_exact call above,
// and buf.capacity() >= len was checked on the line before it.
unsafe { buf.set_len(len); }
```

### Common unsafe patterns in high-perf networking code
- Raw pointer arithmetic into a byte buffer to avoid bounds-check overhead
  in a hot parsing loop (usually not worth it before profiling proves it).
- FFI into `libc` for `epoll_ctl`/`epoll_wait`, raw socket options
  (`setsockopt` for `SO_REUSEPORT`, `TCP_NODELAY`), or `io_uring` — see
  `02-linux/06-epoll.md`, `02-linux/07-io_uring.md`.
- `Vec::set_len` after writing into spare capacity obtained via
  `spare_capacity_mut`, to avoid zero-initializing a read buffer before a
  `read()` syscall fills it.
- Implementing `unsafe impl Send`/`Sync` for a wrapper type when you've
  manually verified thread-safety that the compiler can't infer (e.g. a
  hand-rolled lock-free ring buffer).

Gotcha: an unsound `unsafe impl Send` on a type that isn't actually safe to
move across threads compiles fine and then produces a data race or UB only
under specific timing — exactly the kind of bug that won't show up in a
single-threaded test but will under load in production.

### Keeping unsafe minimal and sound
Wrap every `unsafe` operation in the smallest possible safe function with a
name and signature that make misuse hard, and keep the invariant check next
to the unsafe code (an `assert!` right before an `unsafe` block that relies
on it is cheap insurance). Run `cargo miri test` on unsafe-heavy modules —
Miri catches a large class of undefined behavior (out-of-bounds access,
invalid pointer provenance, data races) that compiles and "works" under
normal `cargo test`.

## Practice
1. Write a safe wrapper function around `Vec::set_len` that takes a closure
   writing into `spare_capacity_mut` and returns the correctly-lengthened
   `Vec` — never expose `set_len` itself as public API.
2. Install and run `cargo miri test` against a small unsafe buffer-pool
   type; intentionally introduce an out-of-bounds write and confirm Miri
   catches it.
3. In the raw-epoll echo server from `02-linux/06-epoll.md`'s exercise, identify every `unsafe` call you need (socket
   creation, `epoll_ctl`, `epoll_wait` via `libc`) and write a `// SAFETY:`
   comment for each before running the code.
4. Find (via docs.rs or source) one real `unsafe impl Send` in a crate you
   depend on (tokio or hyper) and explain in your own words why it's sound.
