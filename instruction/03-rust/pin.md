# Pin

## What to learn

### Why Pin exists: self-referential futures
Desugaring an `async fn` into a state-machine struct (see `03-rust/async.md`)
can produce a struct that borrows from its own fields — e.g. a local
variable in one part of the function borrowed by an `.await` expression
later in the same function, both stored in the same generated struct. Such a
struct must never be moved in memory after those internal references are
set up, because moving it would leave the internal reference pointing at the
old location. `Pin<P>` is a wrapper around a pointer type that upholds
exactly one guarantee: once pinned, the pointee will not move again (for
non-`Unpin` types) — that guarantee is what makes polling a self-referential
future sound.

```rust
// Conceptually, an async fn awaiting across a borrow generates something like:
struct GeneratedFuture<'a> {
    local: String,
    borrow: &'a str, // borrows `local`, a sibling field — self-referential
}
// This shape is only sound to poll if it's guaranteed never to move: hence Pin.
```

### `Unpin`: the common case
Most types are `Unpin` (implemented automatically for anything that doesn't
contain a self-referential structure) — moving them is always fine, and
`Pin<&mut T>` for an `Unpin` `T` behaves just like `&mut T` (you can even get
one back out via `Pin::get_mut`). You only need to think hard about `!Unpin`
types when you're hand-implementing `Future` for something that holds
references into itself — ordinary application code almost never writes
`Pin` by hand; it shows up in `Future::poll`'s signature and in executor
code.

```rust
fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
    // `self` is guaranteed not to move again while pending
    std::task::Poll::Pending
}
```

### `Pin<Box<dyn Future<Output = T>>>`
Boxing a future and pinning the box is the standard way to store a
heap-allocated, dynamically-dispatched future — e.g. a hand-rolled
executor's task queue, or a function returning "some future, don't care
which concrete type" without `async fn` in traits. `Box::pin` allocates on
the heap and immediately pins it, sidestepping any question of the value
moving on the stack afterward.

```rust
struct Task {
    future: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
}

let task = Task { future: Box::pin(async { /* ... */ }) };
```

Gotcha: `Pin` only prevents moving the *pointee*; it says nothing about
thread-safety. A boxed future still needs `+ Send` in its trait object type
if your executor polls tasks from a thread pool (tokio's does) — mixing this
up produces a confusing `Send`-related compiler error that's actually about
missing bounds on the trait object, not about `Pin` itself.

## Practice
1. Write a minimal `!Unpin` self-referential struct (without async) using
   `PhantomPinned`, pin it with `Box::pin`, and observe the compiler reject
   an attempt to move it afterward.
2. In `labs/mini-runtime`, define your `Task` type as
   `Pin<Box<dyn Future<Output = ()> + Send>>` and implement the executor's
   run-queue around it, per the file's TODOs.
3. Explain in your own words (a comment is fine) why `Future::poll` takes
   `self: Pin<&mut Self>` rather than plain `&mut self` — connect it back to
   the self-referential struct shape from `03-rust/async.md`.
4. Read the standard library docs for `Pin::get_mut` and `Pin::new_unchecked`
   and write down, precisely, what invariant `new_unchecked` requires you to
   uphold manually (this is unsafe — see `03-rust/unsafe.md`).
