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
still has (see `http2.md`).

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
cert/SNI/ALPN machinery as `tls.md`, just carried differently on the wire.

### Current Rust ecosystem
`quinn` is the primary async QUIC implementation; `h3` (built on `quinn`)
implements HTTP/3 framing on top of it. As of this writing neither `hyper`
nor `hyper-util` speak HTTP/3 directly — it's a separate integration, which
is why `proxy` treats HTTP/3 as a stretch goal
rather than a baseline requirement.

## Practice

1. Capture HTTP/3 traffic from a browser hitting a site that supports it
   (check `chrome://net-export` or a `curl --http3` build) and confirm it's
   UDP on the wire, not TCP.
2. Read the `quinn` crate's example client/server and identify where the
   TLS 1.3 handshake happens relative to the QUIC handshake.
3. As a stretch exercise on `proxy`, add an
   HTTP/3 listener using `quinn` + `h3` alongside the existing HTTP/1.1/2
   listener, and compare what had to change in your TLS config.
4. Explain in your own words why 0-RTT data should never be trusted for a
   non-idempotent request like `POST /transfer-funds`.
