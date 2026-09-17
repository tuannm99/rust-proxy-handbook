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

Gotcha: rules loaded from config are attacker-adjacent input to your regex
*compiler*, not just your matcher. Enforce a compiled-size limit and
reject a bad rule set at load time with the previous set left running
(`09-architecture/config.md`) — a WAF that takes the proxy down on a typo
in a rule file is a self-inflicted outage. See
`13-algorithms/regex-engine.md` for the ReDoS side of this, which is the
reason Rust's `regex` crate (linear-time, no backtracking) is the right
engine here and `fancy-regex` is not.

### Signature-based detection
Match known attack patterns directly (e.g. `' OR '1'='1` for SQLi,
`<script>` for XSS, `../../` for path traversal) — precise and fast, but
only catches attacks matching a known signature; trivially bypassed by
attackers who vary encoding/casing/whitespace, so signatures need
normalization (URL-decode, lowercase, collapse whitespace) applied
consistently before matching.

### Normalization is where WAFs actually fail
Signatures match bytes, so whoever decides *which* bytes get matched
decides whether the WAF works. Getting that subtly different from the
upstream's own parsing — decode depth, Unicode, charset, which of two
same-named parameters counts — is how essentially every real WAF bypass
works, and it is a permanent structural gap rather than a bug you fix
once.

This is a large enough topic to live on its own, and it is shared with
routing (`05-http-stack/router.md`) and framing
(`07-security/request-smuggling.md`): see
`07-security/normalization.md`. Nothing else in this file works if that
doesn't.

### Body inspection: the cost and the hard limit
Inspecting a body means having the whole body, which means buffering it —
so WAF inspection and streaming are mutually exclusive, and the buffer is
bounded by memory you're willing to spend per concurrent request. This is
the same constraint as retry buffering (`06-proxy/retry.md`), and the two
should share one limit rather than each holding their own copy.

The unavoidable decision is what happens to a body larger than the limit:
- **Fail open** (forward uninspected): an attacker appends padding to
  exceed your limit and bypasses the WAF entirely, every time.
- **Fail closed** (reject): a legitimate large upload gets a 413.

Neither is "correct" — but fail-open is a *silent* bypass and fail-closed
is a visible error, so default to closed and carve out explicit,
authenticated routes for large uploads. If you fail open, alert on it; a
sudden rise in oversized bodies is an attack signature by itself.

### Anomaly scoring
Instead of a hard block per rule, each matched rule adds a score; the
request is blocked only once the *total* score crosses a threshold. This
is ModSecurity's core model (the OWASP Core Rule Set) — it reduces false
positives from any single overly-broad rule, at the cost of being harder
to reason about ("why was this blocked?" requires summing which rules
fired).

CRS layers a second knob on top: **paranoia levels**, where higher levels
enable progressively more aggressive (and more false-positive-prone)
rules. The operational pattern is to raise the paranoia level and lower
the threshold gradually, measuring false positives at each step, rather
than deploying the strictest configuration and discovering which
legitimate traffic it breaks in production.

Gotcha: always log the contributing rule IDs and scores on a block. A
block decision you cannot explain after the fact is one you cannot tune,
and "the WAF blocked a customer and nobody can say why" is how WAFs get
switched off permanently.

### False positives are the real risk
The threat model people bring to a WAF is "attacker gets through." The
outage they actually cause is "WAF blocks legitimate traffic" — a
too-broad rule that matches a customer's ordinary data (a name with an
apostrophe, a code snippet in a support ticket, a base64 blob that happens
to contain `../`) silently breaks a feature, and because the block happens
at the edge, application logs show nothing at all.

The standard mitigation is to run every new rule set in **detection-only
mode** first: score and log, never block. Compare what *would* have been
blocked against real traffic for days, tune, then enforce. This is a
canary process (`09-architecture/canary-deploy.md`) applied to security
rules, and skipping it is how a routine rule update becomes an incident.

Gotcha: detection-only mode must exercise the same code path as blocking
mode, differing only in the final action. A separate "dry run" path that
skips normalization or short-circuits differently tells you nothing about
what enforcement will actually do.

### Performance: prefilter before you match
Running a rule set of hundreds of regexes against every request is
linear in rule count, and it lands on the hot path of every request you
serve. Two structural fixes, both covered in `13-algorithms/`:
- Extract required literal substrings and scan for those first with a
  multi-pattern matcher (`13-algorithms/aho-corasick.md`). Most benign
  traffic contains none of them and never reaches a regex.
- Compile the surviving patterns into a single `RegexSet`
  (`13-algorithms/regex-engine.md`) so P patterns cost one pass, not P.

Gotcha: measure the WAF stage separately from total request latency
(`08-observability/metrics.md`). Folded into an overall number, a WAF that
adds 8ms at p50 and 200ms at p99 on large bodies looks like general
slowness rather than one tunable stage.

### Build your own vs. use a real one
Writing a toy rule engine here is valuable for understanding how request
inspection fits into a proxy's pipeline (see `09-architecture/components.md`)
and its performance cost. For anything internet-facing, prefer embedding a
maintained engine — Coraza (Go, OWASP CRS-compatible, has a Rust FFI
story) or shelling out to ModSecurity — rather than trusting a hand-rolled
signature set to cover real-world attack traffic.

## Practice
Build these in order.

1. In `labs/12-waf`, implement `Rule`/`RuleTarget` with 5-10 hardcoded
   signatures (SQLi, XSS, path traversal). **Done when** an obvious
   payload in the query string is blocked and ordinary traffic passes.
2. Work through `07-security/normalization.md`'s exercises against this
   rule engine. **Done when** the mixed-case, double-encoded, and
   duplicate-parameter bypasses are all caught, and the router and WAF
   read one shared canonical form.
3. Add anomaly scoring with per-rule weights and a threshold; log
   contributing rule IDs on block. **Done when** a single medium-weight
   match passes and two together block, with both rule IDs in the log
   line.
4. Add detection-only mode as a flag on the same code path. **Done when**
   a would-block request is logged with its score and forwarded unchanged,
   and flipping one config value enforces it.
5. Add body inspection with a shared buffer limit and fail-closed
   behavior. **Done when** a payload inside the limit is caught, an
   oversized body gets 413 rather than passing uninspected, and the
   oversized case increments its own metric.
6. Add an Aho-Corasick literal prefilter in front of a `RegexSet`. **Done
   when** you can report the fraction of benign requests that never reach
   a regex, and a benchmark at 10 / 100 / 1000 rules shows latency roughly
   flat instead of linear in rule count.
7. Benchmark the WAF stage on vs off under load, as its own metric. **Done
   when** you have p50 and p99 numbers for the stage alone, on both small
   and large bodies, and can point at which rules dominate.
