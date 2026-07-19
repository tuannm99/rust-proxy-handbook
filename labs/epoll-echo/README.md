# epoll-echo

Small standalone lab: a TCP echo server built directly on `epoll(7)` via
`libc`, with no async runtime. The point is to see what tokio's reactor does
for you under the hood.

Handbook references:
- `instruction/02-linux/epoll.md`
- `instruction/01-network/socket.md`

Run with:

```
cargo run -p epoll-echo
```
