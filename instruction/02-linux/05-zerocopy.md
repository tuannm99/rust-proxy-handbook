# Zero-copy

sendfile, splice, mmap.

## What to learn

### The copy you're trying to avoid
A naive "serve a file over a socket" does: read() copies file data from
kernel page cache into a userspace buffer, then write() copies it back from
userspace into the kernel's socket buffer. That's two copies and two
context-switch round trips for data your process never actually needed to
touch. Zero-copy syscalls let the kernel move data page-cache-to-socket
directly.

### sendfile
```rust
// libc::sendfile(out_fd, in_fd, offset, count)
// out_fd must be a socket (or similar); in_fd must be a regular file.
let sent = unsafe { libc::sendfile(sock_fd, file_fd, std::ptr::null_mut(), len) };
```
This is the classic zero-copy primitive for "serve a static file" and is
exactly what `05-http-stack/05-static.md` should reach for on the happy path.
Gotcha: `sendfile` requires the source to be a *file* — you cannot
`sendfile` socket-to-socket, which matters for a reverse proxy relaying
upstream responses.

### splice and vectored I/O
`splice(2)` moves data between two fds *where at least one is a pipe*,
without copying through userspace — this is how you get zero-copy
socket-to-socket relaying (client-socket -> pipe -> upstream-socket) for a
proxy, unlike `sendfile`. `writev`/`readv` (vectored I/O, exposed in Rust via
`IoSlice`/`IoSliceMut` and `AsyncWrite::poly_write_vectored`) let you write
multiple non-contiguous buffers (e.g. a header you built plus a body you're
streaming) in one syscall instead of concatenating them into one buffer
first.

### mmap
`mmap`-ing a file maps its pages directly into your address space backed by
the page cache — reads fault pages in lazily, and the OS handles caching for
you. Useful for large static assets you'll access randomly (not strictly
sequentially), but gotcha: a `mmap`'d file that gets truncated or modified
out from under you while mapped can `SIGBUS` your process on the next
access — dangerous for a long-running proxy serving user-uploaded or
frequently-rewritten files; `sendfile` doesn't have this failure mode.

### Why zero-copy is hard with TLS
None of these syscalls know anything about TLS — `sendfile`/`splice` move
*ciphertext-oblivious* bytes, but TLS requires encrypting data in userspace
before it hits the wire, which means the data has to pass through a
userspace buffer for encryption anyway. Kernel TLS (`kTLS`, `setsockopt`
with `SOL_TLS`) pushes the encrypt/decrypt step itself into the kernel so
`sendfile` can work again even with TLS, but it's a newer, narrower-support
feature (needs kernel + often specific NIC offload support) and Rust
ecosystem support (`ktls`, ties into `rustls`) is much less mature than
plain `rustls`. In practice: `proxy` terminating TLS
(`01-network/07-tls.md`) will do a userspace copy-and-encrypt on the response
path unless you deliberately reach for kTLS, and that's a normal, acceptable
default — don't treat losing zero-copy under TLS as a bug.

## Practice
1. Write a tiny static file server using raw `libc::sendfile` and benchmark it against a naive read+write loop with `wrk` for a large file.
2. Implement socket-to-socket relaying via `splice` for a bare TCP proxy and confirm (via `strace`) that no userspace buffer copy occurs.
3. `mmap` a file, read from the mapping, then truncate the file from another process and observe the `SIGBUS`.
4. Use vectored writes (`IoSlice`) in `labs/04-static-server` to write a response header and body in one syscall instead of concatenating buffers.
5. Read up on kTLS support in `rustls`/the `ktls` crate and write a short note on whether it's worth pursuing for `proxy`.
