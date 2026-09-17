# WebSocket Proxying

## What to learn

### The Upgrade handshake
A WebSocket connection starts as a normal HTTP/1.1 GET request with `Upgrade: websocket`, `Connection: Upgrade`, and a `Sec-WebSocket-Key`. The server responds `101 Switching Protocols` with `Sec-WebSocket-Accept` computed from that key (SHA-1 + a fixed GUID, per RFC 6455 §1.3). After the `101`, the connection stops being HTTP entirely — it's now a raw framed byte stream over the same TCP (or TLS) connection.

```rust
// sketch: computing Sec-WebSocket-Accept
use sha1::{Sha1, Digest};
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
fn accept_key(client_key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(client_key.as_bytes());
    hasher.update(WS_GUID.as_bytes());
    base64::encode(hasher.finalize())
}
```

Note what this handshake is *not*: `Sec-WebSocket-Key` is not a secret and
the accept computation is not authentication. It exists so that a cache or
a non-WebSocket-aware intermediary can't be tricked into treating the
response as a normal HTTP response — nothing more. Never read security
into it.

Gotcha: `Connection` and `Upgrade` are hop-by-hop headers
(`05-http-stack/keepalive.md`). A proxy must not blindly forward them —
it terminates one upgrade and initiates another, regenerating both headers
for the upstream leg. A proxy that strips hop-by-hop headers correctly and
*then* forgets to re-add them for upgrade requests breaks WebSockets
entirely; this is the most common way a working proxy loses WebSocket
support during a refactor.

Gotcha: validate `Sec-WebSocket-Version: 13` and negotiate
`Sec-WebSocket-Protocol` honestly — if the client offers subprotocols, the
response must name exactly one of them or none at all. Echoing back the
whole list is a spec violation that some clients accept and others drop
the connection over.

### Origin checking: the vulnerability that WebSockets hand you
The browser sends cookies with the WebSocket handshake, and **the
same-origin policy does not apply** — there is no CORS preflight for a
WebSocket upgrade. So any website the victim visits can open a WebSocket
to your service, authenticated as the victim, and read everything on it.
This is Cross-Site WebSocket Hijacking, and it is the default behavior
unless you stop it.

The handshake carries an `Origin` header (browsers set it and don't let
scripts forge it). Validate it against an allowlist and reject mismatches
at the upgrade. Non-browser clients don't send `Origin` at all, so decide
explicitly what a missing `Origin` means for your service rather than
letting it default to allowed.

Gotcha: token-based auth on the WebSocket itself (a token in the URL or in
the first message) sidesteps the cookie problem entirely, and is the more
robust design — but a token in the query string ends up in access logs
(`08-observability/logging.md`), so scrub it there.

### Why a proxy can't treat this as request/response after the upgrade
Once the `101` is sent, both the router and any per-request middleware (auth, compression, caching) built around "one request in, one response out" no longer apply — there is no next request on this connection, just a bidirectional byte pipe. A reverse proxy must special-case Upgrade: after forwarding the handshake, it switches to relaying raw bytes both directions until either side closes.

This has a consequence for every timeout and limit you configured against
a request/response mental model. The read timeout meant to catch a stalled
request (`06-proxy/upstream.md`) now fires on a perfectly healthy idle
WebSocket. The "total request duration" cap kills a connection that is
supposed to live for hours. **Applying request timeouts to upgraded
connections is the single most common WebSocket-through-a-proxy bug**, and
it presents as "our app disconnects every 60 seconds" — which users
notice and logs rarely explain.

Once upgraded, the applicable limits are different in kind: an idle
timeout measured against *ping/pong liveness* rather than requests, a
connection lifetime cap, and per-connection memory bounds.

### Framing basics
WebSocket frames have an opcode (text, binary, close, ping, pong, continuation), a payload length (with an extended-length encoding for large payloads), and a masking key for client→server frames (masking is mandatory client-side, forbidden server-side — an anti-cache-poisoning measure from the RFC). A proxy that only forwards bytes doesn't need to fully parse frames unless it needs to inspect/filter messages.

Gotcha: if you *do* parse frames (to inspect, filter, or enforce message
size), the 64-bit extended length field is attacker-controlled. A frame
header claiming a 2^63-byte payload must be rejected against a configured
maximum *before* any allocation — never `Vec::with_capacity(declared_len)`.
This is the same class of bug as a decompression bomb
(`07-security/ddos.md`): trusting a length field the peer chose.

