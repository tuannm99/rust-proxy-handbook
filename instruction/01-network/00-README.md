# Network

Phase 1 of the learning path. The protocols a proxy speaks, from the
socket up to HTTP/3 — read before `02-linux/` and `05-http-stack/`, which
assume you know what a connection and a request actually are.

## Files

- `01-socket.md` — bind/listen/accept, socket options, `SO_REUSEADDR`
- `02-tcp.md` — handshake, byte-stream framing, short reads and writes
- `03-dns.md` — resolution, TTLs, resolver caching, and why a long-lived process must re-resolve
- `04-http.md` — HTTP/1.1 semantics: methods, status codes, which headers a proxy must rewrite
- `05-http2.md` — stream lifecycle, HPACK state, flow control, Rapid Reset
- `06-http3.md` — QUIC, UDP demux, QPACK, congestion cost, Alt-Svc
- `07-tls.md` — handshake, SNI, ALPN, session resumption
- `08-proxy-protocol.md` — preserving the real client IP behind another load balancer

## Where it goes next

`01-socket.md` + `02-tcp.md` back `labs/00-tcp-server`; `04-http.md` backs
`labs/02-http-server`; `07-tls.md` backs `labs/07-tls`; `05-http2.md` and
`06-http3.md` back `labs/08-http2` and `labs/09-http3`. The kernel-side
mechanics underneath live in `16-kernel/03-tcp-stack.md`.
