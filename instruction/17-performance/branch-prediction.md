# Branch Prediction

Why an unpredictable branch on a hot path costs far more than the one
comparison it looks like. Like the rest of `17-performance/`, chase this
only after a profile points at it.

## What to learn

### The mechanism
Modern CPUs pipeline ~15–20 instructions deep and start executing past a
branch *before* they know which way it goes, by predicting the direction.
A correct prediction is nearly free. A misprediction throws away all the
speculative work and refills the pipeline — a stall of ~15–20 cycles. A
branch the predictor gets right 99% of the time is essentially free; one it
gets right 50% of the time (a coin-flip on the data) is one of the most
expensive things a hot loop can contain.

### Predictable vs unpredictable
The predictor is excellent at patterns: always-taken, never-taken,
alternating, loop back-edges. It is helpless against branches that follow
the *data* when the data is random. A bounds check that never fails, an
error path that almost never triggers — cheap, because the direction is
consistent. A branch on whether a random byte is `>` a threshold —
expensive, because there is no pattern to learn.

```rust
// Sorting the input first makes this branch predictable and the loop
// several times faster — same instructions, same data, different order.
for &b in &data {
    if b >= threshold { sum += b as u64; }  // random data => ~50% mispredict
}
```

### Where a proxy meets it
Byte-at-a-time parsing with a branch per character (`05-http-stack/parser.md`)
is the honest hot spot — a branch deciding "is this a delimiter" runs
millions of times. Per-request policy checks (WAF rules
`07-security/waf.md`, IP filtering `07-security/ip-filtering.md`) are
branches on request data. Usually these are *predictable* (almost all
traffic is allowed, almost all bytes are not delimiters), which is why they
are cheap in practice — the danger is a branch that genuinely splits
50/50 on hot data.

### Making branches disappear
When a branch is inherently unpredictable, the fastest fix is often to
remove it — compute both sides branchlessly:

- **Branchless select:** `sum += (b >= threshold) as u64 * b as u64;`
  turns the condition into arithmetic the CPU never mispredicts.
- **Table lookup:** replace a chain of `if`/`match` on a byte with an
  indexed 256-entry lookup table (a byte-classification table), the trick
  behind fast HTTP header scanners.
- **SIMD:** process 16–32 bytes at once with no per-byte branch at all —
  see `17-performance/simd.md`, which is where header scanning ultimately
  goes.

Gotcha: branchless code is not automatically faster. Removing a
*predictable* branch just adds arithmetic the branch predictor was already
hiding for free, and can be slower. Branchless wins specifically when the
branch was unpredictable. This is the whole folder's rule in miniature:
measure the misprediction rate first.

### The likely/unlikely nudge
For a branch you *know* is lopsided (an error path taken ~never), you can
hint the compiler to lay out the cold side out-of-line, keeping the hot
path's instructions dense in cache. In Rust this is `core::hint`'s
`likely`/`unlikely` (or `cold_path`) where stabilized, and `#[cold]` on the
rarely-called function. It helps code layout more than prediction itself,
and only on genuinely lopsided branches.

## Practice
1. Reproduce the sorted-vs-unsorted effect: sum elements above a threshold
   over random data, then over sorted data, and measure the difference
   from prediction alone. Read `perf stat branch-misses` for both.
2. Rewrite that loop branchlessly (`as u64` multiply) and compare — confirm
   it beats the *unsorted* case but check whether it beats the *sorted*
   (predictable) case.
3. Replace an `if`/`match` byte classifier in an HTTP-parser-style loop
   (`05-http-stack/parser.md`) with a 256-entry lookup table and measure.
4. Measure a WAF/IP-filter branch (`07-security/waf.md`) under realistic
   mostly-allowed traffic and confirm it is *predictable* and therefore
   cheap — practice recognizing a branch not worth touching.
5. Add `#[cold]` to a genuine error path in `proxy`, inspect whether the
   hot path's code got denser, and verify with a benchmark that you did not
   make things worse — then decide whether the next step is
   `17-performance/simd.md`.
