# CI & Static Tooling

Verification that runs before the proxy ever handles a real request — linting, formatting, and undefined-behavior detection for the workspace itself.

## What to learn
### clippy + rustfmt as a non-optional gate
`cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` are cheap (seconds) and catch a large class of bugs and style drift before anything more expensive runs. `-D warnings` matters specifically — without it, clippy's lints are advisory and get ignored under deadline pressure; with it, a lint failure fails the build the same way a compile error would.

### miri for the unsafe code specifically
`labs/01-http-parser` is exactly the kind of crate most likely to contain real `unsafe` (raw pointers into buffers) — see `03-rust/03-unsafe.md`. `cargo +nightly miri test` runs test code under an interpreter that detects undefined behavior real hardware would silently tolerate: out-of-bounds access, use-after-free, data races, invalid pointer arithmetic. Know its limits: miri can't execute real syscalls, so the raw-epoll exercise's actual `epoll_wait`/`libc` calls (`02-linux/07-epoll.md`) can't run under it directly — miri is for testing the pure-Rust unsafe logic (a hand-rolled buffer pool, pointer arithmetic in the parser) in isolation from the syscalls around it.

```yaml
# .github/workflows/ci.yml (excerpt)
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
  miri:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: rustup +nightly component add miri
      - run: cargo +nightly miri test -p http-parser
```
Gotcha: don't run `cargo miri test` over the *whole* workspace by default — crates that open real sockets (`tcp-server`, `reverse-proxy`, ...) will fail or hang under miri for reasons that have nothing to do with unsafety in your code. Scope it to the crates whose unsafe logic is actually being verified.

### cargo-deny / cargo-audit
Once the workspace depends on network-facing crates (TLS libraries, hyper, tokio), dependency-level auditing matters: `cargo audit` checks the dependency tree against the RustSec advisory database for known vulnerabilities; `cargo deny check` can additionally enforce license policy and ban duplicate/unwanted crate versions. Both are near-instant against a `Cargo.lock` that's already resolved — there's no reason to skip them once the workspace has real dependencies, which it already does (`tokio-rustls` in `proxy`).

### What blocks a commit vs what runs on a schedule
Fast checks (fmt, clippy, unit tests, miri on the small unsafe crates) belong on every push — they're seconds to low-minutes and give immediate feedback. Slow checks (fuzzing corpus regression from `12-testing/02-fuzzing.md`, load tests from `12-testing/01-load-testing.md`, chaos runs from `12-testing/03-chaos.md`) belong on a schedule (nightly, or on release branches) — running a multi-minute load test on every single push just slows down iteration without adding proportional signal for most commits.

## Practice
1. Add a GitHub Actions (or equivalent) workflow that runs `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` on every push; fix whatever clippy flags (should be near-zero in the stub state, but re-run after every exercise you implement).
2. Add a `miri` job scoped to `labs/01-http-parser` specifically (not the whole workspace); confirm it passes on the stub code and re-run it as you implement the unsafe parsing logic.
3. Add `cargo-deny` with a `deny.toml` that at minimum denies known-vulnerable advisories; run it once against the current dependency set and fix anything it flags.
4. Split the workflow into a fast job (fmt/clippy/miri, on every push) and a separate scheduled/manual job for anything from `12-testing/01-load-testing.md` or `12-testing/02-fuzzing.md` that takes more than a minute or two to run.
