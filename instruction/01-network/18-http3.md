# HTTP/3

QUIC fundamentals.

## What to learn

### QUIC replaces TCP, not just HTTP framing
HTTP/3 runs over QUIC, which runs over UDP — not TCP. QUIC reimplements
reliability, ordering, and congestion control itself at the transport
layer, in user space, instead of relying on the kernel's TCP stack. This is
the biggest mental shift from HTTP/1.1 and /2: your proxy's "listener" for
HTTP/3 is a UDP socket, not a `TcpListener`.

### Why QUIC avoids head-of-line blocking
QUIC multiplexes independent streams the same way HTTP/2 does, but because
loss recovery happens per-stream inside QUIC (not per-connection the way
TCP retransmission does), a lost packet on one stream doesn't stall the
other streams. This fixes the TCP-level HOL blocking that HTTP/2 over TCP
still has (see [`17-http2.md`](17-http2.md)).

### Connection migration and 0-RTT
QUIC connections are identified by a Connection ID, not a
(source IP, source port, dest IP, dest port) 4-tuple — so a client
switching networks (WiFi to cellular) can keep the same QUIC connection.
0-RTT lets a client resume a previous session and send application data in
its very first flight, at the cost of replay-attack exposure for
non-idempotent requests — a proxy accepting 0-RTT data must treat it as
"might be replayed" and only allow it for safe/idempotent requests.

### TLS 1.3 is mandatory and baked in
QUIC doesn't layer TLS on top the way TCP+TLS does — the QUIC handshake
*is* a TLS 1.3 handshake carried in QUIC transport parameters, so there's
no cleartext QUIC. This means every HTTP/3 deployment needs the same
cert/SNI/ALPN machinery as [`19-tls.md`](19-tls.md), just carried differently on the wire.

### One UDP socket, many connections
The operational shift is bigger than "UDP instead of TCP". With TCP,
`accept()` hands you a distinct fd per connection and the kernel demuxes
for you. With QUIC you own **one** UDP socket receiving datagrams for
*every* connection, and you demultiplex in user space by reading the
Connection ID out of each packet and routing it to the right connection
state. There is no per-connection fd, so fd limits stop being the
constraint — and your own connection table becomes it.

Consequences that show up immediately: the receive loop is a hot single
point (`quinn` mitigates with `recvmmsg` batching and `SO_REUSEPORT` across
workers), and there is no `accept()` backpressure — datagrams arrive
whether you are ready or not, so the accept-rate limiting from
[`07-security/09-ddos.md`](../07-security/09-ddos.md) has to happen after parsing enough of the packet to
know it is a new connection attempt.

Gotcha: UDP buffer sizes matter far more than for TCP. The kernel's default
`net.core.rmem_max` is routinely too small for high-throughput QUIC, and
the symptom is silent packet drops counted in `netstat -su`, not an error
your code sees.

### Streams, and why HTTP/3 needed QPACK
QUIC gives HTTP/3 bidirectional and unidirectional streams, each reliable
and ordered *within itself*. HTTP/3 maps one request/response onto one
bidirectional stream, so the framing layer is much simpler than HTTP/2's —
no stream multiplexing to implement, QUIC already did it.

But HPACK cannot survive here. HPACK's dynamic table assumes headers arrive
in order; QUIC streams deliver independently, so a table update on stream 4
might arrive after a reference to it on stream 8. **QPACK** solves this by
carrying table updates on dedicated unidirectional encoder/decoder streams
and letting a request block until the entries it references have arrived.

Gotcha: that blocking is a head-of-line stall you reintroduced by
compressing aggressively. QPACK exposes `SETTINGS_QPACK_BLOCKED_STREAMS` to
bound it; setting the dynamic table to zero disables the risk entirely at
the cost of compression ratio, which is a legitimate choice for a proxy
that values predictable latency.

### Congestion control moved into your process
TCP's congestion control is the kernel's problem and is tuned by the
operator. QUIC's is your dependency's problem, and it runs on your CPU
budget: per-packet ACK processing, loss detection timers, and pacing all
happen in user space. Expect meaningfully higher CPU per byte than TCP —
that gap is the main reason HTTP/3 is not automatically the right default
for an internal proxy hop.

### Deploying it: Alt-Svc and the fallback path
Clients do not start with HTTP/3. They connect over TCP and learn about
HTTP/3 from an `Alt-Svc: h3=":443"; ma=86400` response header, then try
QUIC on later requests. So an HTTP/3 deployment is *always* also a
TCP deployment — you cannot drop HTTP/1.1/2 — and you must handle UDP being
blocked by a middlebox, where the client silently falls back to TCP.

