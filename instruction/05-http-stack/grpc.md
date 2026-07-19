# gRPC Proxying

gRPC is HTTP/2 with a specific framing convention on top — proxying it correctly means not breaking that convention, not understanding protobuf.

## What to learn
### gRPC's message framing inside HTTP/2 DATA frames
Each gRPC message is a 1-byte compression flag, a 4-byte big-endian length prefix, then that many bytes of protobuf payload — all carried inside ordinary HTTP/2 `DATA` frames (see `01-network/http2.md`). A proxy forwarding gRPC does not need to understand protobuf or even this framing; it only needs to forward `DATA` frames faithfully, byte-for-byte, without doing anything an HTTP/1.1-oriented code path might reflexively do (buffering the whole body to compute `Content-Length`, for instance — gRPC bodies are length-prefixed per-message, not once for the whole stream, and are often unbounded/streaming).

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

### Streaming semantics: don't impose request/response buffering
gRPC has four call shapes: unary, client-streaming, server-streaming, and bidirectional streaming — all of them are just an HTTP/2 stream carrying however many DATA frames in either direction before trailers close it. A proxy that buffers a full request body before forwarding (reasonable for a small HTTP/1.1 POST) breaks client-streaming and bidi calls outright, since the upstream may need to start responding before the client finishes sending. Forward frames as they arrive; don't wait for stream-end unless you have a specific reason to.

### Load balancing gRPC is not the same problem as load balancing HTTP/1.1
A gRPC client typically opens one long-lived HTTP/2 connection and multiplexes many independent RPCs over it (see `01-network/http2.md`'s multiplexing). A load balancer that picks an upstream *per connection* (like a plain L4/TCP balancer, or a naive `06-proxy/load-balancer.md` implementation written with one-request-per-connection HTTP/1.1 in mind) sends every RPC on that connection to the same upstream forever, defeating load balancing entirely once a client connects. Correct gRPC load balancing has to be aware of individual HTTP/2 streams and pick an upstream per-RPC, not per-connection.

## Practice
1. In `milestones/03-reverse-proxy`, confirm HTTP/2 trailers are forwarded end-to-end when proxying to an upstream — set up a real gRPC server (e.g. with `tonic`) that returns a non-OK `grpc-status`, and verify the client sees it through the proxy, not a false "success."
2. Add a client-streaming test case (client sends multiple messages before the server responds) and confirm the proxy doesn't buffer the whole request before forwarding — the upstream should start receiving frames as they arrive.
3. Check your `06-proxy/load-balancer.md` implementation: confirm it's picking an upstream per-RPC (per HTTP/2 stream), not per-connection, by opening one client connection and issuing multiple RPCs, then verifying they can land on different upstreams.
4. Add a metric (`08-observability/metrics.md`) that specifically tracks `grpc-status` codes forwarded through the proxy, separate from the HTTP status code — a 200 with a non-OK `grpc-status` should be visible as a distinct signal.
