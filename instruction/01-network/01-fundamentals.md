# Networking Fundamentals

Lesson one of the networking path, and the one every later file leans
on. It assumes nothing: if you already know the layered model, skim it in
twenty minutes; if not, read it twice and do the Practice before moving
on. Networking is a stack of small, separate ideas — the fastest way to
forget them is to learn them as a pile of acronyms, so this file gives you
the *map* first and the later files fill in the territory.

## What to learn

### Two processes, talking over bytes
Strip away every acronym and networking is this: two programs, possibly
on different machines, exchanging bytes over a wire (or radio). One
listens, one connects. Everything else in this directory — TCP, TLS,
HTTP, DNS — is a set of rules layered on top of "send bytes, receive
bytes" so that two programs written independently, on different
computers, agree on what those bytes mean.

[`proxy/`](../../proxy) is, structurally, just a program that sits in the middle: it's
the "server" to whoever connects to it, and the "client" to whoever it
connects to next. Every protocol file in this directory describes
behavior from one or both of those two roles.

### The layered model: the map everything else hangs on
Sending bytes across the planet is too big a problem to solve at once, so it
is split into **layers**. Each layer solves exactly one problem *for the
layer above it*, using only the layer below, and talks to its **peer layer on
the other machine** through a small header — that header is the layer's
whole contract. Two models name the layers:

| OSI # | TCP/IP layer | The one problem it solves | Unit | Addressed by | Examples | Box that works here |
|---|---|---|---|---|---|---|
| 7 (also 6, 5) | **Application** | what the bytes *mean* | message | name / URL | HTTP, DNS, TLS, gRPC | proxy, API gateway |
| 4 | **Transport** | which *process*, and (TCP) reliable and ordered | segment (TCP), datagram (UDP) | port | TCP, UDP, QUIC | L4 load balancer, firewall |
| 3 | **Internet** | which *host*, across many networks | packet | IP address | IP, ICMP | router |
| 2 | **Link** | next *neighbor* on one local network | frame | MAC address | Ethernet, Wi-Fi, ARP | switch |
| 1 | (Link) | bits on a wire, fibre or radio | bit | — | cable, radio | cable, hub |

Remember it as a ladder of ever-wider scope: **frame -> next hop, packet -> right
host, segment -> right process, message -> right meaning.** The OSI model
(7 layers) is the *vocabulary* people use — "L4 load balancer", "L7 proxy" —
while TCP/IP (4 layers) is what is actually implemented; OSI's layers 5 and 6
don't exist as separate software in practice. Treat the model as a map, not a
law: **TLS** sits awkwardly between 4 and 7, **QUIC** does transport and more
over UDP ([`18-http3.md`](18-http3.md)), and **NAT** edits transport-layer ports from a
network-layer box ([`02-addressing.md`](02-addressing.md)). When a protocol "doesn't fit a layer", that is the
model being approximate, not you misunderstanding.

### Encapsulation: one request, wrapped like nested envelopes
Your proxy calls `write(fd, b"GET / HTTP/1.1...")`. Going **down** the stack
each layer wraps what it received from above in its own header (the sender
*encapsulates*); going **up** at the receiver each layer strips its header and
hands the payload up (it *decapsulates*).

```text
application   [ HTTP: GET / HTTP/1.1 ... ]
transport     [ TCP hdr | HTTP ... ]                      <- adds ports, sequence numbers
internet      [ IP hdr  | TCP hdr | HTTP ... ]            <- adds source/dest IP
link          [ Eth hdr | IP hdr | TCP hdr | HTTP ... | FCS ]  <- adds MACs; this is the frame
wire          0101101...
```

Each device reads only as far as its job needs: a **switch** reads the Ethernet
header; a **router** reads up to the IP header, then *throws the Ethernet
header away and builds a new one* for the next link; a **L4 load balancer**
reads up to TCP; a **L7 proxy** reads everything. "How deep does this box look
into the nested envelopes?" is the entire difference between L2, L3, L4 and L7
devices ([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)).

