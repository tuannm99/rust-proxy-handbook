# gRPC Proxying

gRPC is HTTP/2 with a specific framing convention on top — proxying it correctly means not breaking that convention, not understanding protobuf.

## What to learn
### gRPC's message framing inside HTTP/2 DATA frames
Each gRPC message is a 1-byte compression flag, a 4-byte big-endian length prefix, then that many bytes of protobuf payload — all carried inside ordinary HTTP/2 `DATA` frames (see `01-network/11-http2.md`). A proxy forwarding gRPC does not need to understand protobuf or even this framing; it only needs to forward `DATA` frames faithfully, byte-for-byte, without doing anything an HTTP/1.1-oriented code path might reflexively do (buffering the whole body to compute `Content-Length`, for instance — gRPC bodies are length-prefixed per-message, not once for the whole stream, and are often unbounded/streaming).

Gotcha: gRPC has its own per-message compression (that 1-byte flag),
negotiated with `grpc-encoding`/`grpc-accept-encoding`. It is *not*
HTTP `Content-Encoding`, and a proxy that applies its own response
compression (`05-http-stack/06-compression.md`) to a gRPC body corrupts it —
the client will try to parse a gzip stream as length-prefixed messages.
Exclude `application/grpc` content types from response compression
explicitly.

### Trailers carry the actual RPC outcome
Unlike HTTP/1.1 where a response is headers-then-body-then-done, HTTP/2 (and gRPC specifically) supports **trailers** — a second HEADERS frame sent *after* all DATA frames, carrying `grpc-status` and `grpc-message`. This is where the client learns whether the RPC actually succeeded — an HTTP 200 response from a gRPC call can still be a failed RPC once you check the trailer. Proxy code paths written against an HTTP/1.1 mental model routinely drop or mishandle trailers because "the response already ended" once headers and body were seen; for gRPC that assumption silently discards the actual result.

```rust
// what must survive the proxy hop, conceptually — headers, N data frames, then trailers
struct GrpcResponseShape {
    headers: HeaderMap,        // usually just ":status: 200", content-type
    data_frames: Vec<Bytes>,   // one or more length-prefixed gRPC messages
    trailers: HeaderMap,       // grpc-status, grpc-message — the real result
}
```

Gotcha: there is also a **trailers-only** response — a single HEADERS
frame with `grpc-status` and END_STREAM, no DATA at all — used when an RPC
fails immediately. A proxy that assumes "headers, then body, then
trailers" and waits for a body that never arrives will hang or mangle it.
Handle the zero-DATA case explicitly.

### Your error responses must be gRPC-shaped too
When the proxy itself fails a request — no healthy upstream, circuit open
(`06-proxy/05-retry.md`), rate limited (`07-security/07-ratelimit.md`) — the
reflex is to return HTTP 503 or 429 with a short body. To a gRPC client,
that is a malformed response: it's looking for `grpc-status` in trailers,
and a plain HTTP error surfaces as a confusing transport error rather than
the clean status code the application knows how to handle.

Emit a trailers-only response instead, with `:status: 200` and an
appropriate `grpc-status` — `14` (UNAVAILABLE) for no-upstream or circuit
open, `8` (RESOURCE_EXHAUSTED) for rate limiting, `4` (DEADLINE_EXCEEDED)
for timeouts. The client's existing retry and error handling then works as
designed, which is the entire point.

Gotcha: this requires the proxy to know the request *was* gRPC before it
errors — check `content-type: application/grpc*` early and route error
generation accordingly. A proxy with one shared error path will get this
wrong by construction.

### Streaming semantics: don't impose request/response buffering
gRPC has four call shapes: unary, client-streaming, server-streaming, and bidirectional streaming — all of them are just an HTTP/2 stream carrying however many DATA frames in either direction before trailers close it. A proxy that buffers a full request body before forwarding (reasonable for a small HTTP/1.1 POST) breaks client-streaming and bidi calls outright, since the upstream may need to start responding before the client finishes sending. Forward frames as they arrive; don't wait for stream-end unless you have a specific reason to.

Gotcha: the same applies to every timeout you inherited from the
request/response model, exactly as with WebSockets
(`05-http-stack/09-websocket.md`). A server-streaming RPC that emits an
update every few minutes is healthy; a total-request timeout kills it on
schedule. Worse, gRPC clients send their own deadline in the
`grpc-timeout` header — the proxy should *honor* that (and shorten it by
the time already spent) rather than imposing an unrelated one, so the
deadline the application set is the one that applies.

