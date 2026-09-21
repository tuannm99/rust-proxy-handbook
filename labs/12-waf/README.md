# 12-waf

## Goal

Rule-based request filtering: multi-pattern string matching against
headers/body for known-bad signatures, without a full regex-per-rule scan.

## Handbook references
- `instruction/07-security/06-waf.md`
- `instruction/07-security/04-normalization.md` — the parser-differential problem every bypass exploits
- `instruction/13-algorithms/aho-corasick.md`

## Run

```
cargo run -p waf
```
