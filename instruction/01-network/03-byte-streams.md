# Byte Streams, Packets, and Connections

Part of the from-scratch fundamentals series — see `01-network/01-fundamentals.md`
for the full index. This file covers the single most important mental
model for `labs/00-tcp-server` and `labs/01-http-parser`.

## What to learn

### The byte-stream illusion
TCP presents your application with what *looks like* a continuous stream
of bytes — you call `read()`, you get bytes; you call `write()`, bytes go
out. It feels like writing to a file. Underneath, none of that is true at
the network level: the data actually travels as discrete **packets** (at
the IP layer) carrying TCP **segments**, each with its own header, each
routed independently, each possibly arriving in a different order than it
was sent (TCP reassembles them in order before handing bytes to your
program).

The illusion TCP maintains is *ordering* and *completeness* — the bytes
you read are in the order they were written, and none are missing. It
makes **no promise about grouping**. If the sender calls `write()` once
with 10,000 bytes, the receiver might see that arrive as one `read()` of
10,000 bytes, or five `read()`s of 2,000 each, or one `read()` of 3 bytes
followed by a `read()` of the remaining 9,997 — the network, the kernel's
buffers, and timing all influence this, and your application has no
control over it and cannot assume anything about it.

This is exactly why `01-http-parser`'s guide insists on incremental
parsing (`ParseStatus::Partial`) instead of "read the whole request in one
call" — there is no such thing as "the whole request in one call." A
single `read()` returning 3 bytes of a 2,000-byte request is not a bug or
an edge case; it is completely normal TCP behavior that your code must
handle every time, not just on a bad day.

### Packets, segments, datagrams — the same idea, different layers
The terms differ by which layer you're talking about, and it's worth
knowing which is which because you'll see all three:
- **Packet** — the general term, usually meaning an IP packet: an IP
  header plus whatever the layer above put inside it.
- **Segment** — a TCP packet specifically: TCP's header (sequence
  number, ack number, flags, window size) plus a chunk of your stream's
  bytes.
- **Datagram** — a UDP packet: UDP's much smaller header plus one
  complete, self-contained message. Unlike a TCP segment, a UDP datagram
  is not part of a stream — the application gets it whole or not at all.

### Connection-oriented vs connectionless
TCP is **connection-oriented**: before any data flows, both sides
exchange a handshake (see the next section) to agree they're both there
and synchronize state. Both ends then track that connection's state for
its whole lifetime — sequence numbers, unacknowledged data, buffer sizes.
This state is what makes reliability and ordering possible, and it's also
what makes a TCP connection a real, stateful *resource* on both machines
(see `16-kernel/03-tcp-stack.md` for what that state costs at scale, and
`02-addressing.md` for the 4-tuple that identifies it).

UDP is **connectionless**: a datagram just goes out, with no handshake,
no acknowledgment, no guaranteed order, no automatic retransmission. If
you need those properties over UDP, your application layer has to build
them itself. This sounds strictly worse, and for a normal request/response
it usually is — but it's also why **QUIC** (the transport underneath
HTTP/3, see `01-network/12-http3.md`) is built on UDP rather than TCP:
TCP's in-kernel, one-size-fits-all reliability creates head-of-line
blocking that HTTP/2 suffers from at the multiplexed-stream level
(`01-network/11-http2.md`), and QUIC reimplements reliability
*per-stream*, in userspace, specifically to avoid that — something you
can't do on top of TCP because TCP's ordering guarantee applies to the
whole connection, not per logical stream.

### Handshakes: agreeing on state before exchanging data
A "handshake" is any exchange where both sides agree on shared state
before real data flows — you'll meet this word three separate times in
this directory, each a different instance of the same idea:
- **TCP's 3-way handshake** (`08-tcp.md`) agrees on connection state and
  initial sequence numbers.
- **TLS's handshake** (`13-tls.md`) agrees on encryption keys and which
  protocol version/cipher to use.
- **HTTP/1.1's `Upgrade` handshake** (`05-http-stack/09-websocket.md`)
  agrees to stop speaking HTTP and start speaking a different protocol on
  the same connection.

Recognizing "this is a handshake" tells you what to expect: a fixed
back-and-forth exchange, state that both sides must now agree on, and a
failure mode where one side thinks the handshake succeeded and the other
doesn't (which is where a lot of real bugs live).

### Stateful vs stateless, at the application level
The connection-level distinction above (TCP tracks state, UDP doesn't)
has an application-level echo worth naming separately: **stateless**
protocols (plain HTTP/1.1 request/response, at the semantic level — see
`01-network/10-http.md`) treat every request independently, with no
memory of previous ones; **stateful** interactions (a WebSocket session,
an authenticated session tracked via cookies) require the server to
remember something between exchanges. A proxy that load-balances
stateless requests can send each one anywhere
(`06-proxy/02-load-balancer.md`'s round robin); a proxy fronting stateful
interactions needs affinity (consistent hashing) or the state has to live
somewhere shared, not on one instance.

## Practice
1. In `labs/00-tcp-server`, write a test client that sends a 10,000-byte
   payload in a single `write_all` call, but have the *server* read with
   a 256-byte buffer and log how many `read()` calls it took to receive
   it all. Confirm the count is not `10000 / 256` exactly, and explain why
   from the byte-stream section above.
2. Capture a UDP exchange (e.g. a DNS query: `tcpdump -i lo udp port 53`
   while running `dig @127.0.0.1 example.com` against a local resolver,
   or just any UDP traffic you can generate) and a TCP exchange side by
   side; identify the handshake in the TCP capture and confirm there is
   none in the UDP one.
3. Write down, in one sentence each, what state a TCP connection is
   tracking that a UDP "connection" (really just a fixed 4-tuple you're
   choosing to reuse) is not.
4. Read `01-network/08-tcp.md`'s 3-way handshake section and
   `01-network/13-tls.md`'s handshake section back to back; list what
   each one is agreeing on, using the framing from this file.
