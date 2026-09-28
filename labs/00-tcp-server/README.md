# 00-tcp-server

## Goal

A tokio TCP server that accepts many concurrent clients on one port and
echoes back exactly the bytes each client sends, in order, until that
client closes the connection.

## Done when

- [ ] Lines typed into `nc 127.0.0.1 <port>` come back byte-for-byte, in order.
- [ ] A 10 MB random file piped through the server comes back with an identical checksum (`sha256sum` both sides) — this is what catches mishandled partial reads and writes.
- [ ] 1000 concurrent idle connections are held open while the process's OS thread count stays at the tokio worker count (check `Threads:` in `/proc/<pid>/status`).
- [ ] A client that sends data but never reads its echoes does not stall echoes to any other client.
- [ ] A client that half-closes its write side (`shutdown(Write)`) still receives every pending echo before the server closes the connection.
- [ ] With `ulimit -n 64` and a flood of connection attempts, the process doesn't spin at 100% CPU, and it resumes accepting once connections drop ([`instruction/07-security/09-ddos.md`](../../instruction/07-security/09-ddos.md)'s `EMFILE` gotcha).
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5, and every finding fixed or argued.

## Handbook references
- [`instruction/01-network/01-fundamentals.md`](../../instruction/01-network/01-fundamentals.md), [`instruction/02-linux/01-fundamentals.md`](../../instruction/02-linux/01-fundamentals.md) — read first if "port," "syscall," "file descriptor," or "handshake" aren't already precise terms for you; every reference below assumes them
- [`instruction/01-network/07-socket.md`](../../instruction/01-network/07-socket.md) — bind/listen/accept, `SO_REUSEADDR`
- [`instruction/01-network/08-tcp.md`](../../instruction/01-network/08-tcp.md) — 3-way handshake, byte-stream framing (short reads/writes)
- [`instruction/03-rust/01-ownership.md`](../../instruction/03-rust/01-ownership.md), [`instruction/03-rust/05-async.md`](../../instruction/03-rust/05-async.md) — one owned task per connection
- [`instruction/04-runtime/01-tokio.md`](../../instruction/04-runtime/01-tokio.md) — multi-threaded scheduler, `tokio::spawn` per connection
- [`instruction/02-linux/07-epoll.md`](../../instruction/02-linux/07-epoll.md) — what tokio's reactor is doing under the hood (read this alongside the lab, there's no separate raw-epoll lab in this workspace)
- [`instruction/12-testing/05-debugging.md`](../../instruction/12-testing/05-debugging.md) — `nc`, `ss`, `strace`, `tcpdump`; its Practice section uses this lab

## After you finish
- Read [`instruction/19-reading-source/tokio/reading-guide.md`](../../instruction/19-reading-source/tokio/reading-guide.md) stops 1-4 and compare tokio's I/O path and scheduler with what you assumed while building this.

## Run

```
cargo run -p tcp-server
```
