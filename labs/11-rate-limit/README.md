# 11-rate-limit

Per-client rate limiting using token bucket / sliding window / leaky
bucket, returning a proper 429 with `Retry-After` when exceeded.

Handbook references:
- `instruction/07-security/ratelimit.md`
- `instruction/13-algorithms/token-bucket.md`, `sliding-window.md`, `leaky-bucket.md`

Run with:

```
cargo run -p rate-limit
```
