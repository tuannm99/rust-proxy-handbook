# UDP

The other transport protocol: no connection, no ordering, no retransmission —
and the substrate QUIC/HTTP/3 and DNS are built on. Knowing exactly what
UDP *doesn't* give you is what makes TCP's machinery ([`12-tcp.md`](12-tcp.md),
[`13-tcp-reliability.md`](13-tcp-reliability.md)) legible.

## What to learn

### What UDP is: ports plus a checksum
A UDP header is 8 bytes: source port, destination port, length, checksum.
That is the whole protocol. IP delivers a packet to a *machine*
([`08-ip-and-icmp.md`](08-ip-and-icmp.md)); UDP's one contribution is the **port**, which
picks the *process* on that machine ([`02-addressing.md`](02-addressing.md)). Everything
else a transport might do — setup, ordering, delivery guarantees, rate
adaptation — is simply absent. The checksum detects corruption; a packet
that fails it is silently discarded.

### Datagrams preserve boundaries; streams don't
One `sendto` of 300 bytes produces one datagram, and one `recvfrom` on
the other side returns exactly those 300 bytes, or nothing. Message
boundaries survive — the opposite of TCP's byte stream, where two writes
can arrive as one read and one write as two ([`03-byte-streams.md`](03-byte-streams.md)). The
catch: if your buffer is smaller than the datagram, the excess is
**truncated and lost**, not left for the next read.

```rust
let sock = tokio::net::UdpSocket::bind("0.0.0.0:5353").await?;
let mut buf = [0u8; 1500];
let (n, peer) = sock.recv_from(&mut buf).await?; // one whole datagram
sock.send_to(&buf[..n], peer).await?;
```

There is no `accept` and no per-peer socket: one socket receives from
everyone, and each `recv_from` tells you who sent it. "Connection" state,
if any, is whatever your application builds.

### What can go wrong, and who handles it
UDP datagrams can be **lost**, **duplicated**, **reordered**, and
**delayed**; the sender is never told. The only feedback is indirect: an
ICMP port-unreachable if nothing listens ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)), which a
connected UDP socket (`connect()` on a UDP socket just records a default
peer) surfaces as `ECONNREFUSED` on the next call. Anything that needs
reliability must add it: DNS retries a query itself after a timeout;
QUIC rebuilds ordering, loss recovery, congestion control and encryption
in user space ([`18-http3.md`](18-http3.md)).

### No congestion control means the sender must be polite
TCP slows down when the network is congested; UDP will happily send at
line rate into a saturated link and make things worse for everyone,
including itself. Protocols built on UDP must implement their own
congestion control (QUIC does) or stay low-volume (DNS). A proxy that
fronts UDP traffic should treat unbounded forwarding as a bug, for the
same reason it treats unbounded buffering as one.

### Size limits and fragmentation
A datagram larger than the path MTU gets IP-fragmented, and losing any
fragment loses the datagram — so UDP applications keep payloads under
~1200 bytes to fit in one packet on almost any path (QUIC's minimum
packet size is 1200 for exactly this reason). The hard cap is 65507 bytes
(65535 minus IP and UDP headers). Big DNS answers that exceed the limit
are the reason DNS falls back to TCP ([`14-dns.md`](14-dns.md)).

### UDP and a stateful middlebox
NAT devices and firewalls track UDP "flows" by 4-tuple even though UDP
has none, and expire them on a **timer** (often 30–120 seconds), since
there's no FIN to signal the end. A long-idle QUIC connection or UDP
session can lose its NAT mapping and silently stop working, which is why
protocols send periodic keepalives and why QUIC supports **connection
migration** with a connection ID rather than relying on the 4-tuple
([`18-http3.md`](18-http3.md)).

### Proxying UDP and the reflection-attack gotcha
UDP source addresses are trivially forged because there's no handshake to
verify them. An attacker sends a small query with the victim's address as
the source to a server that replies with something large — **amplification**
(open DNS resolvers, memcached on UDP, NTP). Any UDP service you expose
must either verify the source (QUIC does: a server may not send more than
3x the bytes it has received from an unvalidated address, and uses retry
tokens to validate — see [`18-http3.md`](18-http3.md) for QUIC itself) or never reply with
more than it received ([`07-security/09-ddos.md`](../07-security/09-ddos.md)). Load balancing UDP also needs
affinity: the balancer has to pin a 4-tuple (or QUIC connection ID) to
one backend because there is no connection to anchor it.

## Practice

1. Start a UDP listener with `nc -u -l 9999` and send datagrams with
   `echo -n hello | nc -u -w1 127.0.0.1 9999`; send two in quick
   succession and confirm each arrives as a separate message (contrast
   with TCP, where `nc -l` may merge them).
2. Write a tiny tokio UDP echo (a scratch project, not part of the
   workspace) using the snippet above, shrink the receive buffer to 4 bytes,
   send 10 bytes, and observe the truncation.
3. Run `sudo tcpdump -n -i lo udp port 9999` during item 1 and identify
   the 8-byte UDP header fields in the `-X` hex dump (`tcpdump -n -X`).
4. Send to a UDP port with no listener on a *connected* socket (scratch
   program: `UdpSocket::connect` then `send` twice) and observe
   `ECONNREFUSED` on the second call; tie it to the ICMP message you
   capture with `tcpdump icmp`.
5. Use `tc qdisc add dev lo root netem loss 30%` (see
   [`12-testing/03-chaos.md`](../12-testing/03-chaos.md)) and send 100 numbered datagrams; count how
   many arrive, whether any are reordered (add `delay 20ms 10ms`), and
   explain why the sender cannot tell.
6. Compare the shape of [`labs/00-tcp-server`](../../labs/00-tcp-server) (an `accept` loop spawning a task
   per connection) with what a UDP service needs (one socket, one
   `recv_from` loop, per-peer state in a map keyed by source address), and
   write down in your own notes what would have to change — this is the
   structural jump [`labs/09-http3`](../../labs/09-http3) makes.
