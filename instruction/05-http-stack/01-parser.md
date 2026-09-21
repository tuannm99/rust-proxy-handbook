# HTTP Parser

## What to learn

### Request/status line
An HTTP/1.1 request line is `METHOD SP request-target SP HTTP-version CRLF` (RFC 9112 §3). A hand-rolled parser must reject anything that doesn't match exactly — extra spaces, a missing version, or a bare `\n` instead of `\r\n` are all real-world attack surface, not just malformed input to shrug off.

### Header parsing
Headers are `name: value CRLF` lines until an empty line. Naive parsers get bitten by: header name matching being case-insensitive, leading/trailing whitespace in values, duplicate headers (some must be rejected outright, e.g. duplicate `Content-Length`), and obsolete line folding (a continuation line starting with whitespace) which modern parsers should simply reject.

```rust
// sketch: split a raw header line into (name, value), reject folding
fn parse_header_line(line: &str) -> Option<(&str, &str)> {
    let (name, value) = line.split_once(':')?;
    if name.is_empty() || name.contains(' ') {
        return None; // no whitespace before the colon, per RFC 9112 §5.1
    }
    Some((name, value.trim()))
}
```

### Body framing: the part that actually matters
The body's length comes from exactly one of: `Content-Length`, `Transfer-Encoding: chunked`, or "read until connection close" (responses only). If a message has *both* `Content-Length` and `Transfer-Encoding`, or multiple conflicting `Content-Length` values, the message is ambiguous — RFC 9112 §6.3 says to reject it, not "pick one." Getting this wrong is exactly how request smuggling happens (see `07-security/05-request-smuggling.md`).

### Chunked transfer-encoding
Each chunk is `<hex-size>CRLF<data>CRLF`, terminated by a `0`-size chunk and optional trailers. A correct parser must cap chunk-size digits and total decoded size (an attacker can claim an enormous chunk size to exhaust memory), and must not treat trailer headers as equivalent to headers sent before the body.

### Incremental parsing: the constraint that shapes everything
A parser reading from a socket does **not** get a complete request. It gets
whatever `read()` returned — half a header line, two and a half requests,
or three bytes. A parser written as "take a `&str`, return a `Request`" is
untestable against reality and has to be rewritten the moment it meets a
real socket.

The shape that works returns how much it consumed, or asks for more:

```rust
enum ParseStatus<T> {
    Complete { value: T, consumed: usize },
    Partial, // need more bytes; caller must preserve what it has
}
```

This is `httparse`'s API and hyper's internal contract, and it forces the
two properties you need: the parser never blocks on I/O, and the caller
owns the buffer. On `Partial`, the caller reads more into the *same* buffer
and re-parses from the start — no state is carried between calls.

Gotcha: re-parsing from the start on every read is O(n²) if an attacker
feeds one byte at a time (n reads × n bytes rescanned). For a learning
parser that is acceptable and worth measuring; production parsers either
cap header size tightly enough that n² is bounded, or keep an explicit
state machine with a resume offset. Note the interaction with
`07-security/09-ddos.md`: the byte-at-a-time feed *is* the Slowloris attack,
so the read-side data-rate floor and this parser bound defend the same
hole from two sides.

### Buffer management across reads
The caller's loop must distinguish four cases: parsed a complete message
(consume those bytes, keep the remainder — it may hold the *next*
pipelined request), needs more data (read again), buffer full without a
complete message (reject — this is your header size limit doing its job),
and EOF mid-message (reject; a truncated request is not a valid request).

That last one matters: EOF with a partial request is an error, but EOF at a
clean message boundary is a normal connection close. Conflating them
either leaks a half-request into your handler or logs an error on every
well-behaved client disconnect.

Gotcha: after `Complete { consumed }`, the leftover bytes must be moved to
the front of the buffer (or tracked with a read cursor) before the next
read. Forgetting this is the classic pipelining bug — the second request
on a keep-alive connection (`05-http-stack/04-keepalive.md`) is parsed from a
buffer still containing the first one's tail.

### Limits are part of the parser, not a wrapper around it
Every unbounded quantity is a memory-exhaustion vector, and the parser is
the only place with enough context to bound them. At minimum, enforce:
max request line length, max header count, max single header size, max
total header block size, max chunk size, max total body size. Enforce them
*while parsing* — checking after accumulating is checking after the
attacker already won.

Real defaults for calibration: nginx allows 8k request lines and 8k header
buffers; hyper caps headers at 100 by default. Pick numbers, write them
down, and make exceeding them a clean rejection with the right status
(`431 Request Header Fields Too Large`, `413 Content Too Large`) rather
than a panic or a truncation.

### Bytes, not `String`
HTTP is a byte protocol. Header values may legally contain bytes that are
not valid UTF-8, and a URI is bytes until you decide otherwise — so a
parser built on `&str` either rejects valid traffic or hides an unchecked
conversion. Work on `&[u8]`, validate explicitly against the RFC's
character classes (token, VCHAR, obs-text), and convert only where you have
established the bytes are ASCII.

Gotcha: header names are case-insensitive, so comparison must be too, but
`to_lowercase()` per header per request allocates on the hot path. Compare
with `eq_ignore_ascii_case` against a static, and note that
`str::to_lowercase` does full Unicode case folding — wrong *and* slow for
what is defined as an ASCII token. This is where `03-rust/01-ownership.md`'s
borrow-don't-clone discipline pays off: a parsed request should borrow
slices of the input buffer, not own copies of every field.

### Why production code uses hyper instead of a hand parser
`hyper`'s HTTP/1 codec (`h1`) has absorbed years of interop and security fixes for exactly the ambiguities above. Hand-rolling a parser is valuable for learning what those ambiguities *are*, but shipping one in `labs/02-http-server` or later would mean re-discovering every smuggling CVE hyper already fixed.

## Practice
1. In `labs/01-http-parser`, parse the request line and headers from a raw `&[u8]` buffer into a struct that *borrows* from it; reject malformed input rather than best-effort recovering.
2. Make the entry point return `ParseStatus` (complete + consumed count, or partial). Test it by feeding a request one byte at a time and asserting it returns `Partial` until the final byte.
3. Add body framing: support `Content-Length`, then `Transfer-Encoding: chunked`, and explicitly reject a request that specifies both.
4. Write the buffer loop around it: handle leftover bytes after a complete message, and prove pipelining works by parsing two requests out of one buffer in a single read. Then assert EOF mid-request is an error while EOF at a message boundary is not.
5. Enforce the limits from above (header count, sizes, chunk size) and write one rejection test per limit, asserting the correct status code rather than just "an error".
6. Feed your parser adversarial inputs (duplicate `Content-Length` with different values, folded headers, a bare `\n`, a chunk size of `0x` followed by garbage, a header name with a trailing space) and confirm it rejects each one rather than parsing "something."
7. Fuzz `labs/01-http-parser` (see `12-testing/02-fuzzing.md`) against a corpus of real HTTP/1.1 traffic captures and fix any panics.
8. Once you've done this by hand, read `httparse`'s API and `hyper`'s `h1` codec for the same cases, and note what they do differently from your implementation.
