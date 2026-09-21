# SIMD

Processing many bytes per instruction — where it shows up in a proxy
(header scanning, delimiter search, checksums) and when it is worth the
complexity. The end of the road that `17-performance/05-branch-prediction.md`
started down.

## What to learn

### The idea
SIMD (Single Instruction, Multiple Data) runs one operation across a vector
of lanes at once — compare 16 or 32 bytes to a delimiter in a single
instruction instead of a loop of 16–32 branchy comparisons. For the
byte-scanning a proxy does constantly (finding the CR/LF ending a header
line, the `:` splitting a header, an invalid character), this is a
several-fold speedup *and* it eliminates the per-byte branch that
`17-performance/05-branch-prediction.md` worried about.

```rust
// Conceptually: load 16 bytes, compare-equal to b'\n' across all lanes,
// extract a bitmask of matches, count trailing zeros to find the first.
// One pass over the 16 bytes, zero per-byte branches.
```

### Where it actually pays in a proxy
- **Delimiter search** in the HTTP parser (`05-http-stack/01-parser.md`):
  finding line and header boundaries. This is what `memchr` does, and it is
  the single highest-leverage SIMD use — you get it for free by using the
  `memchr` crate instead of a hand loop.
- **Validation:** checking a whole header value contains no control
  characters in one sweep.
- **Checksums/hashing:** CRC, and the hashing behind
  `13-algorithms/consistent-hash.md`/`maglev.md`, have SIMD-accelerated
  implementations.

Outside these specific byte-bashing spots, a proxy is dominated by I/O and
syscalls, not compute, so SIMD has little surface area. Do not go looking
for places to vectorize; use it where the hot path is genuinely a tight
loop over bytes.

### Getting it without writing intrinsics
Three tiers, in the order you should prefer them:

1. **Use a crate that already did it.** `memchr` (delimiter search),
   `simd-json`, `aho-corasick` (`13-algorithms/aho-corasick.md`, which uses
   SIMD internally for the WAF scan in `07-security/06-waf.md`). This covers
   almost every real proxy need with zero unsafe code.
2. **Autovectorization.** Write a simple, branchless, straight loop over a
   slice and let the compiler vectorize it (`-C target-cpu=native`, or
   `target_feature`). Check with `cargo asm` that it actually emitted
   vector instructions — small changes silently disable it.
3. **Portable SIMD / intrinsics.** `std::simd` (portable) or
   `std::arch` intrinsics only when the above are not enough. Intrinsics
   are `unsafe` (`03-rust/03-unsafe.md`) and CPU-specific.

### Runtime feature detection is mandatory
A binary compiled with AVX2 crashes with SIGILL on a CPU that lacks it. If
you hand-write SIMD you must detect the feature at runtime
(`is_x86_feature_detected!`) and dispatch to a scalar fallback — or restrict
`target-cpu` and accept the binary will not run on older hardware. The
crates in tier 1 handle this for you, which is another reason to prefer
them.

Gotcha: SIMD is the last optimization, not an early one. It is complex,
CPU-specific, and easy to get subtly wrong (tail handling when the input
is not a multiple of the lane width is a classic bug source). Reach for it
only after `08-observability/04-profiling.md` shows a byte-scanning loop is a
top hot spot under real load (`12-testing/01-load-testing.md`), and even then
try the `memchr`/`aho-corasick` route before writing a single intrinsic.

## Practice
1. Replace a hand-written byte-search loop in your HTTP parser
   (`05-http-stack/01-parser.md`) with `memchr` and benchmark the header-scan
   path; this is the highest-value, lowest-risk SIMD change.
2. Write a simple branchless validation loop (no control chars in a header
   value), compile with `target-cpu=native`, and use `cargo asm` to confirm
   the compiler autovectorized it.
3. Deliberately add a branch inside that loop and observe autovectorization
   disappear — learn what defeats the compiler.
4. Confirm `aho-corasick` (`13-algorithms/aho-corasick.md`) is using SIMD
   under your WAF workload (`07-security/06-waf.md`) and compare its throughput
   against a naive multi-substring scan.
5. Only if a profile still demands it: write one `std::simd` delimiter
   search with a scalar tail and a runtime feature check, and verify it
   matches the scalar version on inputs of every length mod the lane
   width — then reflect on whether the crate route would have sufficed.
