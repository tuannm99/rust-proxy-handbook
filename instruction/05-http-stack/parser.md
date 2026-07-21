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
The body's length comes from exactly one of: `Content-Length`, `Transfer-Encoding: chunked`, or "read until connection close" (responses only). If a message has *both* `Content-Length` and `Transfer-Encoding`, or multiple conflicting `Content-Length` values, the message is ambiguous — RFC 9112 §6.3 says to reject it, not "pick one." Getting this wrong is exactly how request smuggling happens (see `07-security/request-smuggling.md`).

### Chunked transfer-encoding
Each chunk is `<hex-size>CRLF<data>CRLF`, terminated by a `0`-size chunk and optional trailers. A correct parser must cap chunk-size digits and total decoded size (an attacker can claim an enormous chunk size to exhaust memory), and must not treat trailer headers as equivalent to headers sent before the body.

### Why production code uses hyper instead of a hand parser
`hyper`'s HTTP/1 codec (`h1`) has absorbed years of interop and security fixes for exactly the ambiguities above. Hand-rolling a parser is valuable for learning what those ambiguities *are*, but shipping one in `labs/02-http-server` or later would mean re-discovering every smuggling CVE hyper already fixed.

## Practice
1. In `labs/01-http-parser`, parse the request line and headers from a raw byte buffer into a struct; reject malformed input rather than best-effort recovering.
2. Add body framing: support `Content-Length`, then `Transfer-Encoding: chunked`, and explicitly reject a request that specifies both.
3. Feed your parser adversarial inputs (duplicate `Content-Length` with different values, folded headers, a chunk size in scientific-notation-like garbage) and confirm it rejects each one rather than parsing "something."
4. Fuzz `labs/01-http-parser` (see `12-testing/fuzzing.md`) against a corpus of real HTTP/1.1 traffic captures and fix any panics.
5. Once you've done this by hand, read how `hyper`'s `h1` codec handles the same cases and note what it does differently from your implementation.