### Hop-by-hop vs end-to-end
Layers 1–3 work **hop by hop**: the MAC addresses change at every router, and
every router makes its own forwarding choice ([`07-link-layer.md`](07-link-layer.md),
[`08-ip-and-icmp.md`](08-ip-and-icmp.md)). Layer 4 and up work **end to end**: a TCP connection's
sequence numbers and retransmissions are between the two endpoints only, and
routers in the middle never look at them. A proxy changes that on purpose: it
*terminates* a transport connection and opens a second one, so it becomes a new
"end" — the client's TCP/TLS session ends at the proxy, and the upstream's
begins there. This single idea explains why proxies can add TLS, pool
connections, retry, and why they must also re-send the client's IP in a header
([`15-http.md`](15-http.md)).

### Debugging bottom-up: which layer is it?
Because each layer depends on the one below, diagnose from the bottom:

1. **Link/IP** — can I reach the host at all? (`ping`, `ip route`, ARP)
2. **Transport** — does the port answer? (`connection refused` = host fine,
   nothing listening; a hang = packets dropped) (`ss`, `tcpdump`)
3. **Application** — does it speak the protocol correctly? (a `403`, a TLS
   alert, a malformed header)

A `403` is never a routing problem and a timeout is rarely an HTTP-header
problem. [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) teaches the tools and
[`21-life-of-a-request.md`](21-life-of-a-request.md) ends with a symptom-to-layer table.

### The six pieces, and where each lives
The next layer of this primer is split into short files rather than one
long one, because each piece is a genuinely separate idea and you'll want
to jump back to individual ones later rather than re-reading a wall of
text:

- **[`02-addressing.md`](02-addressing.md)** — how a host and a process on it get identified:
  IP addresses, ports, CIDR notation, and NAT (why the address a packet
  arrives with often isn't the address it was sent from).
- **[`03-byte-streams.md`](03-byte-streams.md)** — what TCP actually hands your program (a
  stream, not messages), TCP vs UDP, and handshakes as a recurring
  pattern.
- **[`04-latency-throughput.md`](04-latency-throughput.md)** — four numbers people conflate: latency,
  bandwidth, throughput, RTT — and why a "fast" connection can still feel
  slow.
- **[`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)** — forward proxy vs reverse proxy vs NAT
  gateway vs load balancer vs L4 vs L7. This repo builds one specific
  point in that space, and this file is where "which one, and why" gets
  answered explicitly.
- **[`06-crypto-basics.md`](06-crypto-basics.md)** — symmetric vs asymmetric encryption,
  hashing, HMAC, digital signatures, certificates/PKI. Not cryptography
  as a field — just enough that [`19-tls.md`](19-tls.md)'s handshake and
  [`07-security/02-jwt.md`](../07-security/02-jwt.md)'s signatures stop being magic.

Read them in that order once; after that, treat each as a standalone
lookup.

These six explain *what* a connection and a packet are. The next group
([`07-link-layer.md`](07-link-layer.md) through [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md)) goes one level down —
how a packet physically moves, and how to look at it — and
[`21-life-of-a-request.md`](21-life-of-a-request.md) at the end of the directory retells the whole
stack as one request.

## Practice
1. Run `ss -tlnp` (or `netstat -tlnp`) on your own machine and identify
   every listening port and which process owns it — confirm you can
   explain, for at least three of them, why that program picked that
   port.
2. Run `curl -v http://example.com` and identify, in the output, where
   the TCP handshake happens, where the HTTP request is sent, and where
   the response headers vs body are — curl labels each phase.
3. Read files 02–06 in order, then come back here and explain,
   in one sentence each: what a socket is, why TCP feels like a file,
   what NAT does to a source address, and which proxy category
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) falls into.
4. From memory, draw the five-layer stack and, for each of these, list which
   layers' headers are present and which device reads how deep: an ARP request,
   a `ping`, a DNS query over UDP, `curl http://example.com`. Check against the
   table above.
5. Capture one `curl http://example.com` with `sudo tcpdump -i any -n -e -X
   port 80` ([`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md) explains the flags) and point at the
   Ethernet, IP and TCP headers and the HTTP text in the hex dump, naming the
   layer of each.
