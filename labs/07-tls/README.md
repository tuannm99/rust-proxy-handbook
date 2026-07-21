# 07-tls

Terminate TLS in front of an HTTP server using `tokio-rustls`, with ALPN
picking HTTP/1.1 vs HTTP/2 per client.

Handbook references:
- `instruction/01-network/tls.md` — handshake, SNI, ALPN, session resumption

Run with:

```
cargo run -p tls
```
