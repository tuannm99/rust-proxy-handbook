# 01-http-parser

## Goal

Small standalone lab: parse HTTP/1.1 requests by hand from a byte buffer,
without hyper. The point is to feel the edge cases hyper normally hides.

## Handbook references
- `instruction/05-http-stack/01-parser.md`
- `instruction/07-security/05-request-smuggling.md`

## Run

```
cargo run -p http-parser
```
