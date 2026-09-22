# Network

Phase 1 of the learning path. The protocols a proxy speaks, from the
socket up to HTTP/3 — read before `02-linux/` and `05-http-stack/`, which
assume you know what a connection and a request actually are.

## Files

**Fundamentals** (start here if "port," "packet," "handshake," "NAT," or
"certificate" don't already have a precise meaning — the one group in
this directory written as a from-scratch primer rather than assuming a
baseline; every file below assumes what these cover):
- `01-fundamentals.md` — the client-server model, and an index to the five files below
- `02-addressing.md` — IP addresses, ports, CIDR notation, NAT (SNAT/DNAT/CGNAT), routing basics
- `03-byte-streams.md` — the byte-stream illusion, packets/segments/datagrams, TCP vs UDP, handshakes as a pattern
- `04-latency-throughput.md` — latency, bandwidth, throughput, RTT, bandwidth-delay product
- `05-proxy-taxonomy.md` — forward vs reverse proxy, L4 vs L7, NAT gateway, API gateway, CDN, sidecar — where `proxy/` sits
- `06-crypto-basics.md` — symmetric/asymmetric encryption, hashing, HMAC, digital signatures, certificates/PKI — the prerequisite `13-tls.md` and `07-security/`'s identity files assume

**Protocols:**
- `07-socket.md` — bind/listen/accept, socket options, `SO_REUSEADDR`
- `08-tcp.md` — handshake, byte-stream framing, short reads and writes
- `09-dns.md` — resolution, TTLs, resolver caching, and why a long-lived process must re-resolve
- `10-http.md` — HTTP/1.1 semantics: methods, status codes, which headers a proxy must rewrite
- `11-http2.md` — stream lifecycle, HPACK state, flow control, Rapid Reset
- `12-http3.md` — QUIC, UDP demux, QPACK, congestion cost, Alt-Svc
- `13-tls.md` — handshake, SNI, ALPN, session resumption
- `14-proxy-protocol.md` — preserving the real client IP behind another load balancer

## Where it goes next

The fundamentals group first if you need it — everything else assumes it.
`07-socket.md` + `08-tcp.md` back `labs/00-tcp-server`; `10-http.md` backs
`labs/02-http-server`; `13-tls.md` backs `labs/07-tls`; `11-http2.md` and
`12-http3.md` back `labs/08-http2` and `labs/09-http3`. The kernel-side
mechanics underneath live in `16-kernel/03-tcp-stack.md`.
