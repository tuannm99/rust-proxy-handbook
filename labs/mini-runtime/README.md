# mini-runtime

Small standalone lab: a minimal single-threaded executor and waker built by
hand, no tokio. The point is to understand what `#[tokio::main]` and
`.await` are actually doing.

Handbook references:
- `instruction/03-rust/async.md`, `instruction/03-rust/pin.md`
- `instruction/04-runtime/waker.md`

Run with:

```
cargo run -p mini-runtime
```
