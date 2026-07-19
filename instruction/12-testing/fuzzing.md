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
`labs/http-parser-raw` is exactly the kind of code fuzzing is built for:
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

## Practice
1. Add a `fuzz/` directory to `labs/http-parser-raw` with `cargo fuzz init`
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
   ambiguity rather than guessing — tie back to `07-security/request-smuggling.md`.
