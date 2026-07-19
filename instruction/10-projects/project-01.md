# Project 1
TCP Echo Server.

## Goal
A tokio TCP server that accepts many concurrent clients on one port and
echoes back exactly the bytes each client sends, in order, until that client
closes the connection. Concretely "done" means: it holds 500+ idle
connections without spawning 500+ OS threads, a slow client can't stall
other clients, and a client half-closing its write side still gets its
pending echoes flushed before the connection is torn down.

## What to learn
- `01-network/socket.md` — bind/listen/accept, what `SO_REUSEADDR` does
- `01-network/tcp.md` — 3-way handshake, why TCP is a byte stream not a message stream (short reads/writes)
- `03-rust/ownership.md`, `03-rust/async.md` — why each connection needs its own owned task, not a shared borrow
- `04-runtime/tokio.md` — multi-threaded scheduler, `tokio::spawn` per connection
- `02-linux/epoll.md` — what tokio's reactor is doing under the hood (cross-reference after finishing `labs/epoll-echo`)

## Practice
Build `milestones/01-echo` incrementally:
1. Bind a `TcpListener`, `accept()` in a loop, handle exactly one connection at a time (no spawn yet) — confirm a second client blocks until the first disconnects.
2. `tokio::spawn` a task per accepted connection so clients run concurrently; verify with `nc` from two terminals at once.
3. In the per-connection loop, read into a fixed buffer and write back only the bytes actually read (handle short reads — don't assume one `read` = one message).
4. Handle `read() == 0` (EOF/half-close) by flushing and returning cleanly instead of looping forever or panicking.
5. Load test with many simultaneous connections (e.g. a small script opening 1000 sockets) and confirm memory/CPU stay flat — no thread-per-connection blowup.
