# Static File Serving

## What to learn

### Zero-copy file sending
Naively serving a static file means: read the whole file into a userspace buffer, then write that buffer to the socket — two copies and two context switches more than necessary. `sendfile(2)` (see `02-linux/05-zerocopy.md`) copies data kernel-to-kernel, bypassing userspace entirely; on Linux, `tokio-uring`/`io_uring` (`02-linux/02-io_uring.md`) can do the same asynchronously with less syscall overhead than epoll-based `sendfile`.

Gotcha: `sendfile` is fast on a page-cache *hit* and blocking on a miss
(`16-kernel/08-page-cache.md`). In an async runtime, a blocking `sendfile` on
a cold file stalls the entire worker thread and every other connection it
was multiplexing — so the "obvious optimization" turns a slow file read
into a latency spike across unrelated requests. This is exactly why
`tokio::fs` dispatches to a blocking thread pool instead: it costs a copy
and keeps the reactor responsive.

The practical rule: zero-copy pays when the working set is hot and hurts
when it isn't. Measure with a working set larger than RAM before
committing to it.

### The filesystem is blocking, and it's everywhere
Beyond the file body, a static handler makes several *metadata* syscalls
per request — `stat` for size and mtime, `canonicalize` for path safety,
`open` itself. Every one of them can block on a cold dentry cache or a
slow/networked filesystem, and none of them look like I/O when you read
the code.

`tokio::fs` wraps these in `spawn_blocking`, which is correct but not
free: the blocking pool is bounded (512 threads by default), and a burst
of static requests against a slow filesystem can saturate it — at which
point *every* `spawn_blocking` user in the process, including unrelated
ones, queues behind them.

Two mitigations worth knowing: cache metadata (and even open file
descriptors) for hot files, the way nginx's `open_file_cache` does, so
repeat requests skip the syscalls entirely; and bound static-file
concurrency separately from overall request concurrency
(`07-security/09-ddos.md`), so a cold-filesystem stall can't consume the
whole process.

Gotcha: an fd cache is bounded by `ulimit -n` and must evict
(`13-algorithms/lru.md`), and it must key on something that detects file
replacement (device + inode, not path) — a deploy that swaps a file
leaves you serving the old fd's contents forever.

### Range requests
Clients (browsers resuming a download, video players seeking) send `Range: bytes=1000-1999`. A compliant server responds `206 Partial Content` with `Content-Range`, or `416 Range Not Satisfiable` if the range is invalid — and must handle multi-range requests (`bytes=0-99,200-299`) or explicitly refuse them via `Accept-Ranges: none`.

Gotcha: multi-range is an amplification vector. A request listing hundreds
of tiny overlapping ranges forces the server to build a large
`multipart/byteranges` response — far more output than input, plus the CPU
to assemble it. This has been a real CVE in both Apache and nginx. Cap the
number of ranges you'll honor (a handful), reject or coalesce overlapping
ones, and consider answering multi-range with the full body instead, which
is always permitted.

Gotcha: `If-Range` exists so a resumed download doesn't silently splice
together two different versions of a file. If the validator doesn't match,
you must return the *entire* file (200), not the requested range.

### Conditional requests: ETag & If-Modified-Since
Serving a full 200 response to a client that already has an up-to-date cached copy wastes bandwidth. An `ETag` (a hash or version stamp of the file) or `Last-Modified` timestamp lets the client send `If-None-Match`/`If-Modified-Since`; if unchanged, respond `304 Not Modified` with no body.

```rust
// sketch: conditional check before touching the file body at all
fn is_not_modified(etag: &str, if_none_match: Option<&str>) -> bool {
    if_none_match.is_some_and(|inm| inm == etag)
}
```

Gotcha: how you generate the ETag matters more than it appears. Hashing
file contents per request is correct and costs a full read — defeating the
point. Deriving it from `(inode, size, mtime)` is what nginx and most
servers do: cheap, and stable as long as your deploys actually change
mtime. But `mtime` has one-second granularity on some filesystems, so a
file modified twice within the same second keeps its ETag and clients
serve stale content indefinitely. Use nanosecond precision where
available, or include a content hash computed once at startup/deploy.

Gotcha: know the difference between weak (`W/"abc"`) and strong ETags.
Range requests require a *strong* validator — an `If-Range` against a weak
ETag must be treated as a mismatch, because "semantically equivalent" is
not good enough when splicing bytes.