Gotcha: advertise `Alt-Svc` only once your UDP path is actually reachable.
Advertising it while UDP is firewalled sends clients into a connect-timeout
and retry cycle on every request, which is slower than never having
advertised it.

### Current Rust ecosystem
`quinn` is the primary async QUIC implementation; `h3` (built on `quinn`)
implements HTTP/3 framing on top of it. As of this writing neither `hyper`
nor `hyper-util` speak HTTP/3 directly — it's a separate integration, which
is why [`proxy`](../../proxy) treats HTTP/3 as a stretch goal
rather than a baseline requirement.

### quinn in practice
What you need to get [`labs/09-http3`](../../labs/09-http3) talking, in the order you hit it:

- **Certificates.** QUIC always runs TLS 1.3, so even a lab echo server
  needs a certificate. Make a CA and a leaf exactly as in
  [`01-network/19-tls.md`](19-tls.md)'s "A local CA for testing". Don't serve the CA
  itself: the quinn client verifies with rustls, and rustls rejects a CA
  certificate presented as a server certificate.
- **Server side.** `quinn::ServerConfig::with_single_cert(chain, key)` builds a
  config from the same `CertificateDer`/`PrivateKeyDer` values rustls uses.
  `quinn::Endpoint::server(config, addr)` binds the one UDP socket. For ALPN
  or other rustls settings, build a `rustls::ServerConfig` yourself, convert
  it with `quinn::crypto::rustls::QuicServerConfig::try_from(...)`, and wrap
  that in `quinn::ServerConfig::with_crypto(Arc::new(...))`.
- **Client side.** `quinn::Endpoint::client("0.0.0.0:0".parse()?)` binds an
  ephemeral UDP port. `quinn::ClientConfig::with_root_certificates(roots)`
  takes a `rustls::RootCertStore` into which you added your `ca.pem`.
  Install it with `set_default_client_config`. `endpoint.connect(addr,
  "localhost")?.await` connects, and the name must be one of the leaf's
  SANs.
- **ALPN.** If either side sets ALPN protocols, both must share one, or the
  handshake fails. A raw echo lab can set none on both sides. HTTP/3
  requires the protocol `h3`, which is where the `h3` stretch goal starts.
- **Accepting.** `endpoint.accept().await` yields an `Incoming` for each new
  connection, and awaiting that completes the handshake into a
  `Connection`. Spawn one task per connection, as with TCP.
- **Streams.** `connection.open_bi().await` (client) and
  `connection.accept_bi().await` (server) give a `(SendStream, RecvStream)`
  pair. Each stream has its own flow-control window, which is exactly why
  a stream you stop reading stalls only itself. `send.finish()` marks the
  end of what you send. `recv.read_to_end(limit)` reads until the peer
  finishes, with a cap. Streams are cheap: open one per request, not one
  per connection.
- **Seeing the demultiplexing.** Every connection has a `stable_id()` and a
  `remote_address()`. With a `tracing_subscriber` installed and
  `RUST_LOG=quinn_proto=trace`, quinn logs `new connection` events with the
  initial connection ID. Two clients show two connection IDs arriving on
  one socket, while `ss -uanp` shows that there is only one socket.

Gotcha: `netstat` (from `net-tools`) isn't installed on many modern
distros. The same UDP counters are in `nstat -az | grep -i udp` or
`/proc/net/snmp`. The one that climbs when the socket buffer overflows is
`UdpRcvbufErrors` (`RcvbufErrors` in `netstat -su` output).

## Practice

1. Capture HTTP/3 traffic from a browser hitting a site that supports it
   (check `chrome://net-export` or a `curl --http3` build) and confirm it's
   UDP on the wire, not TCP.
2. Read the `quinn` crate's example client/server and identify where the
   TLS 1.3 handshake happens relative to the QUIC handshake.
3. Build a minimal QUIC echo endpoint in [`labs/09-http3`](../../labs/09-http3) with `quinn`,
   then as a stretch exercise add an HTTP/3 listener using `quinn` + `h3`
   to [`proxy`](../../proxy) alongside the existing HTTP/1.1/2 listener, and compare what
   had to change in your TLS config.
4. Explain in your own words why 0-RTT data should never be trusted for a
   non-idempotent request like `POST /transfer-funds`.
5. In [`labs/09-http3`](../../labs/09-http3), run two concurrent QUIC connections against your one
   UDP socket and log the Connection ID demux — confirm you can see both
   being routed from the same socket.
6. Check `net.core.rmem_max` on your machine, then drive the endpoint hard
   enough to see drops in `netstat -su`; raise the buffer and confirm the
   drop counter stops climbing. Then compare CPU per megabyte transferred
   between your QUIC endpoint and a plain TCP+TLS transfer of the same
   data — quantify the gap for yourself rather than taking the claim on
   faith.
