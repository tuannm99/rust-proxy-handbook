# HTTP

RFC 9110, methods, status codes, headers, chunked encoding.

## What to learn

### RFC 9110 semantics
RFC 9110 defines HTTP semantics independent of version (9112 for /1.1 wire
format, 9113 for /2, 9114 for /3). It's where "what does GET actually
promise" (safe, idempotent, cacheable) and "what does a 3xx mean" live.
Read it as the contract your proxy must not violate when it rewrites or
forwards a request — e.g. forwarding a POST retry when the server never
confirmed idempotency breaks the contract.

### Methods and idempotency
GET/HEAD/PUT/DELETE are idempotent (repeating has the same effect as doing
it once); POST/PATCH generally aren't. This is the deciding factor for
whether your proxy's retry logic (`06-proxy/retry.md`) is safe to apply
automatically or needs an explicit opt-in/idempotency key.

### Status codes a proxy actually generates
Most status codes a proxy returns are about the proxy layer itself, not the
backend: `502 Bad Gateway` (upstream unreachable/invalid response), `503
Service Unavailable` (no healthy upstream, or deliberate load-shedding),
`504 Gateway Timeout` (upstream too slow), `429 Too Many Requests` (rate
limit, see `07-security/ratelimit.md`). Returning `500` for these is a
common beginner mistake — it hides whether the failure was your proxy's or
the backend's.

### Headers: hop-by-hop vs end-to-end
`Connection`, `Keep-Alive`, `Transfer-Encoding`, `TE`, `Upgrade` are
hop-by-hop — a proxy must strip/regenerate them per hop, never blindly
forward them. Everything else is end-to-end and should pass through mostly
untouched (aside from `X-Forwarded-*`/`Forwarded` additions). Forwarding a
hop-by-hop header verbatim to the next hop is a classic proxy bug.

```rust
const HOP_BY_HOP: &[&str] = &[
    "connection", "keep-alive", "proxy-authenticate",
    "proxy-authorization", "te", "trailers",
    "transfer-encoding", "upgrade",
];
```

### Chunked transfer encoding and message framing
When `Content-Length` isn't known upfront, `Transfer-Encoding: chunked`
frames the body as a series of `<hex-size>\r\n<data>\r\n` chunks ending in a
zero-size chunk. A message must not specify both `Content-Length` and
`Transfer-Encoding: chunked` — RFC 9112 says a recipient must reject or
normalize that ambiguity. This is exactly the ambiguity request-smuggling
attacks exploit when a front-end and back-end parser disagree on which
header wins — see `07-security/request-smuggling.md`.

## Practice

1. Use `curl -v` against a real server and identify which response headers
   are hop-by-hop vs end-to-end from the raw wire output.
2. Send a request with both `Content-Length` and
   `Transfer-Encoding: chunked` to a test server you control and observe
   how it's rejected (or isn't — try more than one HTTP library).
3. In `milestones/02-http`, implement correct hop-by-hop header
   stripping for both the request and response path.
4. In `milestones/03-reverse-proxy`, return `502`/`503`/`504`
   distinctly for "upstream refused connection", "no healthy upstream", and
   "upstream timed out" respectively.
5. Read `05-http-stack/parser.md` and `07-security/request-smuggling.md` to
   connect this file's framing discussion to how a parser must enforce it.
