# FFI and ABI

## What to learn

### `repr(C)` and why layout matters at the FFI boundary
Rust's default layout (`repr(Rust)`) is deliberately unspecified — the compiler can reorder fields for better packing, and two structs with identical fields aren't guaranteed the same layout. `#[repr(C)]` pins the layout to C's rules (fields in declared order, standard alignment and padding), which is required whenever a struct crosses an FFI boundary — without it, the two sides can silently disagree about where each field lives.

```rust
#[repr(C)]
struct SockAddrLike { family: u16, port: u16, addr: u32 } // layout now matches C's expectations
```
This is exactly why [`03-rust/03-unsafe.md`](03-unsafe.md)'s `libc` calls (`epoll_ctl`, `setsockopt`) work at all: every struct `libc` hands you is `#[repr(C)]`.

### Calling into C: `extern "C"` and the `libc` crate
`extern "C"` on a function declares (or, for exporting to C, defines) the C calling convention. The `libc` crate is mostly a large set of `extern "C"` declarations plus `#[repr(C)]` structs matching platform headers — you rarely hand-write these for common syscalls because `libc` already has them; you write your own only when binding a library `libc` doesn't cover.

```rust
extern "C" { fn getpid() -> i32; } // hand-written binding, though libc::getpid() already exists
unsafe { let pid = getpid(); }
```
Gotcha: every FFI call is `unsafe`, because the compiler cannot verify the foreign function's contract — does it expect a null-terminated string? does it take ownership of a pointer you pass? is it thread-safe to call from any thread? The `// SAFETY:` comment discipline from [`03-rust/03-unsafe.md`](03-unsafe.md) matters more here, not less, since the invariant lives in someone else's documentation with no borrow checker to cross-check it.

### Ownership across the FFI boundary
The hardest FFI question is always "who frees this, and when." If a C function returns a heap pointer, does it expect you to call its matching free function, or does Rust own it now? Getting this wrong is a double-free or a leak, and neither shows up until it does. The standard fix is a Rust wrapper type whose `Drop` impl calls the matching C free function exactly once, enforced by the type system on the Rust side even though the C side has no such enforcement.

```rust
struct CBuf(*mut u8);
impl Drop for CBuf {
    fn drop(&mut self) {
        // SAFETY: self.0 was allocated by this library and never freed elsewhere.
        unsafe { free_from_c_lib(self.0); }
    }
}
```

### Generating bindings: `bindgen` and `cbindgen`
`bindgen` generates Rust `extern "C"` declarations from a C header (Rust calling into C); `cbindgen` generates a C header from `#[repr(C)]`/`extern "C"` Rust code (C calling into Rust — relevant if [`proxy`](../../proxy) ever exposes a C-ABI plugin surface instead of a Rust-native `dyn Trait` one, [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md)). Both exist because hand-maintaining bindings in sync with a changing header or API is exactly the tedious, error-prone job a tool should own.

### ABI stability: why Rust plugins are usually `dyn Trait`, not `dylib`
Rust has no stable ABI across compiler versions — a `dylib` built with one rustc version is not guaranteed loadable by a binary built with another. This is the concrete reason [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md)'s in-process plugin system compiles plugins into the same binary as `dyn Trait` objects (or exposes a `#[repr(C)]` C-ABI boundary if dynamic loading is truly required) rather than `dlopen`-ing an arbitrary Rust `.so` — the latter only works reliably if every plugin and the host share the exact same rustc build, which is a fragile operational requirement.

## Practice
1. Write a tiny C function, call it from Rust via a hand-written `extern "C"` block, confirm it works, then break it by mismatching an argument type and observe that this is undefined behavior, not a compile error.
2. Add `#[repr(C)]` to a struct used across an FFI boundary in a small example, remove it, and use `std::mem::size_of` to confirm the layout actually can differ without it.
3. Write a `Drop`-based wrapper around a C-allocated resource (a stand-in "C" function is fine) and confirm double-free/use-after-free is prevented by construction.
4. Read the `libc` crate's source for one syscall binding you've already used by hand (`epoll_ctl`, from [`02-linux/07-epoll.md`](../02-linux/07-epoll.md)'s exercise) and identify its `#[repr(C)]` struct definitions.
5. Explain, referencing [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md), why an ABI-stable ecosystem like C makes `dlopen`-style plugins practical while Rust's lack of a stable ABI pushes toward compiling plugins into the same binary instead.
