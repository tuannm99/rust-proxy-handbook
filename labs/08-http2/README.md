# 08-http2

## Goal

Exercise HTTP/2-specific behavior — stream multiplexing over one
connection, flow-control windows, stream limits, and graceful `GOAWAY` —
that HTTP/1.1 doesn't have.

## Done when

- [ ] Many concurrent requests from one client (`h2load` or `curl --http2 --parallel`) complete over a single TCP connection (confirm with `ss`).
- [ ] A client reading a large response slowly causes backpressure: the server's memory stays flat instead of buffering the whole body.
- [ ] With `SETTINGS_MAX_CONCURRENT_STREAMS` set low (for example 2), extra streams are held back by the protocol rather than all running at once.
- [ ] A client that opens and immediately resets streams in a loop (Rapid Reset) gets its connection closed rather than consuming unbounded server work ([`instruction/01-network/11-http2.md`](../../instruction/01-network/11-http2.md)).
- [ ] Graceful shutdown sends `GOAWAY`: in-flight streams finish, new streams are refused.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/01-network/11-http2.md`](../../instruction/01-network/11-http2.md)
- [`instruction/09-architecture/04-graceful-shutdown.md`](../../instruction/09-architecture/04-graceful-shutdown.md) — `GOAWAY` as part of draining

## Run

```
cargo run -p http2
```
