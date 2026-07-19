# gzip, brotli, zstd

## What to learn

### Trade-offs between algorithms
gzip (DEFLATE) is universally supported and cheap to decode but has the weakest compression ratio of the three. Brotli generally compresses text/HTML best and is well-supported in browsers but is slower to encode at high quality levels. zstd compresses and decompresses very fast at reasonable ratios and is increasingly used for internal/east-west traffic where CPU matters more than the last few percent of size; browser support for `Content-Encoding: zstd` is still not universal, so it's more useful proxy-to-upstream than proxy-to-browser.

### Negotiation via Accept-Encoding
The client lists what it can decode, optionally with quality weights: `Accept-Encoding: gzip, br;q=0.8`. The server (or proxy) picks one it supports, sets `Content-Encoding` on the response, and must add `Vary: Accept-Encoding` so any cache in front of it (see `05-http-stack/cache.md`) doesn't serve a gzip response to a client that only asked for brotli.

### Streaming vs buffer-then-compress
Compressing a full response in memory before sending it adds latency (the client waits for the whole body) and memory pressure for large bodies. Streaming compression (`flate2`'s `Compress` API, `async-compression`'s stream wrappers) compresses chunks as they're produced/forwarded, which matters a lot in a proxy that's relaying a large body from an upstream.

```rust
// sketch: streaming gzip over an async body using async-compression
use async_compression::tokio::bufread::GzipEncoder;
let compressed = GzipEncoder::new(upstream_body_reader);
// compressed implements AsyncRead; forward it straight to the client socket
```

### CPU cost under load
Compression is CPU-bound; at high request rates, encoding on every request can become the bottleneck before the network does. Two common mitigations: cache the compressed bytes for cacheable responses (compress once, serve many times) and skip compression below a minimum body size (compressing a 50-byte JSON response usually isn't worth the CPU).

### Don't double-compress
If the upstream already compressed the body (it sent `Content-Encoding: gzip`), the proxy must not compress it again — either pass it through as-is if the client accepts that encoding, or decompress-then-recompress only if the client needs a different encoding than the upstream provided.

## Practice
1. In `milestones/02-http`, add gzip response compression gated on the request's `Accept-Encoding`, setting `Content-Encoding` and `Vary` correctly.
2. Make it streaming rather than buffer-the-whole-body-then-compress, and verify memory use doesn't scale with body size for a large test file.
3. Add a minimum-size threshold below which compression is skipped, and measure the CPU cost difference under `12-testing/load-testing.md`.
4. Add brotli and zstd as additional negotiated encodings and implement `Accept-Encoding` quality-value (`q=`) selection.
5. In `milestones/03-reverse-proxy`, verify the proxy doesn't double-compress an already-compressed upstream response.
