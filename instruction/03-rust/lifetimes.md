# Lifetimes

## What to learn

### Lifetime elision and explicit annotations
Lifetimes are compile-time-only labels the borrow checker uses to prove a
reference never outlives what it points to — they add no runtime
representation. The elision rules cover the common cases (one input
reference -> output borrows from it; `&self` methods -> output borrows from
`self`), so you mostly only write `'a` explicitly when a function or struct
ties together *multiple* independent borrows.

```rust
struct RequestView<'a> {
    method: &'a [u8],
    path: &'a [u8],
}

fn view<'a>(buf: &'a [u8]) -> RequestView<'a> { RequestView { method: buf, path: buf } }
```

### Structs holding borrowed data
A struct with a lifetime parameter cannot outlive the data it borrows from —
the compiler enforces this everywhere the struct is used, which is exactly
how a zero-copy `RequestView<'a>` (above) is kept safe: it can never be
returned or stored somewhere that outlives the original read buffer.

Gotcha: this is why zero-copy parsers and self-referential state don't mix.
A struct cannot hold both a buffer and a borrow into that same buffer as
sibling fields — the borrow would need to reference a field of the same
struct it lives in, which Rust's ownership model forbids without indirection
(`Pin`, see `03-rust/pin.md`, or just storing an offset/`Range<usize>`
instead of a `&[u8]`, which is what most production zero-copy parsers do).

### Lifetimes vs async
`.await` points are where the big lifetime headaches show up in a proxy.
An `async fn` desugars to a state machine struct that must hold everything
live across an `.await` — including any borrows. If that struct also needs
to be `'static` (true for anything you `tokio::spawn`), it cannot hold a
borrow of anything shorter-lived than the task itself.

```rust
async fn handle(buf: &[u8]) { /* ... */ } // fine to call and .await inline

// but you CANNOT do:
// tokio::spawn(handle(&local_buf)); // error: `local_buf` does not live long enough
```

The fix is almost always to make the spawned task own its data (`Vec<u8>`,
`Bytes`, or `Arc<T>`) rather than borrow it — see `03-rust/sync.md` for
`Arc`, and `05-http-stack/parser.md` for `bytes::Bytes` (a cheaply-cloneable
owned buffer, the standard fix for this exact problem in the hyper
ecosystem).

### HRTB and trait objects (brief)
`for<'a> Fn(&'a T) -> ...` (higher-ranked trait bounds) show up when you
store a closure or trait object that must work for *any* lifetime the caller
picks, not one fixed lifetime — e.g. a middleware trait whose `handle`
method takes `&Request` with a lifetime chosen per call. You don't need to
write these often, but recognize the syntax when a compiler error mentions
"higher-ranked lifetime error" while building a plugin/middleware system
(`09-architecture/plugin.md`).

## Practice
1. Take the `RequestView<'a>` from ownership.md's practice and make the
   compiler reject a version of your code that tries to return a
   `RequestView` after its source buffer is dropped.
2. Refactor a function that borrows a buffer into one that owns
   `bytes::Bytes` instead, and explain in a comment when each choice is
   correct for a request/response type moving through
   `milestones/02-http`.
3. Reproduce the "does not live long enough" error from a borrow crossing
   `tokio::spawn`, then fix it three different ways: cloning into an owned
   type, wrapping in `Arc`, and restructuring to avoid spawning at all.
4. Write a struct that intentionally cannot compile because it tries to hold
   a buffer and a `&[u8]` slice of itself as sibling fields; read the error
   and connect it to `03-rust/pin.md`.
