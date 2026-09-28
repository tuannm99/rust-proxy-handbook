# 09-http3

## Goal

A basic QUIC endpoint using `quinn`, as the transport HTTP/3 replaces TCP
with — independent streams without cross-stream head-of-line blocking,
encryption built in.

## Done when

- [ ] A `quinn` server and client exchange data over a bidirectional stream, using a self-signed certificate the client is configured to trust.
- [ ] Several streams run concurrently on one connection, and a deliberately stalled stream doesn't block the others.
- [ ] Two clients connect at once and you log the connection-ID demultiplexing on the server's single UDP socket.
- [ ] You checked `net.core.rmem_max`, pushed enough traffic to see drops in `netstat -su`, raised the buffer, and saw the drops stop ([`instruction/01-network/13-http3.md`](../../instruction/01-network/13-http3.md)).
- [ ] Stretch: serve one HTTP/3 request using the `h3` crate on top of `quinn`.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/01-network/13-http3.md`](../../instruction/01-network/13-http3.md)
- [`instruction/19-reading-source/quinn/`](../../instruction/19-reading-source/quinn) — read `quinn`'s own source alongside this lab
- [`instruction/01-network/14-tls.md`](../../instruction/01-network/14-tls.md) — making the CA and leaf certificate the quinn client will trust

## Run

```
cargo run -p http3
```
