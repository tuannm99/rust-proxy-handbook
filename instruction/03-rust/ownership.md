# Ownership

## What to learn

### Move semantics
Every value in Rust has exactly one owner. Assigning or passing a non-`Copy`
value moves it — the old binding becomes invalid at compile time, so there's
no runtime cost and no possibility of use-after-free from a stale alias. This
is what lets a proxy hand a `Vec<u8>` read buffer down through several layers
(parser -> router -> upstream writer) without ever copying it.

```rust
let buf: Vec<u8> = read_request();
let parsed = parse(buf); // buf moved into parse; buf is no longer usable here
```

Gotcha: moving a value *out of* a struct field while a `&mut` borrow of the
whole struct is live doesn't compile — this is exactly the shape you hit when
trying to take ownership of a connection's read buffer while also holding a
mutable reference to the connection for bookkeeping. Split the struct or use
`Option::take`.

### Copy vs Clone
`Copy` types (integers, bools, small fixed-size data) are implicitly
bitwise-duplicated on assignment — no move happens, both bindings stay valid.
`Clone` is an explicit, possibly expensive deep copy you opt into with
`.clone()`. In a hot request path, an accidental `.clone()` on a
multi-kilobyte header map is a real perf bug, not just style — prefer `Arc`
(see `03-rust/sync.md`) or borrowing over cloning buffers per-request.

```rust
#[derive(Clone, Copy)]
struct ConnId(u64); // cheap, Copy is correct

struct Headers(Vec<(String, String)>); // NOT Copy — clone is O(n) and allocates
```

### Borrowing rules
At any point you may have either one `&mut T` or any number of `&T`, never
both — enforced at compile time, zero runtime cost. This is what makes
`unsafe`-free zero-copy parsing possible: a parser can hand out `&[u8]` slices
into the original read buffer instead of allocating substrings, because the
borrow checker guarantees the buffer outlives the slices.

```rust
fn parse_method(buf: &[u8]) -> &[u8] {
    &buf[..buf.iter().position(|&b| b == b' ').unwrap()]
}
```

Gotcha: borrows cannot be held across an `.await` point if the future also
needs to be `Send` and the borrowed data lives on a caller's stack frame that
moves — this is the root cause of many "future cannot be sent between
threads" errors when mixing borrowed slices with async fns. See
`03-rust/lifetimes.md` and `03-rust/async.md`.

### Drop order and RAII
Values are dropped in reverse declaration order at end of scope; struct
fields drop in declaration order. Wrapping a raw resource (socket fd, mmap
region, lock guard) in a type whose `Drop` releases it means "forgetting to
clean up" becomes a compile-time non-issue instead of a runtime leak — this
is the same trick `MutexGuard` and `TcpListener` use.

```rust
struct UpstreamConn {
    id: u64,
    // socket dropped automatically when UpstreamConn drops
}
```

Gotcha: `std::mem::forget` (or a panic during unwind with `catch_unwind`)
skips `Drop` — relevant if you ever hand a raw fd to `libc` code (see
`03-rust/unsafe.md`) and rely on Rust's `Drop` to close it.

## Practice
1. Write a function that takes ownership of a `Vec<u8>` request buffer,
   parses out a method/path/headers view as borrowed slices, and returns a
   struct holding both the buffer and the slices — notice why this needs a
   lifetime parameter (continue in `03-rust/lifetimes.md`).
2. Deliberately trigger and then fix an "use of moved value" error by
   restructuring a function to borrow instead of take ownership.
3. Benchmark (with a quick `std::time::Instant`) cloning a 8KB header map
   1M times vs wrapping it in `Arc` and cloning the `Arc` — confirm the
   difference is real before you believe it.
4. In `milestones/01-echo`, decide whether your per-connection read
   buffer is owned by the task or borrowed from a pool, and justify it in a
   comment.
5. Write a small type with a custom `Drop` impl that prints when it runs;
   confirm the order across nested structs and `Vec<T>` matches your
   prediction.
