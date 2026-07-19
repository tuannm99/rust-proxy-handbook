# WAF Engine
Rule engine, signatures, anomaly scoring.

## What to learn
### Rule engine design
A WAF rule matches some part of the request (path, headers, query string,
body) against a pattern and takes an action (block, log, score). Model
rules as data, not code, so they can be updated without redeploying the
proxy:

```rust
struct Rule {
    id: u32,
    target: RuleTarget,       // Path, Header(String), Query, Body
    pattern: regex::Regex,
    action: RuleAction,       // Block, Log, Score(i32)
}
```
Gotcha: running every rule's regex against every request body is
expensive — order rules cheapest-first (path/header checks before body
regex) and short-circuit on the first `Block` match.

### Signature-based detection
Match known attack patterns directly (e.g. `' OR '1'='1` for SQLi,
`<script>` for XSS, `../../` for path traversal) — precise and fast, but
only catches attacks matching a known signature; trivially bypassed by
attackers who vary encoding/casing/whitespace, so signatures need
normalization (URL-decode, lowercase, collapse whitespace) applied
consistently before matching.

### Anomaly scoring
Instead of a hard block per rule, each matched rule adds a score; the
request is blocked only once the *total* score crosses a threshold. This
is ModSecurity's core model (the OWASP Core Rule Set) — it reduces false
positives from any single overly-broad rule, at the cost of being harder
to reason about ("why was this blocked?" requires summing which rules
fired).

### Build your own vs. use a real one
Writing a toy rule engine here is valuable for understanding how request
inspection fits into a proxy's pipeline (see `09-architecture/components.md`)
and its performance cost. For anything internet-facing, prefer embedding a
maintained engine — Coraza (Go, OWASP CRS-compatible, has a Rust FFI
story) or shelling out to ModSecurity — rather than trusting a hand-rolled
signature set to cover real-world attack traffic.

## Practice
1. In `proxy`, implement the `Rule`/`RuleTarget`
   types above with 5-10 hardcoded signatures (SQLi, XSS, path traversal).
2. Add anomaly scoring: give each rule a weight, sum matches, block only
   above a threshold; log the contributing rule IDs on block.
3. Add input normalization (URL-decode + lowercase) before matching and
   write a test proving a naive signature bypass (e.g. mixed-case
   `<ScRiPt>`) is now caught.
4. Benchmark request latency with the WAF stage on vs off under load to
   see the real cost of body-regex scanning.
