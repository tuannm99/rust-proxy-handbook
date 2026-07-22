# 00-tcp-server

## Goal

A tokio TCP server that accepts many concurrent clients on one port and
echoes back exactly the bytes each client sends, in order, until that
client closes the connection. Done means: it holds 500+ idle connections
without spawning 500+ OS threads, a slow client can't stall other clients,
and a client half-closing its write side still gets its pending echoes
flushed before the connection is torn down.

## Handbook references
- `instruction/01-network/socket.md` — bind/listen/accept, `SO_REUSEADDR`
- `instruction/01-network/tcp.md` — 3-way handshake, byte-stream framing (short reads/writes)
- `instruction/03-rust/ownership.md`, `instruction/03-rust/async.md` — one owned task per connection
- `instruction/04-runtime/tokio.md` — multi-threaded scheduler, `tokio::spawn` per connection
- `instruction/02-linux/epoll.md` — what tokio's reactor is doing under the hood (read this alongside the lab, there's no separate raw-epoll lab in this workspace)

## Run

```
cargo run -p tcp-server
```