### Path traversal
`GET /../../etc/passwd` or an encoded equivalent (`%2e%2e%2f`) must never resolve outside the served root. The safe approach is to canonicalize the resolved path (`std::fs::canonicalize`) and verify it's still a descendant of the root directory — string-matching against `".."` is not sufficient (symlinks, encoding tricks).

Gotcha: canonicalize-then-open is a TOCTOU race. Between your check and
your open, a symlink can be swapped in — on a directory anyone else can
write to, that's a real exploit, not a theoretical one. The robust fix is
to resolve relative to an opened root directory using `openat2` with
`RESOLVE_BENEATH` (or `cap-std`, which wraps this pattern in Rust), so the
kernel enforces containment atomically instead of you checking a string.

Gotcha: the same normalization discussion as `05-http-stack/03-router.md`
applies here, and if the router already normalized the path, the static
handler must not decode it *again* — a double decode reintroduces
traversal from `%252e%252e%252f`. Decode exactly once, at a documented
place.

### Don't serve what you didn't mean to
Serving a directory root means serving everything under it, including what
you forgot was there. The recurring real-world leaks:
- **`.git/`** — a deployed working tree exposes full source history,
  including credentials committed and later removed.
- **`.env`, `config.yml`, `*.bak`, editor swap files** — secrets, plainly.
- **Directory listings** — off by default unless you deliberately want
  them, because a listing turns "attacker must guess filenames" into
  "attacker reads the index".

Deny dotfiles by default, serve from a directory that contains *only*
build output (not a repo checkout), and disable listings. Each of these is
one line and each has caused real incidents.

### MIME type & caching headers
Content-Type should come from a real extension→MIME table, not guessed from content. Static assets that are content-hashed in their filename (`app.a3f9c1.js`) can be served with a very long `Cache-Control: max-age=31536000, immutable`; unhashed files need a much shorter TTL or must rely on conditional requests.

Gotcha: always send `X-Content-Type-Options: nosniff`. Without it, browsers
may ignore your `Content-Type` and sniff the body — so a file you serve as
`text/plain` can be executed as `text/html` or JavaScript.

Gotcha: serving *user-uploaded* files from the same origin as your
application is a stored-XSS hole that no header fully closes — an uploaded
`.html` (or a file sniffed as one) runs with your origin's cookies. Serve
user content from a separate origin, or force `Content-Disposition:
attachment` for anything uploaded.

## Practice
Build these in order.

1. In `labs/04-static-server`, serve files from a root directory with
   `tokio::fs::read` plus a response body. **Done when** a known file is
   returned with the correct `Content-Type` from an extension table.
2. Write traversal tests before hardening: `../../../etc/passwd`,
   `%2e%2e%2f` variants, a double-encoded variant, and a symlink pointing
   outside the root. **Done when** at least one gets out — then add
   containment and **done again when** none do.
3. Replace canonicalize-then-open with `cap-std` (or `openat2` with
   `RESOLVE_BENEATH`). **Done when** a test that swaps a symlink between
   check and open cannot escape the root.
4. Deny dotfiles and disable listings. **Done when** `GET /.git/config`
   and `GET /` both 404 rather than revealing anything.
5. Add `ETag` from `(inode, size, mtime-with-nanos)` and
   `If-None-Match`/`If-Modified-Since` handling. **Done when** a repeat
   request gets `304` with an empty body, and modifying a file twice
   within one second still produces a changed ETag.
6. Add `Range` with `206`/`416`/`Content-Range`, `If-Range`, and a cap on
   multi-range counts. **Done when** a video player can seek, an invalid
   range gets 416, and a 500-range request is refused rather than
   assembled.
7. Add `nosniff` and long-lived `immutable` caching for content-hashed
   filenames only. **Done when** hashed assets get a year-long TTL and
   unhashed ones don't.
8. Add an open-file/metadata cache keyed by device+inode with LRU
   eviction. **Done when** repeat requests for a hot file make no `stat`
   syscall (verify with `strace -c`), and replacing the file on disk is
   picked up rather than served stale.
9. Swap the naive read+write for `sendfile` or `io_uring`. **Done when**
   throughput improves on a hot working set, and you have measured the
   *cold* case too — if p99 gets worse when the working set exceeds RAM,
   you've found the gotcha above and should say so in your notes.
