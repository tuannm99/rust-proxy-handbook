# 09-http3

## Goal

A basic QUIC endpoint using `quinn`, as the transport HTTP/3 replaces TCP
with — no head-of-line blocking across streams, built-in encryption.

## Handbook references
- `instruction/01-network/12-http3.md`
- `instruction/19-reading-source/quinn/` — read `quinn`'s own source alongside this lab

## Run

```
cargo run -p http3
```
