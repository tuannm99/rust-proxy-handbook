# 02-http-server

## Goal

A hyper-based HTTP/1.1 and HTTP/2 server that responds correctly to a
small set of routes with correct headers and connection semantics.

## Done when

- [ ] `curl -v` gets the expected status, `Content-Type`, and `Content-Length` for each route, and a 404 for an unknown path.
- [ ] `curl -v http://.../a http://.../b` shows the second request re-using the first connection (keep-alive works), and a request carrying `Connection: close` gets its response and then a closed connection.
- [ ] `curl --http2-prior-knowledge` (cleartext HTTP/2) works against the same server — the auto builder serves both protocols.
- [ ] A handler that returns an error produces a `500` response, not a dropped connection.
- [ ] An idle keep-alive connection is closed after a timeout you chose, rather than held forever ([`instruction/05-http-stack/04-keepalive.md`](../../instruction/05-http-stack/04-keepalive.md)).
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/05-http-stack/01-parser.md`](../../instruction/05-http-stack/01-parser.md) — do [`labs/01-http-parser`](../01-http-parser) first so hyper's API makes sense
- [`instruction/05-http-stack/04-keepalive.md`](../../instruction/05-http-stack/04-keepalive.md) — persistent connections, when a connection can't be reused
- [`instruction/05-http-stack/02-hop-by-hop-headers.md`](../../instruction/05-http-stack/02-hop-by-hop-headers.md) — which headers must not be forwarded
- [`instruction/01-network/10-http.md`](../../instruction/01-network/10-http.md), [`instruction/01-network/11-http2.md`](../../instruction/01-network/11-http2.md) — status codes/headers, h1 vs h2

## After you finish
- Read [`instruction/19-reading-source/hyper/reading-guide.md`](../../instruction/19-reading-source/hyper/reading-guide.md) stops 4-5 — hyper's connection state machine and dispatcher.

## Run

```
cargo run -p http-server
```
