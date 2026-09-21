# Aho-Corasick

Multi-pattern string matching in one pass. The algorithm that makes a WAF
(`07-security/06-waf.md`) affordable: match 5000 signatures against a request
body in the time a naive loop matches one.

## What to learn

### The problem it solves
A WAF with P signatures, checked naively, runs P separate searches over
each request — O(P × n) for a body of length n, and it re-reads the same
bytes P times, destroying cache locality. Aho-Corasick finds *all*
occurrences of *all* patterns in O(n + matches), independent of P, in a
single pass that touches each input byte once.

The cost is a preprocessing step over the pattern set, done once at
startup or config reload — not per request.

### The automaton: trie plus failure links
Build a trie of all patterns. Then add a **failure link** from each node to
the node representing the longest proper suffix of the current match that
is also a prefix of some pattern. Matching then never backtracks over the
input: on a mismatch, follow the failure link and continue with the *same*
input byte.

The classic example: patterns `he`, `she`, `his`, `hers`. Matching
`"ushers"`, the automaton reaches `sh`→`she` and reports `she`; the
failure link from that node points into `he`, so it reports `he` at the
same position without rewinding — then continues into `hers`.

**Output links** handle the case where one pattern is a suffix of another
(`he` inside `she`): a node must report not only its own pattern but every
pattern reachable through its failure chain, or nested matches are silently
missed.

Gotcha: this is the bug people ship. If your test set has no pattern that
is a substring of another, a broken output-link implementation passes every
test — then misses real matches in production the day someone adds an
overlapping signature.

### Construction
Failure links are computed by BFS over the trie, level by level: a node's
failure target is derived from its parent's failure target, which BFS
guarantees is already resolved. Root children fail to the root.

For matching speed the trie is usually converted to a **DFA**: precompute a
full transition for every (state, byte) pair so matching is one table
lookup per byte with no failure-link chasing. The tradeoff is memory —
`states × 256` entries — which is why `aho-corasick` offers NFA and DFA
variants and picks based on pattern set size.

```rust
use aho_corasick::AhoCorasick;

// built once at startup / config reload, then shared read-only
let ac = AhoCorasick::new(&["' OR '1'='1", "<script", "../"])?;
for m in ac.find_iter(request_body) {
    // m.pattern() -> which signature; m.start() -> where
}
```

Gotcha: building the automaton is expensive relative to matching. Build it
once and share it behind an `Arc`; rebuilding per request (or worse, per
rule) discards the entire advantage. On config reload, build the new
automaton fully, then swap the `Arc` — never mutate a live one.

### Normalization must happen before matching
Aho-Corasick matches bytes literally. `<ScRiPt>` does not match `<script`,
and `%2e%2e%2f` does not match `../`. The normalization pipeline from
`07-security/06-waf.md` — URL-decode, lowercase, collapse whitespace — is what
makes literal matching viable, and it must be applied identically to the
patterns at build time and the input at match time.

Gotcha: normalize once into a buffer, then match. Normalizing lazily per
pattern reintroduces the O(P × n) cost you adopted this algorithm to avoid.
Watch for double-decoding too — decoding `%252e` twice yields `.`, and
whether an attacker can exploit that depends on what the *upstream* does,
which is the same class of parser-mismatch bug as
`07-security/05-request-smuggling.md`.

### Where the literal-match boundary is
Aho-Corasick handles literal strings, not regex. Real WAF rules need both:
use Aho-Corasick as a fast **prefilter** — if none of a rule's literal
substrings appear, the rule's regex cannot match, so skip it. This is
exactly how the `regex` crate accelerates alternations internally, and it
turns "run 5000 regexes" into "run the 3 regexes whose literals were
present." See `13-algorithms/regex-engine.md` for the engine side.

## Practice
1. In `labs/12-waf`, replace per-rule substring scanning with a single
   `AhoCorasick` built from all signature literals; benchmark both against
   a 100 KB body at 10, 100, and 1000 patterns and confirm the AC timing is
   flat in pattern count.
2. Write the overlapping-pattern test: include `he`, `she`, `his`, `hers`
   and assert `"ushers"` reports both `she` and `he`. This is the
   output-link check.
3. Build the trie and failure links by hand (no crate) for those four
   patterns and draw the automaton; verify your BFS order resolves each
   node's failure target before its children need it.
4. Wire normalization: confirm `<ScRiPt>` and `%3Cscript` both match the
   `<script` signature, and that patterns and input are normalized by the
   same code path.
5. Use AC as a prefilter in front of your regex rules; measure the
   percentage of rules skipped on benign traffic.
6. Reload the signature set under concurrent load by building a new
   automaton and swapping an `Arc` — confirm no request sees a partially
   built one.
