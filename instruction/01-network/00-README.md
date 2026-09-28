# Network

Phase 1 of the learning path. The protocols a proxy speaks, from the
socket up to HTTP/3 — read before [`02-linux/`](../02-linux) and [`05-http-stack/`](../05-http-stack), which
assume you know what a connection and a request actually are.

## How to read this directory

Two ways to use it, depending on where you're starting from:

- **No networking background yet:** read every file below in order,
  [`01-fundamentals.md`](01-fundamentals.md) through [`15-proxy-protocol.md`](15-proxy-protocol.md), doing each file's
  `## Practice` before moving to the next. Treat the whole directory as
  one tutorial — later files assume everything before them, so don't skip
  ahead even if a filename sounds familiar.
- **Already comfortable with sockets, TCP, and HTTP:** use this directory
  as a reference instead — jump straight to whichever file covers your
  gap, in any order. The Fundamentals group below is a from-scratch
  primer you likely don't need. [`05-proxy-taxonomy.md`](05-proxy-taxonomy.md) and
  [`06-crypto-basics.md`](06-crypto-basics.md) are worth reading anyway even with a strong
  background, since they're this handbook's own framing (where [`proxy/`](../../proxy)
  sits, and the crypto vocabulary [`14-tls.md`](14-tls.md) assumes) rather than general
  networking knowledge you'd already have from elsewhere.

## Files

**Fundamentals** (start here if "port," "packet," "handshake," "NAT," or
"certificate" don't already have a precise meaning — the one group in
this directory written as a from-scratch primer rather than assuming a
baseline; every file below assumes what these cover):
- [`01-fundamentals.md`](01-fundamentals.md) — the client-server model, and an index to the five files below
- [`02-addressing.md`](02-addressing.md) — IP addresses, ports, CIDR notation, NAT (SNAT/DNAT/CGNAT), routing basics
- [`03-byte-streams.md`](03-byte-streams.md) — the byte-stream illusion, packets/segments/datagrams, TCP vs UDP, handshakes as a pattern
- [`04-latency-throughput.md`](04-latency-throughput.md) — latency, bandwidth, throughput, RTT, bandwidth-delay product
- [`05-proxy-taxonomy.md`](05-proxy-taxonomy.md) — forward vs reverse proxy, L4 vs L7, NAT gateway, API gateway, CDN, sidecar — where [`proxy/`](../../proxy) sits
- [`06-crypto-basics.md`](06-crypto-basics.md) — symmetric/asymmetric encryption, hashing, HMAC, digital signatures, certificates/PKI — the prerequisite [`14-tls.md`](14-tls.md) and [`07-security/`](../07-security)'s identity files assume

**Protocols:**
- [`07-socket.md`](07-socket.md) — bind/listen/accept, socket options, `SO_REUSEADDR`
- [`08-tcp.md`](08-tcp.md) — handshake, byte-stream framing, short reads and writes
- [`09-dns.md`](09-dns.md) — resolution, TTLs, resolver caching, and why a long-lived process must re-resolve
- [`10-http.md`](10-http.md) — HTTP/1.1 semantics: methods, status codes, which headers a proxy must rewrite
- [`11-http1-wire-format.md`](11-http1-wire-format.md) — HTTP/1.1 byte grammar: request line, header rules, body-length rules, chunked coding, which status to reject with
- [`12-http2.md`](12-http2.md) — stream lifecycle, HPACK state, flow control, Rapid Reset
- [`13-http3.md`](13-http3.md) — QUIC, UDP demux, QPACK, congestion cost, Alt-Svc
- [`14-tls.md`](14-tls.md) — handshake, SNI, ALPN, session resumption
- [`15-proxy-protocol.md`](15-proxy-protocol.md) — preserving the real client IP behind another load balancer

## Where it goes next

The fundamentals group first if you need it — everything else assumes it.
[`07-socket.md`](07-socket.md) + [`08-tcp.md`](08-tcp.md) back [`labs/00-tcp-server`](../../labs/00-tcp-server); [`11-http1-wire-format.md`](11-http1-wire-format.md) backs
[`labs/01-http-parser`](../../labs/01-http-parser); [`10-http.md`](10-http.md) backs [`labs/02-http-server`](../../labs/02-http-server); [`14-tls.md`](14-tls.md) backs [`labs/07-tls`](../../labs/07-tls); [`12-http2.md`](12-http2.md) and
[`13-http3.md`](13-http3.md) back [`labs/08-http2`](../../labs/08-http2) and [`labs/09-http3`](../../labs/09-http3). The kernel-side
mechanics underneath live in [`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md).
