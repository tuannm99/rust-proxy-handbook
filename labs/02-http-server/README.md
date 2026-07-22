# 02-http-server

## Goal

A hyper-based HTTP/1.1 (and HTTP/2) server that responds correctly to a
single route with correct headers and connection semantics. Done means:
`curl` gets correct status codes, keep-alive connections correctly serve
multiple requests without hyper hanging or closing early, and hop-by-hop
headers are handled correctly.

## Handbook references
- `instruction/05-http-stack/parser.md` — do `labs/01-http-parser` first so hyper's API makes sense
- `instruction/05-http-stack/keepalive.md` — persistent connections, when a connection can't be reused
- `instruction/01-network/http.md`, `instruction/01-network/http2.md` — status codes/headers, h1 vs h2

## Run

```
cargo run -p http-server
```
