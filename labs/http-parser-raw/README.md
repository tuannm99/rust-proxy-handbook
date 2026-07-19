# http-parser-raw

Small standalone lab: parse HTTP/1.1 requests by hand from a byte buffer,
without hyper. The point is to feel the edge cases hyper normally hides.

Handbook references:
- `instruction/05-http-stack/parser.md`
- `instruction/07-security/request-smuggling.md`

Run with:

```
cargo run -p http-parser-raw
```
