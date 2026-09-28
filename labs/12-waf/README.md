# 12-waf

## Goal

Rule-based request filtering: multi-pattern matching against normalized
headers and body for known-bad signatures, without a regex-per-rule scan.

## Done when

- [ ] A request containing any configured signature is blocked with `403`, and the matching rule ID is logged.
- [ ] Matching runs on the normalized form: percent-encoded, double-encoded, and mixed-case variants of a signature are all caught ([`instruction/07-security/04-normalization.md`](../../instruction/07-security/04-normalization.md)).
- [ ] A set of legitimate requests passes untouched (no false positives on your test corpus).
- [ ] Matching cost is roughly independent of rule count: a benchmark with 10 vs 10,000 patterns shows no linear slowdown ([`instruction/13-algorithms/aho-corasick.md`](../../instruction/13-algorithms/aho-corasick.md)).
- [ ] Body inspection is bounded by a maximum size and works on a streamed body, including a signature split across two chunks.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/07-security/06-waf.md`](../../instruction/07-security/06-waf.md)
- [`instruction/07-security/04-normalization.md`](../../instruction/07-security/04-normalization.md) — the parser-differential problem every bypass exploits
- [`instruction/13-algorithms/aho-corasick.md`](../../instruction/13-algorithms/aho-corasick.md)

## Run

```
cargo run -p waf
```
