# HTTP

RFC 9110 semantics: the shape of a message, methods, status codes, headers, the
state HTTP carries (cookies, validators), and the rules a proxy must keep.

## What to learn

### RFC 9110 semantics, and the version split
RFC 9110 defines HTTP semantics independent of version (9112 for the /1.1
wire format, 9113 for /2, 9114 for /3). It's where "what does GET actually
promise" (safe, idempotent, cacheable) and "what does a 3xx mean" live. Read
it as the contract your proxy must not violate when it rewrites or forwards a
request — e.g. forwarding a POST retry when the server never confirmed
idempotency breaks the contract. The versions differ in *how bytes are
framed*, not in what a request means; a proxy routinely accepts HTTP/2 from
clients and speaks HTTP/1.1 to upstreams, translating between framings while
preserving semantics ([`17-http2.md`](17-http2.md)).

### A message is a start line, headers, and an optional body
```text
GET /search?q=rust HTTP/1.1\r\n             <- request line: method, target, version
Host: example.com\r\n                       <- headers: Name: value
Accept: text/html\r\n
\r\n                                        <- blank line ends the header section
(body, if any)
```
```text
HTTP/1.1 200 OK\r\n                         <- status line: version, code, reason
Content-Type: text/html\r\n
Content-Length: 1234\r\n
\r\n
<1234 bytes>
```
Headers are case-insensitive names with ordered, possibly repeated values.
The body is delimited by `Content-Length`, by chunked coding, or — for a
response — by connection close ([`16-http1-wire-format.md`](16-http1-wire-format.md) has the exact rules
and their order). HTTP is **stateless**: each request carries everything
needed to answer it, which is what lets a proxy send consecutive requests to
different backends. The state that exists (sessions, logins) is carried in
headers such as `Cookie` and `Authorization`, below.

### Targets, `Host`, and virtual hosting
The request *target* has four forms. **origin-form** (`/search?q=rust`) is the
normal one sent to a server. **absolute-form** (`GET http://example.com/x`) is
what a client sends to a **forward** proxy. **authority-form**
(`CONNECT example.com:443`) is used only by `CONNECT`, and **asterisk-form**
(`OPTIONS *`) addresses the server as a whole. A URL splits as
`scheme://host:port/path?query#fragment`; the **fragment never reaches the
server**.

Because many sites share one IP and port, HTTP/1.1 *requires* a `Host` header
(HTTP/2 uses `:authority`) naming which site the request is for — this is
**virtual hosting**, and the basis of [`05-http-stack/12-vhost-routing.md`](../05-http-stack/12-vhost-routing.md). A
request with no `Host`, or two conflicting ones, must be rejected. A reverse
proxy decides routing from `Host` + path, and decides what `Host` to send to
the upstream (preserve the client's, or rewrite to the backend's) — a choice
backends often depend on.

### Methods, safety, and idempotency
`GET`/`HEAD`/`OPTIONS` are **safe** (read-only by contract). `GET`/`HEAD`/
`PUT`/`DELETE`/`OPTIONS` are **idempotent** (repeating has the same effect as
doing it once); `POST`/`PATCH` generally aren't. This is the deciding factor
for whether your proxy's retry logic ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)) is safe to apply
automatically or needs an explicit opt-in/idempotency key. `CONNECT` asks the
proxy to open a raw TCP tunnel to `host:port` and then relay bytes blindly (how
HTTPS passes through a forward proxy; the proxy sees only the destination, not
the content). `Upgrade` switches the connection to another protocol
(WebSocket, [`05-http-stack/10-websocket.md`](../05-http-stack/10-websocket.md)).

### Status codes: the classes, and the ones that bite
`1xx` informational (`100 Continue`, `101 Switching Protocols`), `2xx` success,
`3xx` redirect, `4xx` the client erred, `5xx` the server erred. Worth knowing
exactly:

- `200` OK; `201` Created; `204` No Content (no body, ever); `206` Partial
  Content (answer to a `Range` request).
- **Redirects**: `301`/`302` are ambiguous about whether a `POST` becomes a
  `GET`; the precise ones are `307` (temporary) and `308` (permanent), which
  **preserve the method and body**. `304 Not Modified` is not a redirect — it
  means "your cached copy is still good."
- `400` malformed request; `401` unauthenticated (send credentials); `403`
  authenticated but forbidden; `404`; `408` request took too long to arrive;
  `413` body too large; `414` URI too long; `431` headers too large; `429` too
  many requests.
- `500` server bug; `501` not implemented; `502`/`503`/`504` below.

