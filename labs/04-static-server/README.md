# 04-static-server

## Goal

A static file server that streams file bodies from disk rather than
buffering them fully in memory, with correct `Content-Type` and no path
traversal.

## Done when

- [ ] Serving a 1 GB file keeps the server's RSS within a few tens of MB of idle (measure with `/proc/<pid>/status` during the download).
- [ ] `Content-Type` comes from the file extension, with `application/octet-stream` as the fallback.
- [ ] Missing files return `404`; directories return `404` or a deliberate index behavior, not a crash.
- [ ] Traversal is rejected in every form: `../`, percent-encoded `%2e%2e%2f`, and a symlink inside the root pointing outside it.
- [ ] `Range: bytes=0-99` returns `206` with a correct `Content-Range`; an unsatisfiable range returns `416`.
- [ ] `HEAD` returns the same headers as `GET` with no body.
- [ ] Stretch: a `sendfile`-based path ([`instruction/02-linux/11-zerocopy.md`](../../instruction/02-linux/11-zerocopy.md)) benchmarked against the `tokio::fs` path.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/05-http-stack/06-static.md`](../../instruction/05-http-stack/06-static.md) — streaming files, `Content-Type` from extension, range requests
- [`instruction/02-linux/11-zerocopy.md`](../../instruction/02-linux/11-zerocopy.md) — get correctness first with `tokio::fs`, then try `sendfile`
- [`instruction/07-security/04-normalization.md`](../../instruction/07-security/04-normalization.md) — why path checks must run on the normalized path
- [`instruction/05-http-stack/02-hyper.md`](../../instruction/05-http-stack/02-hyper.md) — streaming bodies and why they keep RSS flat, what hyper does for `HEAD`
- [`instruction/12-testing/06-lab-environment.md`](../../instruction/12-testing/06-lab-environment.md) — measuring RSS during the download

## Run

```
cargo run -p static-server
```
