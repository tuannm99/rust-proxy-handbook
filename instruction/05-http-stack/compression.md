# gzip, brotli, zstd

## What to learn

### Trade-offs between algorithms
gzip (DEFLATE) is universally supported and cheap to decode but has the weakest compression ratio of the three. Brotli generally compresses text/HTML best and is well-supported in browsers but is slower to encode at high quality levels. zstd compresses and decompresses very fast at reasonable ratios and is increasingly used for internal/east-west traffic where CPU matters more than the last few percent of size; browser support for `Content-Encoding: zstd` is still not universal, so it's more useful proxy-to-upstream than proxy-to-browser.

Gotcha: the quality level dominates this comparison, and the top levels
are traps for dynamic content. Brotli quality 11 can be an order of
magnitude slower to encode than quality 4-5 for a few percent better
ratio — fine when you compress once at build time and serve forever
(`05-http-stack/static.md`'s content-hashed assets), never worth it when
compressing a per-request response. Use high quality for precompressed
static files, low-to-middle for anything dynamic.

### Negotiation via Accept-Encoding
The client lists what it can decode, optionally with quality weights: `Accept-Encoding: gzip, br;q=0.8`. The server (or proxy) picks one it supports, sets `Content-Encoding` on the response, and must add `Vary: Accept-Encoding` so any cache in front of it (see `05-http-stack/cache.md`) doesn't serve a gzip response to a client that only asked for brotli.

Gotcha: handle the q-value edge cases, because they're how the negotiation
gets subtly wrong. `q=0` means explicitly *not* acceptable, not "lowest
preference" — `gzip;q=0` is a refusal. `identity;q=0` means the client
refuses uncompressed, and `*;q=0` refuses everything not otherwise listed;
if you can't satisfy the request you're supposed to return `406`, though
in practice serving `identity` anyway is the common pragmatic choice.

Gotcha: forgetting `Vary: Accept-Encoding` is a correctness bug that only
appears once a cache is involved — and then it serves brotli bytes to a
client that can only decode gzip, which surfaces as mysterious corruption
rather than a clean error.

### What not to compress
Compression has a floor and a ceiling, both worth enforcing:
- **Already-compressed formats** — JPEG, PNG, WebP, MP4, zip, and anything
  the upstream already encoded — gain essentially nothing and cost full
  CPU. Gate on `Content-Type`, not just size.
- **Tiny bodies.** Below roughly 1 KB, gzip's framing overhead can make
  the response *larger*, and the CPU is pure waste.
- **Already-encrypted or random data** compresses to slightly more than it
  started with, for the same reason.

### Streaming vs buffer-then-compress
Compressing a full response in memory before sending it adds latency (the client waits for the whole body) and memory pressure for large bodies. Streaming compression (`flate2`'s `Compress` API, `async-compression`'s stream wrappers) compresses chunks as they're produced/forwarded, which matters a lot in a proxy that's relaying a large body from an upstream.

```rust
// sketch: streaming gzip over an async body using async-compression
use async_compression::tokio::bufread::GzipEncoder;
let compressed = GzipEncoder::new(upstream_body_reader);
// compressed implements AsyncRead; forward it straight to the client socket
```

Gotcha: once you compress while streaming, **the upstream's
`Content-Length` is wrong** — it describes the uncompressed body, and you
no longer know the compressed length until you're done. You must remove
`Content-Length` and switch to chunked transfer encoding (HTTP/1.1) or
rely on frame lengths (HTTP/2). Forwarding a stale `Content-Length`
alongside a compressed body is precisely the framing disagreement that
`07-security/request-smuggling.md` is about — this is one of the most
common ways a proxy manufactures one by accident.

Gotcha: flushing is a latency/ratio trade. A compressor that never flushes
buffers data for better ratios, which stalls streaming responses (SSE,
long-poll, `05-http-stack/websocket.md`-adjacent patterns) — the client
waits for output that's sitting in the compressor. Flush at meaningful
boundaries for streaming content types; don't for bulk downloads.

### CPU cost under load
Compression is CPU-bound; at high request rates, encoding on every request can become the bottleneck before the network does. Two common mitigations: cache the compressed bytes for cacheable responses (compress once, serve many times) and skip compression below a minimum body size (compressing a 50-byte JSON response usually isn't worth the CPU).

Gotcha: compression is also the classic way to accidentally block an async
runtime. Compressing a large buffer synchronously inside a task holds the
worker thread for the whole operation (`03-rust/async.md`'s cooperative
scheduling), stalling every other connection on it. Either use a streaming
encoder that yields between chunks, or push large compressions to
`spawn_blocking`.

### BREACH: compression plus secrets is a side channel
If a response body contains both a **secret** (a CSRF token, an API key)
and **attacker-controlled reflected input**, compressing it leaks the
secret. The attacker submits guesses as input; when a guess matches part
of the secret, the two strings compress together and the response gets
measurably *smaller*. Repeat byte by byte and the secret falls out — over
HTTPS, since response length is visible regardless of encryption. This is
the BREACH attack, and it is why response compression is not an
unconditional win.

The mitigations, in order of practicality: don't reflect user input into
responses that contain secrets; separate secrets from compressible
content; disable compression for authenticated responses that reflect
input; or add random-length padding so lengths stop being a clean signal
(masking rather than fixing). For a proxy, the actionable version is to
make compression configurable per route so a team can turn it off for the
endpoints where this applies.

Gotcha: this is a real consideration, not a reason to disable compression
globally — the vast majority of responses contain no secrets, and turning
off compression everywhere is a large, certain cost against a narrow,
conditional risk.

### Don't double-compress
If the upstream already compressed the body (it sent `Content-Encoding: gzip`), the proxy must not compress it again — either pass it through as-is if the client accepts that encoding, or decompress-then-recompress only if the client needs a different encoding than the upstream provided.

Gotcha: when you *do* decompress an upstream response, you've taken on the
decompression-bomb risk from `07-security/ddos.md` — bound the
decompressed size and the expansion ratio, and stream rather than
materializing the whole thing. The same applies to compressed *request*
bodies you decompress for WAF inspection (`07-security/waf.md`).

## Practice
Build these in order.

1. In `labs/02-http-server`, add gzip response compression gated on
   `Accept-Encoding`, setting `Content-Encoding` and `Vary`. **Done when**
   a client sending `Accept-Encoding: gzip` gets a compressed body that
   `curl --compressed` decodes correctly, and one sending nothing gets
   plaintext.
2. Make it streaming and drop `Content-Length` in favour of chunked.
   **Done when** memory use is flat while serving a 1 GB body, and the
   response carries no stale `Content-Length` — inspect the raw bytes with
   `curl --raw` to confirm, since this is where accidental smuggling
   originates.
3. Add content-type and minimum-size gates. **Done when** a 50-byte JSON
   response and a JPEG both come back uncompressed while HTML does not,
   and you've measured that compressing the JPEG gained under 1%.
4. Add brotli and zstd with full q-value negotiation, including `q=0` and
   `*`. **Done when** `gzip;q=0, br` selects brotli, `*;q=0` is handled
   deliberately, and your selection logic has a test per edge case.
5. Measure the CPU cost. **Done when** you have throughput numbers at
   brotli quality 4 vs 11 under load (`12-testing/load-testing.md`) and
   can state the ratio gained for the CPU spent.
6. Cache compressed bytes for cacheable responses (`05-http-stack/cache.md`).
   **Done when** repeat requests for the same resource compress zero times.
7. In `labs/05-reverse-proxy`, handle an already-compressed upstream
   response. **Done when** a gzip'd upstream body is passed through
   untouched for a gzip-accepting client, and transcoded exactly once for
   a brotli-only client — with a bounded decompression step.
8. (Stretch) Demonstrate BREACH on a toy endpoint that reflects a query
   parameter next to a secret, compressed. **Done when** you can recover
   the secret from response lengths alone — then add per-route
   compression disabling and confirm the signal disappears.
