# 04-static-server

## Goal

A static file server that streams file bodies from disk rather than
buffering them fully in memory, with correct `Content-Type` and no path
traversal. Done means: large files don't spike memory usage, missing files
404, and a `../` path is rejected.

## Handbook references
- `instruction/05-http-stack/static.md` — streaming files, `Content-Type` from extension, range requests
- `instruction/02-linux/zerocopy.md` — get correctness first with `tokio::fs`, then try `sendfile`

## Run

```
cargo run -p static-server
```