Most status codes a proxy *generates* are about the proxy layer itself, not the
backend: `502 Bad Gateway` (upstream unreachable/invalid response), `503
Service Unavailable` (no healthy upstream, or deliberate load-shedding),
`504 Gateway Timeout` (upstream too slow), `429 Too Many Requests` (rate limit,
see [`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)). Returning `500` for these is a common
beginner mistake — it hides whether the failure was your proxy's or the
backend's. Status codes `204`, `304` and every response to `HEAD` carry no body
even if `Content-Length` is present — a proxy that waits for a body that will
never come hangs the connection.

### Validators, conditional requests, and ranges
A response can carry a **validator**: `ETag` (an opaque version tag) and/or
`Last-Modified`. A client revalidating sends `If-None-Match: "<etag>"` or
`If-Modified-Since`; if unchanged, the server answers `304` with no body,
saving the transfer. The same machinery protects writes (`If-Match` stops a
lost update). `Cache-Control` (`max-age`, `no-store`, `private`, ...) says how
long and by whom a response may be reused — a proxy cache's whole rulebook
([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)). `Range: bytes=0-999` asks for part of a
resource; the server replies `206` with `Content-Range`, enabling resumable
downloads and video seeking ([`05-http-stack/06-static.md`](../05-http-stack/06-static.md)). `Vary` lists the
request headers that changed the response (e.g. `Accept-Encoding`), and must be
respected or a cache serves the wrong variant.

### Content negotiation and encoding
`Accept`/`Accept-Language`/`Accept-Encoding` let the client state preferences;
`Content-Type` (with `charset`) and `Content-Encoding: gzip|br|zstd` describe
the body ([`05-http-stack/07-compression.md`](../05-http-stack/07-compression.md)). **`Content-Encoding` is
end-to-end** (the body is compressed as a whole and decompressed by the final
client), whereas **`Transfer-Encoding` is hop-by-hop** (applies to this one
connection only). Confusing them is how a proxy double-compresses or corrupts a
body.

### State on top of a stateless protocol: cookies and credentials
The server sets `Set-Cookie: sid=abc; HttpOnly; Secure; SameSite=Lax;
Max-Age=3600`; the client returns `Cookie: sid=abc` on matching requests. A
proxy must forward multiple `Set-Cookie` headers *separately* (they cannot be
comma-joined — the one header that breaks the "repeated headers fold into a
comma list" rule). `Authorization: Bearer <token>` or `Basic <base64>` carries
credentials ([`07-security/01-auth.md`](../07-security/01-auth.md)); `WWW-Authenticate` is the `401`
challenge. A caching proxy must never serve a response personalized by
`Authorization` or `Cookie` to a different user ([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)).

### Headers: hop-by-hop vs end-to-end
`Connection`, `Keep-Alive`, `Transfer-Encoding`, `TE`, `Upgrade`,
`Proxy-Authenticate` and `Proxy-Authorization` are hop-by-hop — a proxy must
strip/regenerate them per hop, never blindly forward them, and must also strip
any header *named* in `Connection:`. Everything else is end-to-end and should
pass through mostly untouched (aside from `X-Forwarded-*`/`Forwarded`/`Via`
additions). Forwarding a hop-by-hop header verbatim to the next hop is a
classic proxy bug ([`05-http-stack/03-hop-by-hop-headers.md`](../05-http-stack/03-hop-by-hop-headers.md)).

```rust
const HOP_BY_HOP: &[&str] = &[
    "connection", "keep-alive", "proxy-authenticate",
    "proxy-authorization", "te", "trailers",
    "transfer-encoding", "upgrade",
];
```

### Telling the backend who the client was
Once a proxy terminates the connection, the backend sees the *proxy's* IP.
`X-Forwarded-For: client, proxy1` (de-facto) and `Forwarded: for=...;proto=https`
(standard) pass it on; `X-Forwarded-Proto`/`-Host` carry scheme and original
host. These are plain headers **anyone can forge**: a proxy should *append* to a
trusted chain, and **overwrite or drop** the header when it comes from an
untrusted client, or a client can claim any IP to bypass IP filtering or rate
limits ([`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md)). At the TCP level the PROXY protocol
solves the same problem ([`20-proxy-protocol.md`](20-proxy-protocol.md)).

### Chunked transfer encoding and message framing
When `Content-Length` isn't known upfront, `Transfer-Encoding: chunked` frames
the body as a series of `<hex-size>\r\n<data>\r\n` chunks ending in a zero-size
chunk. A message must not specify both `Content-Length` and
`Transfer-Encoding: chunked` — RFC 9112 says a recipient must reject or
normalize that ambiguity (the exact grammar and rule order are in
[`16-http1-wire-format.md`](16-http1-wire-format.md)). This is exactly the ambiguity request-smuggling
attacks exploit when a front-end and back-end parser disagree on which header
wins — see [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md).

### `Expect: 100-continue` and early responses
A client about to send a large body may send `Expect: 100-continue` and wait for
an interim `100 Continue` before transmitting, so a server can reject on headers
alone (`401`, `413`) without receiving gigabytes. A proxy must forward the
`Expect` handshake correctly (or answer the `100` itself) and be ready for the
server to respond **before the request body has been fully read** — it must not
assume "read request, then write response".

## Practice

1. Use `curl -v` against a real server and identify, from the raw wire output,
   which response headers are hop-by-hop vs end-to-end, and the request/status
   line, header block and body boundary. Then `printf` the same request to
   `nc` by hand and compare.
2. Send a request with both `Content-Length` and `Transfer-Encoding: chunked`
   to a test server you control and observe how it's rejected (or isn't — try
   more than one HTTP library).
3. Against any static-file server that supports validators and ranges (nginx,
   or a file served from a public CDN), fetch the file with `curl -sv -o
   /dev/null`, copy its `ETag`, then repeat with `-H 'If-None-Match: "<etag>"'`
   to get a `304`, and with `curl -r 0-99` to get a `206`; read
   `Content-Range` and note the body sizes.
4. Request `curl -v --http1.1 http://example.com/` and a `HEAD` for the same
   URL; compare headers, and confirm the `HEAD` response has `Content-Length`
   but no body.
5. In [`labs/02-http-server`](../../labs/02-http-server), implement correct hop-by-hop header stripping
   for both the request and response path, and reject a request with no
   `Host` or with duplicate conflicting `Host` headers.
6. In [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), return `502`/`503`/`504` distinctly for
   "upstream refused connection", "no healthy upstream", and "upstream timed
   out" respectively, append `X-Forwarded-For` while dropping a spoofed
   incoming one, and read [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) and
   [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md) to connect the framing discussion
   to how a parser must enforce it.
