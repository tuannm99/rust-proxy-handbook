# 07-tls

## Goal

Terminate TLS in front of an HTTP server using `tokio-rustls`, with ALPN
picking HTTP/1.1 vs HTTP/2 per client.

## Handbook references
- `instruction/01-network/07-tls.md` — handshake, SNI, ALPN, session resumption

## Run

```
cargo run -p tls
```
