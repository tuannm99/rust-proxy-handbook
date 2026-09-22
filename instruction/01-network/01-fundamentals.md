# Networking Fundamentals

## Status: the one exception to "not a tutorial"

Every other file in this handbook assumes you already have the baseline
and only teaches the proxy-relevant angle — the way `03-rust/` assumes you
know Rust syntax and only deepens ownership/async/unsafe. This file and
its five siblings below are different on purpose: networking fundamentals
aren't something most readers already have the way Rust syntax often is,
and `07-socket.md`/`08-tcp.md`/`09-dns.md` are unreadable without them.
Start here if terms like "port," "packet," "handshake," or "NAT" don't
already have a precise meaning — skip the whole group if they do.

## What to learn

### Two processes, talking over bytes
Strip away every acronym and networking is this: two programs, possibly
on different machines, exchanging bytes over a wire (or radio). One
listens, one connects. Everything else in this directory — TCP, TLS,
HTTP, DNS — is a set of rules layered on top of "send bytes, receive
bytes" so that two programs written independently, on different
computers, agree on what those bytes mean.

`proxy/` is, structurally, just a program that sits in the middle: it's
the "server" to whoever connects to it, and the "client" to whoever it
connects to next. Every protocol file in this directory describes
behavior from one or both of those two roles.

### The six pieces, and where each lives
This introductory layer is split into six short files rather than one
long one, because each piece is a genuinely separate idea and you'll want
to jump back to individual ones later rather than re-reading a wall of
text:

- **`02-addressing.md`** — how a host and a process on it get identified:
  IP addresses, ports, CIDR notation, and NAT (why the address a packet
  arrives with often isn't the address it was sent from).
- **`03-byte-streams.md`** — what TCP actually hands your program (a
  stream, not messages), TCP vs UDP, and handshakes as a recurring
  pattern.
- **`04-latency-throughput.md`** — four numbers people conflate: latency,
  bandwidth, throughput, RTT — and why a "fast" connection can still feel
  slow.
- **`05-proxy-taxonomy.md`** — forward proxy vs reverse proxy vs NAT
  gateway vs load balancer vs L4 vs L7. This repo builds one specific
  point in that space, and this file is where "which one, and why" gets
  answered explicitly.
- **`06-crypto-basics.md`** — symmetric vs asymmetric encryption,
  hashing, HMAC, digital signatures, certificates/PKI. Not cryptography
  as a field — just enough that `13-tls.md`'s handshake and
  `07-security/02-jwt.md`'s signatures stop being magic.

Read them in that order once; after that, treat each as a standalone
lookup.

## Practice
1. Run `ss -tlnp` (or `netstat -tlnp`) on your own machine and identify
   every listening port and which process owns it — confirm you can
   explain, for at least three of them, why that program picked that
   port.
2. Run `curl -v http://example.com` and identify, in the output, where
   the TCP handshake happens, where the HTTP request is sent, and where
   the response headers vs body are — curl labels each phase.
3. Read the five sibling files in order, then come back here and explain,
   in one sentence each: what a socket is, why TCP feels like a file,
   what NAT does to a source address, and which proxy category
   `labs/05-reverse-proxy` falls into.
