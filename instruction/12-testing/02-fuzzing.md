# Fuzzing

## What to learn
### cargo-fuzz / AFL basics
`cargo-fuzz` wraps libFuzzer: it compiles your code with sanitizers and
coverage instrumentation, then mutates a byte-string input, feeding it into
a `fuzz_target!(|data: &[u8]| { ... })` function you write, guided by which
mutations reach new code paths. AFL works similarly but as an external
binary-instrumentation tool, useful when you can't easily link libFuzzer.
For this handbook, `cargo-fuzz` is the more idiomatic choice since
everything is a normal Cargo workspace.

### Fuzzing the hand-rolled parser
[`labs/01-http-parser`](../../labs/01-http-parser) is exactly the kind of code fuzzing is built for:
byte-level input, non-trivial state machine (headers, chunked encoding,
Content-Length), and a real history of security bugs (request smuggling)
coming from exactly this class of parser disagreeing with another parser
about ambiguous input. Wrap your parser's entry point in a
`fuzz_target!(|data: &[u8]| { let _ = parse_request(data); })` and let it
run — a parser should never panic or hang on *any* byte string, even
garbage.

### Corpus-based fuzzing vs property testing
Fuzzing (cargo-fuzz/AFL) explores raw bytes guided by coverage feedback and
is best at finding crashes/panics/hangs on malformed input you didn't think
of. Property testing (`proptest`) instead generates *structured* inputs
(e.g. "a valid request line + 0-10 valid headers + optional body") and
checks a property holds (e.g. "parse(serialize(req)) == req") — better for
catching logic bugs in well-formed input than for finding parser crashes.
Use both: proptest for round-trip correctness, cargo-fuzz for "never panics
on anything."

### Running cargo-fuzz for real
The mechanics that trip people up the first time:

- **Nightly toolchain.** libFuzzer needs sanitizer flags that only
  nightly accepts, so every command is `cargo +nightly fuzz ...`. Install
  the tool once with `cargo install cargo-fuzz`.
- **The code under test must be a library.** A fuzz target is a separate
  crate that *depends on* yours, and it can't import from a `main.rs`.
  If your parser lives in `src/main.rs`, move it into `src/lib.rs` and
  keep `main.rs` as a thin binary that calls it.
- **Layout.** `cargo fuzz init` (run inside the crate) creates `fuzz/`
  with its own `Cargo.toml` and `fuzz/fuzz_targets/<name>.rs`. That
  `Cargo.toml` carries an empty `[workspace]` table so it stays *out* of
  the repo's workspace. Leave it there, or `cargo` complains the fuzz
  crate is in a workspace that doesn't list it.
- **Running with a time limit.** Arguments after `--` go to libFuzzer:
  `cargo +nightly fuzz run <target> -- -max_total_time=1800` runs 30
  minutes. `-max_len=8192` caps input size, and `-timeout=5` makes any
  single input that runs longer than 5 seconds count as a hang, which is
  a finding too (an infinite loop on some byte string).
- **Corpus and crashes.** Interesting inputs accumulate in
  `fuzz/corpus/<target>/`. Put hand-written seed files there before the
  first run. A crash writes the input to `fuzz/artifacts/<target>/`, and
  `cargo +nightly fuzz run <target> <artifact-file>` replays just that
  input so you can debug it. `cargo +nightly fuzz tmin <target> <file>`
  shrinks it to the smallest input that still crashes.
- **Dictionaries.** A text file of tokens like `"Content-Length:"`,
  `"Transfer-Encoding:"`, `"chunked"`, `"\x0d\x0a"` (the dictionary format escapes bytes as `\xNN`) passed with `-dict=<file>`
  lets the mutator splice real protocol words in, instead of waiting to
  guess them one byte at a time.

Gotcha: "no panic" is the weakest property a target can check. A target
can also `assert!` an invariant on every input, and the fuzzer then hunts
for a counterexample. For a parser, a strong one is the split-point
property from [`labs/01-http-parser`](../../labs/01-http-parser): parsing the bytes whole and parsing
them in two pieces at any offset must give the same result. The fuzzer
chooses both the bytes and the offset.

## Practice
1. Add a `fuzz/` directory to [`labs/01-http-parser`](../../labs/01-http-parser) with `cargo fuzz init`
   and a target that calls your parser on raw bytes.
2. Run it for a few minutes and fix any panic it finds (index out of
   bounds on truncated input is the classic first crash).
3. Seed the fuzz corpus with real captured HTTP requests (from `curl -v`
   output) so the fuzzer starts from valid structure instead of random
   bytes.
4. Write a `proptest` that generates a valid request (method, path, a
   handful of headers, optional Content-Length body) and asserts your
   parser extracts the same method/path/headers/body back out.
5. Feed your parser two semantically different but superficially similar
   inputs (e.g. both a `Content-Length` and a `Transfer-Encoding: chunked`
   header) and confirm it picks one deterministically and rejects the
   ambiguity rather than guessing — tie back to [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md).
