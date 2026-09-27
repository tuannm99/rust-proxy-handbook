# Error Handling

## What to learn

### `Result`, `Option`, and the `?` operator
`Result<T, E>` models recoverable failure, `Option<T>` models a possibly-absent value. `?` propagates `Err`/`None` early and converts the error type via `From` along the way — which is why a function's error type is usually a hand-rolled enum with `impl From<std::io::Error> for MyError` rather than the raw `io::Error`: it unifies every internal error source into one type at the API boundary.

```rust
#[derive(Debug)]
enum ProxyError { Io(std::io::Error), BadUpstream(String) }
impl From<std::io::Error> for ProxyError {
    fn from(e: std::io::Error) -> Self { ProxyError::Io(e) }
}
fn connect() -> Result<std::net::TcpStream, ProxyError> {
    let s = std::net::TcpStream::connect("10.0.0.1:80")?; // io::Error auto-converted via From
    Ok(s)
}
```

### `thiserror` vs `anyhow`
`thiserror` derives `Display`/`Error` for a concrete enum — use it where callers need to match on a specific variant and decide behavior (retry vs. `502` vs. `503`, per `01-network/10-http.md`). `anyhow::Error` is a type-erased, context-chaining "any error" box — use it in binary/glue code (`main.rs`, CLI setup) where you just want to log or bail without callers needing to match a variant. Putting `anyhow` in a library's public API forces every downstream caller to also depend on `anyhow` and lose the ability to match variants — that mismatch is the most common misuse of the two crates.

```rust
// library-ish: concrete, matchable
#[derive(thiserror::Error, Debug)]
enum UpstreamError {
    #[error("upstream unreachable: {0}")]
    Unreachable(#[from] std::io::Error),
    #[error("upstream timed out")]
    Timeout,
}

// binary glue: just propagate + add context
fn load_config(path: &str) -> anyhow::Result<Config> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading config at {path}"))?;
    Ok(toml::from_str(&text)?)
}
```

### Panics are bugs, not expected failure
A panic means an invariant the code assumed was true, wasn't — an out-of-bounds index, an `.unwrap()` on a `None` that "should never happen," a debug-mode integer overflow. A proxy must never let malformed, attacker-controlled input reach an `unwrap()`/`expect()` — that's a remotely triggerable DoS. Reserve panics for programmer errors you'd want a stack trace for in development; return `Result` for anything derived from network input.

Gotcha: a panic inside a `tokio::spawn`ed task does not crash the process — it's caught and surfaces as an `Err` on the task's `JoinHandle` — but if nothing ever awaits that handle, the panic is silently swallowed and the connection just vanishes with no log line. `03-rust/04-sync.md`'s spawn discussion and `08-observability/01-logging.md` both come back to this: always observe a spawned task's panic path.

### Mutex poisoning and `catch_unwind`
A `std::sync::Mutex` becomes "poisoned" if a thread panics while holding the lock — every subsequent `.lock()` returns an `Err` rather than silently continuing with possibly-corrupted state. `parking_lot::Mutex` (a common substitute in proxy code) never poisons, trading that safety net for not having to handle a poisoned-lock error path everywhere, which many codebases decide is the pragmatic choice once critical sections are small and audited. `std::panic::catch_unwind` can catch a panic at an FFI boundary or inside a hand-rolled executor, but it is not a general error-handling tool — most code should let `tokio::spawn`'s per-task panic boundary do this instead of sprinkling `catch_unwind` around.

## Practice
1. Design `ProxyError`/`UpstreamError` enums with `thiserror`, and in `labs/05-reverse-proxy` map each variant to the correct HTTP status (`502`/`503`/`504`) per `01-network/10-http.md`.
2. Write a small `main.rs` using `anyhow::Result` + `.context()` for config loading, and compare the printed error chain against a raw `Result<_, io::Error>` version.
3. Deliberately panic inside a `tokio::spawn`ed task, confirm the process keeps running, then add code that awaits the `JoinHandle` and logs the panic instead of letting it disappear silently.
4. Swap a `std::sync::Mutex` for `parking_lot::Mutex` in a small example; panic while holding the lock in the std version and observe the poisoned-lock `Err`, then note `parking_lot` has no equivalent mechanism at all.
5. Find one `.unwrap()` in your own `labs/01-http-parser` code that runs on attacker-controlled input, and convert it to a `Result` path that returns a parse error instead of panicking.
