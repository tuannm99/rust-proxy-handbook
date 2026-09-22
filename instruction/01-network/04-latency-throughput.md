# Latency, Bandwidth, Throughput, RTT

Part of the from-scratch fundamentals series — see `01-network/01-fundamentals.md`
for the full index. Four numbers that get used interchangeably in casual
conversation and shouldn't be — mixing them up leads to optimizing the
wrong thing.

## What to learn

### Latency: how long, not how much
**Latency** is how long one piece of data takes to get from A to B.
It's dominated by physical distance (light in fiber travels at roughly
200,000 km/s, not 300,000, due to the refractive index of glass) and the
number of hops (`02-addressing.md`'s routing section) — not by how "fast"
your connection is in the colloquial sense. A cross-continental link has
tens of milliseconds of latency no matter how much bandwidth you throw at
it, because that's a speed-of-light floor, not a congestion problem.

### Bandwidth: the ceiling
**Bandwidth** is the maximum rate a link can carry data, e.g. 1 Gbps.
It's a ceiling, not a guarantee you'll hit it — plenty of factors keep
real transfers well under the theoretical maximum.

### Throughput: what you actually got
**Throughput** is the rate you *actually* achieve, bounded by bandwidth
but usually lower, due to protocol overhead, congestion, retransmission,
or application behavior (e.g. how fast your own code can produce or
consume bytes). "Bandwidth" and "throughput" get used interchangeably in
casual speech; keep them distinct when you're actually diagnosing
something; a link can have plenty of bandwidth and still deliver poor
throughput if something else is the bottleneck.

### RTT: the number that decides handshake cost
**RTT (round-trip time)** is the time for a message to go out and its
reply to come back — roughly `2 × latency` plus processing time at the
far end. This is the number that matters for "how many round trips does
this cost": every handshake (`03-byte-streams.md`) — TCP's, then TLS's on
top of it — is one more RTT of pure waiting before the first real
request byte moves. That's the entire argument for connection reuse in
`06-proxy/01-upstream.md`: paying an RTT-bound handshake once and reusing
the connection beats paying it on every request.

### Why a high-bandwidth link can still feel slow
A link can have huge bandwidth and still feel sluggish if latency (and
therefore RTT) is high — a satellite link is the classic example:
enormous bandwidth, terrible latency (the trip to geostationary orbit and
back is real physical distance). For a small request/response exchange —
most HTTP traffic — the transfer itself is so quick that RTT, not
bandwidth, dominates total time: you're waiting on round trips, not on
bytes. This is precisely why HTTP/2's multiplexing
(`01-network/11-http2.md`) and 0-RTT/session resumption in TLS
(`01-network/13-tls.md`) exist — they're attacking round-trip *count*,
not throughput.

### Bandwidth-delay product: how much can be "in flight"
The **bandwidth-delay product** (bandwidth × RTT) is how many bytes can
be in transit on the link at once, unacknowledged — TCP's congestion
window (`01-network/08-tcp.md`) has to grow to roughly this size before a
single connection can use the link's full bandwidth. On a high-bandwidth,
high-latency link ("long fat network" — a satellite link, or a
cross-continental fiber run), this product is large, and a connection
that starts with a small congestion window (every new TCP connection
does, via slow start) takes a while to ramp up to using the available
bandwidth at all. This is another reason a fresh connection per request
underperforms a reused one, independent of the handshake-RTT cost above:
a reused connection's congestion window is already warm.

## Practice
1. Run `ping example.com` (ICMP, a rough latency measurement) and
   `curl -w "%{time_connect} %{time_appconnect} %{time_starttransfer}\n"
   -o /dev/null -s https://example.com` — compare `ping`'s latency
   against `time_connect` (TCP handshake RTT), `time_appconnect` (add
   TLS), and `time_starttransfer` (add the first response byte).
   Identify how many extra round trips TLS cost you.
2. Pick a site geographically close to you and one far away (e.g. a
   server on another continent) and repeat the `curl -w` measurement for
   both — confirm the distant one's `time_connect` roughly matches what
   you'd expect from speed-of-light latency for that distance.
3. Download a large file (a few hundred MB) from a fast mirror and
   compute actual throughput (`size / time`); compare it against your
   connection's advertised bandwidth — explain any gap you find.
4. Compute the bandwidth-delay product for a 100 Mbps link at 150ms RTT
   (a plausible transcontinental figure) in bytes; compare that number
   against TCP's default initial congestion window (~10 segments,
   ~14KB) and explain why a fresh connection on this link starts out
   using a small fraction of the available bandwidth.
