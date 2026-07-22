# HTTP/2

Frames, streams, HPACK, multiplexing.

## What to learn

### Frames and the binary framing layer
HTTP/2 replaces the text-based /1.1 wire format with typed, length-prefixed
binary frames (`HEADERS`, `DATA`, `SETTINGS`, `WINDOW_UPDATE`, `RST_STREAM`,
`GOAWAY`, ...) multiplexed over a single TCP connection. Every frame
belongs to a stream ID (0 = connection-level). A proxy terminating HTTP/2
has to speak this frame layer explicitly — it's not "HTTP with fancier
headers", it's a genuinely different wire protocol.

### Streams and multiplexing
Many logical request/response exchanges (streams) run concurrently over
one TCP connection, each independently flow-controlled. This is what fixes
HTTP/1.1's need for 6 parallel TCP connections per origin. Gotcha:
multiplexing solves *connection-level* head-of-line blocking but not
*TCP-level* HOL blocking — one dropped TCP segment still stalls every
stream on that connection until it's retransmitted (this is exactly what
HTTP/3 over QUIC fixes, see `http3.md`).

### HPACK header compression
Headers are compressed with HPACK: a static table of common header
name/value pairs, a dynamic table built incrementally per connection, and
Huffman coding for literal values. Because the dynamic table is stateful
per-connection, a proxy that decodes HPACK on the client-facing side and
re-encodes for the upstream side must maintain two independent HPACK
states — you cannot just relay raw frames if headers are being modified.

### Flow control
Both stream-level and connection-level flow control use a credit-based
`WINDOW_UPDATE` mechanism — a sender must not send more `DATA` than the
receiver's advertised window. A proxy forwarding a large response body from
a fast upstream to a slow client must respect the client's flow control
window and apply backpressure to the upstream read, not buffer unboundedly.

### Stream lifecycle and concurrency limits
A stream walks a small state machine: idle → open (on `HEADERS`) →
half-closed (one side sent `END_STREAM`) → closed (both sides done, or
`RST_STREAM`). Client-initiated stream IDs are odd, server-initiated even,
and IDs must strictly increase — a stream ID can never be reused, so a
long-lived connection eventually exhausts the 31-bit space and must be
retired with `GOAWAY`.

`SETTINGS_MAX_CONCURRENT_STREAMS` caps how many streams may be open at
once. This is the single most important setting for a proxy: it is the
per-connection concurrency bound, and it maps directly onto how many
upstream requests one client connection can generate.

Gotcha: `GOAWAY` carries the last stream ID the sender will process, which
is what makes graceful shutdown (`09-architecture/graceful-shutdown.md`)
possible — streams below it complete, streams above it the client may
safely retry elsewhere. A proxy that closes the TCP connection without
`GOAWAY` turns a clean drain into client-visible errors.

### HPACK is stateful, and that is the danger
The dynamic table means header decoding depends on every previous header
frame on that connection. Three consequences a proxy must handle:

**Ordering is mandatory.** `HEADERS` frames must be decoded in the order
received, even though streams are otherwise independent — decoding stream 7
before stream 5 corrupts the table for both. A `HEADERS` frame followed by
`CONTINUATION` frames is atomic: nothing else may interleave.

**The table is a memory commitment.** `SETTINGS_HEADER_TABLE_SIZE` bounds
it per connection, but a proxy holding thousands of connections multiplies
that by the connection count. Two tables per connection, in fact — decode
state for the client side, encode state for the upstream side.

**Decompression is an amplification vector.** A small compressed header
block can expand enormously (the "HPACK bomb"), so bound the *decoded*
header size, not just the frame size. This is the HTTP/2 form of the same
limits discipline as `05-http-stack/parser.md`.

### Flow control in a proxy: where backpressure actually lands
Two independent windows exist: per-stream and per-connection. A sender
must respect both, so a stream with window available still cannot send if
the connection window is exhausted. Windows start at 65535 bytes and only
grow via `WINDOW_UPDATE`.

The proxy-specific problem: a fast upstream feeding a slow client. The
correct behavior is to let the client's unreplenished window stop you from
reading further from the upstream — the backpressure chain must run
end to end. The failure mode is buffering the response in memory because
the read side had data available, which converts one slow client into
unbounded memory growth.

Gotcha: when the upstream leg is HTTP/1.1, there is no window to
propagate. Backpressure there is TCP's receive window — you stop reading
the upstream socket, its send buffer fills, and the kernel stops ACKing.
That works, but only if you actually stop calling `read()`; an
eager read-into-a-`Vec` loop defeats it silently.

### The attacks unique to HTTP/2
Cheap-to-send frames that cause expensive server work are the recurring
theme, and a proxy must bound each:

- **Rapid Reset (CVE-2023-44487)** — open a stream and immediately
  `RST_STREAM` it. The stream no longer counts against
  `MAX_CONCURRENT_STREAMS`, so a client can create unlimited *work* while
  never exceeding the concurrency limit. The mitigation is to track and
  rate-limit resets per connection, closing connections that exceed it.
- **Settings/ping floods** — `SETTINGS` and `PING` demand acknowledgement;
  flooding them forces the server to generate responses indefinitely.
- **Empty `DATA` frame floods** — zero-length frames consume parsing work
  and consume no flow-control window, so windows never throttle them.

The pattern: any frame that is cheap for the client and not accounted for
by an existing limit needs its own rate limit. `h2` has fixed each of
these, which is a concrete argument for the position in
`05-http-stack/parser.md` — use the maintained implementation.

### Where Rust fits
`h2` (used internally by `hyper` when the `http2` feature is enabled) is
the de facto HTTP/2 implementation in the Rust ecosystem; `hyper-util`'s
auto server builder in `labs/02-http-server`/`reverse-proxy`
negotiates HTTP/1.1 vs HTTP/2 via ALPN (see `tls.md`) so you get this "for
free" once TLS is wired up, but you should still be able to explain what's
happening below that abstraction.

## Practice

1. Capture an HTTP/2 connection with Wireshark (or `nghttp -v`) and
   identify at least 4 distinct frame types on the wire.
2. Enable the `http2` feature on the hyper server in
   `labs/02-http-server` and confirm via `curl --http2` that it
   negotiates HTTP/2 over TLS (ALPN).
3. In `labs/08-http2`, send two concurrent requests over the same HTTP/2
   connection with `curl --http2 -v` and confirm both streams complete on
   one TCP connection (check with `ss` or `lsof`); then shrink the initial
   window and watch the second stream stall behind flow control.
4. Read the `h2` crate's flow-control docs and explain, in your own words,
   what happens if your proxy's client-facing connection is HTTP/2 but the
   upstream connection is HTTP/1.1 — where does the flow-control mismatch
   get absorbed?
5. Prove backpressure end to end in `labs/08-http2`: serve a large response
   to a client that reads slowly (or stops reading entirely), and confirm
   your proxy's memory stays flat instead of buffering the whole body.
6. Set `SETTINGS_MAX_CONCURRENT_STREAMS` low (e.g. 2), open more streams
   than that, and observe how the peer is held back; then reproduce Rapid
   Reset by opening and immediately resetting streams in a loop and confirm
   the concurrency limit alone does *not* stop you.
7. Trigger `GOAWAY` by shutting the server down mid-request and confirm
   in-flight streams below the last-stream-ID complete rather than erroring.
