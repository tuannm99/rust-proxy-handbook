# Network

Phase 1 of the learning path. The protocols a proxy speaks, from the
socket up to HTTP/3 — read before [`02-linux/`](../02-linux) and [`05-http-stack/`](../05-http-stack), which
assume you know what a connection and a request actually are.

## How to read this directory

Two ways to use it, depending on where you're starting from:

- **No networking background yet:** read every file below in order,
  [`01-fundamentals.md`](01-fundamentals.md) through [`21-life-of-a-request.md`](21-life-of-a-request.md), doing each file's
  `## Practice` before moving to the next. Treat the whole directory as
  one tutorial — later files assume everything before them, so don't skip
  ahead even if a filename sounds familiar. The last file,
  [`21-life-of-a-request.md`](21-life-of-a-request.md), ties everything together and is the test of
  whether the rest stuck: if you can narrate it with no gaps, you own the
  material.
- **Already comfortable with sockets, TCP, and HTTP:** use this directory
  as a reference instead — jump straight to whichever file covers your
  gap, in any order. The Fundamentals group below is a from-scratch
  primer you likely don't need, and so is the "Below the socket" group
  except [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) if you don't yet read a packet capture
  fluently. [`05-proxy-taxonomy.md`](05-proxy-taxonomy.md) and
  [`06-crypto-basics.md`](06-crypto-basics.md) are worth reading anyway even with a strong
  background, since they're this handbook's own framing (where [`proxy/`](../../proxy)
  sits, and the crypto vocabulary [`19-tls.md`](19-tls.md) assumes) rather than general
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
- [`06-crypto-basics.md`](06-crypto-basics.md) — symmetric/asymmetric encryption, hashing, HMAC, digital signatures, certificates/PKI — the prerequisite [`19-tls.md`](19-tls.md) and [`07-security/`](../07-security)'s identity files assume

**Below the socket** (the layers under TCP — how a packet physically gets
from A to B, and how to *see* it; read before the protocols, since every
Practice below uses these tools):
- [`07-link-layer.md`](07-link-layer.md) — encapsulation, Ethernet, MAC, switches, ARP, MTU, VLANs/bridges/veth
- [`08-ip-and-icmp.md`](08-ip-and-icmp.md) — IPv4 header, TTL, longest-prefix routing, fragmentation and PMTUD, ICMP, IPv6
- [`09-udp.md`](09-udp.md) — datagrams vs streams, what UDP does not give you, amplification, why QUIC is built on it
- [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) — reading `tcpdump`, `ss`, `ip`, `curl -v`, `dig`, `openssl s_client`; the tools every Practice relies on

**Protocols:**
- [`11-socket.md`](11-socket.md) — bind/listen/accept, socket options, `SO_REUSEADDR`
- [`12-tcp.md`](12-tcp.md) — segments and sequence numbers, handshake, state machine, FIN vs RST, `TIME_WAIT`, keepalive
- [`13-tcp-reliability.md`](13-tcp-reliability.md) — retransmission, SACK, head-of-line blocking, flow control, congestion control, Nagle
- [`14-dns.md`](14-dns.md) — hierarchy, what `getaddrinfo` really does, wire format, caching, `ndots`, rebinding
- [`15-http.md`](15-http.md) — HTTP semantics: message shape, `Host`, methods, status codes, validators, cookies, forwarding headers
- [`16-http1-wire-format.md`](16-http1-wire-format.md) — HTTP/1.1 byte grammar: request line, header rules, body-length rules, chunked coding, which status to reject with
- [`17-http2.md`](17-http2.md) — stream lifecycle, HPACK state, flow control, Rapid Reset
- [`18-http3.md`](18-http3.md) — QUIC, UDP demux, QPACK, congestion cost, Alt-Svc
- [`19-tls.md`](19-tls.md) — handshake, SNI, ALPN, session resumption
- [`20-proxy-protocol.md`](20-proxy-protocol.md) — preserving the real client IP behind another load balancer

**Capstone:**
- [`21-life-of-a-request.md`](21-life-of-a-request.md) — one HTTPS request through a reverse proxy, every layer in order, plus a symptom-to-layer debugging table

**Review:**
- [`22-recall-and-review.md`](22-recall-and-review.md) — the twelve-fact skeleton, per-file questions, drawings to redo from memory, predict-then-run experiments; use it on a 1/3/7/21-day schedule so the material sticks

## Where it goes next

The fundamentals group first if you need it — everything else assumes it.
[`11-socket.md`](11-socket.md) + [`12-tcp.md`](12-tcp.md) back [`labs/00-tcp-server`](../../labs/00-tcp-server); [`16-http1-wire-format.md`](16-http1-wire-format.md) backs
[`labs/01-http-parser`](../../labs/01-http-parser); [`15-http.md`](15-http.md) backs [`labs/02-http-server`](../../labs/02-http-server); [`19-tls.md`](19-tls.md) backs [`labs/07-tls`](../../labs/07-tls); [`17-http2.md`](17-http2.md) and
[`18-http3.md`](18-http3.md) back [`labs/08-http2`](../../labs/08-http2) and [`labs/09-http3`](../../labs/09-http3). The OS side of the same
story — processes, the kernel, memory, files, and the Linux networking stack
(netfilter, namespaces) — is [`02-linux/`](../02-linux); the kernel-side mechanics of TCP
live in [`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md), and the classical math behind congestion
control and queueing in [`22-theory/`](../22-theory).
