# Static File Serving

## What to learn

### Zero-copy file sending
Naively serving a static file means: read the whole file into a userspace buffer, then write that buffer to the socket — two copies and two context switches more than necessary. `sendfile(2)` (see `02-linux/zerocopy.md`) copies data kernel-to-kernel, bypassing userspace entirely; on Linux, `tokio-uring`/`io_uring` (`02-linux/io_uring.md`) can do the same asynchronously with less syscall overhead than epoll-based `sendfile`.

### Range requests
Clients (browsers resuming a download, video players seeking) send `Range: bytes=1000-1999`. A compliant server responds `206 Partial Content` with `Content-Range`, or `416 Range Not Satisfiable` if the range is invalid — and must handle multi-range requests (`bytes=0-99,200-299`) or explicitly refuse them via `Accept-Ranges: none`.

### Conditional requests: ETag & If-Modified-Since
Serving a full 200 response to a client that already has an up-to-date cached copy wastes bandwidth. An `ETag` (a hash or version stamp of the file) or `Last-Modified` timestamp lets the client send `If-None-Match`/`If-Modified-Since`; if unchanged, respond `304 Not Modified` with no body.

```rust
// sketch: conditional check before touching the file body at all
fn is_not_modified(etag: &str, if_none_match: Option<&str>) -> bool {
    if_none_match.is_some_and(|inm| inm == etag)
}
```

### Path traversal
`GET /../../etc/passwd` or an encoded equivalent (`%2e%2e%2f`) must never resolve outside the served root. The safe approach is to canonicalize the resolved path (`std::fs::canonicalize`) and verify it's still a descendant of the root directory — string-matching against `".."` is not sufficient (symlinks, encoding tricks).

### MIME type & caching headers
Content-Type should come from a real extension→MIME table, not guessed from content. Static assets that are content-hashed in their filename (`app.a3f9c1.js`) can be served with a very long `Cache-Control: max-age=31536000, immutable`; unhashed files need a much shorter TTL or must rely on conditional requests.

## Practice
1. In `labs/04-static-server`, serve files from a root directory using a plain `tokio::fs::read` + response body first (correctness before performance).
2. Add path traversal protection: canonicalize and verify containment, with a test asserting `../../../etc/passwd`-style requests are rejected.
3. Add `ETag`/`If-None-Match` support and verify a repeated request returns `304` with an empty body.
4. Add `Range` support and verify `206` responses and multi-range or invalid-range edge cases.
5. Swap the naive read+write for `sendfile` (via a crate like `tokio-sendfile` or manual `libc::sendfile` behind a `spawn_blocking`, or `tokio-uring`) and compare throughput under load (`12-testing/load-testing.md`).