### Keepalive via ping/pong
Long-lived idle WebSocket connections look identical to a dead connection from the network's perspective (a NAT box or LB can silently drop them). Either side can send a `ping` frame; the other must reply `pong` with the same payload. A proxy relaying WebSocket traffic should either pass these through transparently or, if terminating and re-establishing two separate WebSocket legs, generate its own keepalive pings on each leg independently.

Gotcha: pings must be on a timer *and* have a pong deadline. Sending pings
without tracking whether pongs come back detects nothing — the connection
is dead either way, you just feel better about it. Close the connection
after a missed pong, and count those closures as a metric
(`08-observability/metrics.md`): a rising rate usually means an
intermediary is dropping idle connections, which is actionable.

### Backpressure
Unlike normal request/response where `Content-Length` bounds the work, a WebSocket connection can have one side producing messages faster than the other can consume — the proxy sits in the middle of two independently-paced streams and needs bounded buffers (not unbounded queues) so a slow client can't cause unbounded memory growth on the proxy.

`tokio::io::copy_bidirectional` gives you this for free at the byte level:
it reads into a fixed buffer and won't read more until the write side has
drained, so a slow reader naturally stops the fast writer. The moment you
introduce your own channel between the two legs (to inspect or transform
messages), you own the backpressure problem — use a bounded channel, and
understand that "bounded" means a slow client eventually blocks the
upstream read, which is correct.

Gotcha: a WebSocket connection is a *long-lived* resource, so the
accounting from `07-security/ddos.md` changes shape. Ten thousand idle
WebSockets cost ten thousand fds, sockets, and buffer pairs, indefinitely,
while generating no requests at all — so request-rate limits don't
constrain them. Cap concurrent upgraded connections explicitly, and
per-client as well as globally.

### WebSocket over HTTP/2 and HTTP/3
RFC 8441 carries WebSockets over HTTP/2 using an extended `CONNECT` with a
`:protocol` pseudo-header, rather than the HTTP/1.1 `Upgrade` mechanism
(HTTP/3 does the equivalent). A proxy terminating HTTP/2 from clients and
speaking HTTP/1.1 upstream — or vice versa — has to translate between the
two forms, and a proxy that only knows the `Upgrade` path will reject
HTTP/2 WebSocket clients outright. Know which forms your stack supports
before promising WebSocket support over HTTP/2.

## Practice
Build these in order.

1. In `labs/02-http-server`, implement the upgrade handshake by hand —
   validate `Sec-WebSocket-Version`, compute `Sec-WebSocket-Accept`, no
   WebSocket crate. **Done when** a real browser or `websocat` client
   completes the handshake against it.
2. Add `Origin` validation against an allowlist, with an explicit policy
   for a missing `Origin`. **Done when** a handshake from a disallowed
   origin is rejected at `101` time — write an HTML page on a different
   origin that opens a socket to your server and confirm it fails.
3. Take over the raw upgraded stream and echo text frames, parsing just
   enough to unmask and re-frame. **Done when** a client round-trips text
   and binary messages, and a frame declaring an absurd payload length is
   rejected without allocating.
4. In `labs/05-reverse-proxy`, add pass-through proxying: forward the
   handshake (regenerating hop-by-hop headers), then relay with
   `tokio::io::copy_bidirectional`. **Done when** an end-to-end WebSocket
   works through the proxy.
5. Audit your timeouts. **Done when** an idle WebSocket survives well past
   the request read timeout and the total request timeout — set both to
   10 seconds deliberately and confirm a 5-minute idle socket lives, after
   first confirming it *dies* without the fix.
6. Add ping/pong with a pong deadline and a missed-pong metric. **Done
   when** a peer that stops responding to pings is closed within the
   deadline and the closure is counted.
7. Add a backpressure test: fast upstream writer, deliberately slow client
   reader. **Done when** proxy memory stays flat for the duration rather
   than growing with the backlog.
8. Cap concurrent upgraded connections, globally and per client. **Done
   when** a client opening connections in a loop is refused past its cap
   and normal HTTP traffic is unaffected.
