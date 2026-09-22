# TCP/IP

Topics:
- 3-way handshake
- Congestion Control
- Flow Control
- Keepalive
- TIME_WAIT
- Socket lifecycle

## What to learn

### 3-way handshake
`SYN` -> `SYN-ACK` -> `ACK` establishes a connection and synchronizes
initial sequence numbers before any application data flows. This round
trip is pure latency overhead your proxy pays on every new upstream
connection — it's the core argument for connection pooling/reuse to
upstreams (see `06-proxy/01-upstream.md`) instead of dialing fresh per request.

### TIME_WAIT and socket lifecycle
The side that sends the first `FIN` (active closer) ends up in `TIME_WAIT`
for 2×MSL (typically ~60s on Linux) before the fd is fully released, to
guarantee any stray duplicate packets from the old connection don't get
confused with a new one reusing the same 4-tuple. A busy proxy that closes
many short-lived upstream connections can accumulate thousands of
`TIME_WAIT` sockets and exhaust ephemeral ports — another reason to reuse
upstream connections rather than open-and-close per request.

### Nagle's algorithm and TCP_NODELAY
Nagle's algorithm batches small writes to avoid sending many tiny packets,
trading latency for fewer packets on the wire. Combined with delayed ACKs
on the receiving side, this can add up to ~40ms of avoidable latency to
small request/response exchanges. Proxies (and most HTTP servers) disable
it with `TCP_NODELAY` because HTTP request/response framing already
batches data sensibly at the application layer.

```rust
let stream = tokio::net::TcpStream::connect(addr).await?;
stream.set_nodelay(true)?;
```

### Congestion control (basics)
The sender maintains a congestion window that grows (slow start, then
congestion avoidance) until packet loss signals congestion, then shrinks.
Algorithms like Cubic (Linux default) or BBR trade off differently under
loss vs latency-based congestion signals. A proxy doesn't implement this
itself (it's kernel/TCP-stack territory) but a *new* connection always
starts from a small congestion window — which is why connection reuse to
upstreams matters for throughput, not just latency.

### Flow control
Distinct from congestion control: flow control (the TCP receive window)
protects the *receiver* from being overwhelmed, independent of network
congestion. If your proxy's read loop falls behind draining a socket's
receive buffer, the window shrinks and the sender stalls — this is how
backpressure naturally propagates through a proxy chain if you don't
buffer unboundedly.

### Keepalive
TCP keepalive (`SO_KEEPALIVE` + `TCP_KEEPIDLE`/`TCP_KEEPINTVL`/
`TCP_KEEPCNT`) periodically probes an idle connection to detect a dead peer
that never sent a `FIN` (e.g. the machine crashed, or a NAT/firewall
silently dropped the mapping). This is distinct from *application-level*
HTTP keep-alive (`05-http-stack/04-keepalive.md`) — TCP keepalive detects a
dead peer, HTTP keep-alive decides whether to reuse a connection for
another request.

## Practice

1. Capture a handshake and a connection teardown with
   `tcpdump -i lo port 8080` while hitting `labs/00-tcp-server`, and
   identify the SYN/SYN-ACK/ACK and FIN/FIN-ACK sequences.
2. Run `ss -tn state time-wait | wc -l` while hammering
   `labs/05-reverse-proxy` with short-lived connections (no
   keep-alive), then again with connection reuse enabled — compare counts.
3. Benchmark request latency with and without `set_nodelay(true)` on small
   request/response payloads and measure the difference.
4. Configure TCP keepalive on the upstream client connections in
   `labs/05-reverse-proxy` and verify (by killing an upstream
   process without closing its socket, e.g. via `iptables` drop rules) that
   your proxy eventually detects the dead peer.