Gotcha: body buffering for WAF inspection (`07-security/06-waf.md`) or retry
replay (`06-proxy/05-retry.md`) is fundamentally incompatible with streaming
RPCs. Decide per content-type, not globally, or your first bidi-streaming
customer discovers it for you.

### Load balancing gRPC is not the same problem as load balancing HTTP/1.1
A gRPC client typically opens one long-lived HTTP/2 connection and multiplexes many independent RPCs over it (see `01-network/11-http2.md`'s multiplexing). A load balancer that picks an upstream *per connection* (like a plain L4/TCP balancer, or a naive `06-proxy/02-load-balancer.md` implementation written with one-request-per-connection HTTP/1.1 in mind) sends every RPC on that connection to the same upstream forever, defeating load balancing entirely once a client connects. Correct gRPC load balancing has to be aware of individual HTTP/2 streams and pick an upstream per-RPC, not per-connection.

Gotcha: this interacts badly with scaling events. Long-lived connections
pinned to a subset of upstreams means new upstreams added by autoscaling
(`06-proxy/07-service-discovery.md`) receive nothing — the connection
lifetime caps from `05-http-stack/04-keepalive.md` are what eventually
rebalance, and for gRPC the equivalent is periodically sending `GOAWAY` so
clients reconnect and redistribute.

### Health checking gRPC upstreams
gRPC defines its own health checking protocol (`grpc.health.v1.Health`)
with `Check` and `Watch` methods — an upstream that speaks gRPC may not
serve an HTTP `/healthz` at all, so the probe from
`06-proxy/03-healthcheck.md` needs a gRPC-aware variant. `Watch` is the
better of the two for a proxy: it streams status changes rather than
requiring a poll interval, so detection is immediate and the probe cost
from `healthcheck.md` largely disappears.

### Observability needs the gRPC dimension
An HTTP-only view of gRPC traffic is misleading in a specific way: nearly
every response is HTTP 200, including all the failures. A dashboard built
on HTTP status codes shows a perfectly healthy service while every RPC
returns `grpc-status: 13` (INTERNAL).

Record `grpc-status` as its own metric dimension, and take the method name
from the path (`/package.Service/Method`) as the route label rather than
treating each as a unique URL (`08-observability/02-metrics.md`).

## Practice
Build these in order.

1. Stand up a real gRPC upstream with `tonic`, including one method that
   returns a non-OK status and one server-streaming method. **Done when**
   a `grpcurl` client talks to it directly and sees both behaviors.
2. In `labs/05-reverse-proxy`, proxy it end-to-end. **Done when** a unary
   call succeeds through the proxy and the non-OK `grpc-status` reaches
   the client as a proper status — not a transport error and not a false
   success.
3. Add a trailers-only case. **Done when** an RPC that fails immediately
   (no DATA frames at all) is proxied correctly rather than hanging.
4. Make the proxy's own errors gRPC-shaped. **Done when** killing every
   upstream produces `grpc-status: 14 UNAVAILABLE` at the client rather
   than an HTTP 503 body, and a rate-limited call produces
   `8 RESOURCE_EXHAUSTED`.
5. Test client-streaming and bidi. **Done when** the upstream receives
   frames as the client sends them (verify with upstream-side timestamps)
   rather than all at once at stream end.
6. Honor `grpc-timeout`, decremented by time already spent, and exclude
   upgraded/streaming RPCs from request-shaped timeouts. **Done when** a
   server-streaming RPC emitting one message every 30 seconds survives
   indefinitely, and a client-set 100ms deadline is enforced by the
   upstream rather than by a proxy default.
7. Exclude `application/grpc` from response compression. **Done when** a
   gRPC response passes through byte-identical while an HTML response on
   the same proxy is still compressed.
8. Verify per-RPC load balancing. **Done when** one client connection
   issuing many RPCs distributes them across upstreams — if they all land
   on one, your balancer is picking per connection and needs fixing.
9. Add gRPC health checking (`Check`, then `Watch`) and `grpc-status` as a
   metric dimension labelled by `/package.Service/Method`. **Done when** a
   dashboard shows an upstream returning all-INTERNAL as unhealthy, even
   though every HTTP status is 200.
