# 01-http-parser

## Goal

Parse HTTP/1.1 requests by hand from a byte buffer, without hyper or
`httparse`. The point is to feel the edge cases hyper normally hides — and
to make a deliberate, defensible decision about each one.

## Done when

- [ ] Parses a request line and headers into your own request type, and returns an explicit "need more bytes" result for incomplete input rather than an error.
- [ ] Split-point test: for a set of valid requests, feeding the bytes in two pieces at *every* possible split offset yields the same parsed result as feeding them whole.
- [ ] Body framing works for `Content-Length` and for `Transfer-Encoding: chunked` (including chunk extensions and a trailer section).
- [ ] Rejects, with a distinct error per case: both `Content-Length` and `Transfer-Encoding` present; multiple `Content-Length` values that disagree; non-numeric or negative `Content-Length`; malformed chunk sizes. Your choice on obsolete line folding and bare `\n` line endings is written down with a reason ([`instruction/07-security/05-request-smuggling.md`](../../instruction/07-security/05-request-smuggling.md)).
- [ ] Enforces a maximum header section size and header count, returning an error instead of growing memory without bound.
- [ ] A fuzz target ([`instruction/12-testing/02-fuzzing.md`](../../instruction/12-testing/02-fuzzing.md)) runs for 30 minutes with no panic.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/01-network/11-http1-wire-format.md`](../../instruction/01-network/11-http1-wire-format.md) — the grammar itself: every byte-level rule this lab checks, and which status each rejection earns
- [`instruction/05-http-stack/01-parser.md`](../../instruction/05-http-stack/01-parser.md) — turning that grammar into an incremental parser: buffers, partial input, limits
- [`instruction/07-security/05-request-smuggling.md`](../../instruction/07-security/05-request-smuggling.md)
- [`instruction/03-rust/02-lifetimes.md`](../../instruction/03-rust/02-lifetimes.md), [`instruction/03-rust/10-smart-pointers-and-interior-mutability.md`](../../instruction/03-rust/10-smart-pointers-and-interior-mutability.md) — borrowing header slices from the input buffer, `Cow` for the rare rewrite
- [`instruction/03-rust/16-testing-idioms.md`](../../instruction/03-rust/16-testing-idioms.md) — table-driven and property-based tests for the cases above

## After you finish
- Read [`instruction/19-reading-source/hyper/reading-guide.md`](../../instruction/19-reading-source/hyper/reading-guide.md) stops 1-3 and list every case hyper handles differently from you.

## Run

```
cargo run -p http-parser
```
