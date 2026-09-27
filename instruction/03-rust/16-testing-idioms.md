# Testing Idioms

## What to learn

### Unit tests: `#[cfg(test)] mod tests`
The standard idiom is a `mod tests` at the bottom of the same file, gated `#[cfg(test)]` so it only compiles for `cargo test`, with access to private items in that module — impossible from outside the crate. This is why Rust unit tests live next to the code rather than in a mirrored test-file tree the way many languages do.

```rust
fn parse_content_length(s: &str) -> Option<u64> { s.trim().parse().ok() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_negative() {
        assert_eq!(parse_content_length("-1"), None);
    }
}
```

### Integration tests: the `tests/` directory
Each file directly under `tests/` compiles as its own crate that links against your library's `pub` API only — it exercises the crate the way an external consumer would, catching "compiles fine inside the crate, but the public API can't actually do this" gaps that unit tests miss. Gotcha: shared setup code needs to live in a submodule (`tests/common/mod.rs`), not a top-level `tests/common.rs`, or cargo treats it as its own test binary with zero tests in it and warns.

### Doctests: examples that must keep compiling
A fenced Rust block inside a `///` doc comment is compiled and run as a test by `cargo test`. This is the one kind of Rust test that guarantees your documentation's examples never silently rot — a common real bug in other languages simply doesn't compile-check here.

````rust
/// Parses a CIDR prefix length.
///
/// ```
/// assert_eq!(my_crate::prefix_len("/24"), Some(24));
/// ```
pub fn prefix_len(s: &str) -> Option<u8> { s.strip_prefix('/')?.parse().ok() }
````
Gotcha: a doctest that "just runs" without asserting anything gives false confidence — write doctests that actually check a value, not ones that only prove the code doesn't panic.

### Mocking via traits, not a mocking framework
Rust's idiomatic mock is a second, test-only implementation of the same trait the production code already depends on — no framework required, because the trait boundary already is the seam. Designing for testability means depending on `dyn UpstreamPool` (or a generic `P: UpstreamPool`) in the code under test, not a concrete `HyperUpstreamPool` — the same dispatch decision as [`03-rust/07-traits-and-generics.md`](07-traits-and-generics.md), applied for a different reason.

```rust
trait Clock { fn now(&self) -> std::time::Instant; }
struct FixedClock(std::time::Instant);
impl Clock for FixedClock { fn now(&self) -> std::time::Instant { self.0 } }
// production code takes `impl Clock` / `Arc<dyn Clock>`; tests inject FixedClock
```
Gotcha: don't introduce a trait purely to make something mockable if it has exactly one real implementation and will never have a second — that's premature abstraction. Reach for it when the thing you're mocking is inherently non-deterministic in tests (time, network, randomness), not as a default pattern.

### Table-driven tests
A list of `(input, expected)` pairs looped over in one `#[test]` function keeps a large case list readable and turns adding a new case into a one-line diff — the idiomatic Rust replacement for writing N nearly-identical test functions.

```rust
#[test]
fn parses_hop_by_hop_headers() {
    let cases = [("Connection", true), ("Content-Type", false)];
    for (header, expected) in cases {
        assert_eq!(is_hop_by_hop(header), expected, "header={header}");
    }
}
```
Gotcha: always include the failing row's actual input in the assertion message (the `"header={header}"` above) — without it, a table-driven test failure tells you *that* something broke, not *which* row.

### Property-based testing with `proptest`
Instead of hand-picking example inputs, `proptest` generates hundreds of random inputs matching a shape you describe and asserts an invariant holds for all of them, automatically shrinking a failing case down to the smallest input that still fails. This matters most for parsers and encoders ([`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md), [`labs/01-http-parser`](../../labs/01-http-parser)), where the interesting bugs live in inputs nobody would have thought to write by hand.

```rust
proptest::proptest! {
    #[test]
    fn roundtrips_through_encode_decode(n in 0u64..u64::MAX) {
        let encoded = encode_varint(n);
        prop_assert_eq!(decode_varint(&encoded), Some(n));
    }
}
```

### Benchmarking with `criterion`
Nightly-only `#[bench]` isn't the practical default; `criterion` runs a benchmark enough times to get a statistically meaningful measurement and tracks regressions against the previous run — it's what you reach for before claiming a change is "faster." An ad-hoc `Instant::now()` timing wrapped around a loop is too noisy to trust for anything beyond a rough sanity check.

## Practice
1. Add a `#[cfg(test)] mod tests` to a private parsing function in [`labs/01-http-parser`](../../labs/01-http-parser) and write a test that exercises a private helper directly — confirm it wouldn't compile from outside the crate.
2. Write an integration test in `tests/` for [`labs/03-router`](../../labs/03-router) that only uses the crate's `pub` API, and deliberately try to reach a private field from it to confirm the compiler stops you.
3. Write a doctest for one public function with a real `assert_eq!` in it, then break the function and confirm `cargo test` fails on the doctest.
4. Define a `Clock` trait (or similar non-deterministic dependency) in [`labs/11-rate-limit`](../../labs/11-rate-limit), inject a `FixedClock` in tests, and write a test that would be flaky without it.
5. Write a `proptest` round-trip test for one encode/decode pair in your workspace (a header value normalizer, a varint encoder) and let it find an edge case you didn't think of by hand.
6. Set up `criterion` for one hot-path function (a routing lookup, a header parse) and record a baseline before making a performance change, so you can prove the change actually helped.
