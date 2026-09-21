# Regex Engines

How a regex engine is actually built, and why the answer decides whether
your WAF (`07-security/06-waf.md`) is a defense or a denial-of-service
vector.

## What to learn

### Two families, and the one that matters here
**Backtracking engines** (PCRE, Python `re`, JavaScript, Java) try one
alternative, and on failure rewind and try the next. This buys
backreferences and lookaround, and costs worst-case *exponential* time.

**Automata engines** (RE2, Rust's `regex`, Go's `regexp`) compile the
pattern to an NFA and simulate it, tracking the *set* of states currently
active rather than exploring one path at a time. Guaranteed O(n × m) —
linear in input length — at the price of dropping backreferences and
arbitrary lookaround, which are exactly the features that make linear-time
matching impossible.

For anything matching attacker-controlled input, this is not a preference.
It is the difference between a bounded cost and a remote crash.

### ReDoS: the failure mode
A backtracking engine on `(a+)+b` against `"aaaaaaaaaaaaaaaaaaaaaaaaX"`
explores every way to partition the `a`s between the inner and outer `+`.
That is exponential in input length: ~25 characters is already seconds,
~35 is minutes. One request, one CPU core pinned indefinitely.

This is a real and recurring outage class — Cloudflare's 2019 global
outage was a WAF rule with catastrophic backtracking, and Stack Overflow's
2016 outage was a single trim regex. The pattern is always the same: a
rule that looks harmless, deployed to inspect user input, meeting a string
its author never tried.

Gotcha: the `regex` crate protects you *by construction*, and it is easy to
lose that protection by reaching for a backtracking crate to get
backreferences (`fancy-regex`), or by shelling out to a PCRE-based engine.
If a WAF rule needs a backreference, treat that as a signal the rule is
wrong for this position in the stack, not as a reason to change engines.

### Thompson construction and the NFA simulation
Compilation goes regex → NFA via Thompson's construction: each operator
maps to a small NFA fragment (concatenation chains them, alternation forks
with epsilon transitions, `*` adds a loop), composed recursively.

Simulation then advances a *set* of active states one input byte at a time.
Since the state set can never exceed the total state count `m`, and each
byte is processed once, the bound is O(n × m) with no backtracking —
and the input is read strictly forward, which is what allows streaming.

### DFA construction and the memory trade
A DFA converts each distinct NFA state *set* into a single state, making
matching one table lookup per byte — the fastest option. The catch is that
the number of subsets is exponential in the worst case, so a naive full
DFA can blow up memory on a hostile pattern.

RE2 and the `regex` crate resolve this with a **lazy DFA**: build states on
demand as input is consumed, cache them, and evict the cache when it hits a
size cap, falling back to NFA simulation. You get DFA speed on realistic
patterns and bounded memory on pathological ones. That fallback is why
`regex` exposes `size_limit` and `dfa_size_limit` — set them explicitly
when the pattern set is loaded from config rather than written by you.

### Literal prefilters
The largest practical speedup is not the engine at all: extract required
literal substrings from the pattern, scan for those first with a fast
multi-pattern matcher (`13-algorithms/aho-corasick.md` or memchr), and
only run the full engine where a literal hit. A pattern like
`\d+-admin-\w+` cannot match without `-admin-` present, so most input is
rejected at memchr speed. The `regex` crate does this internally, and it is
the same design you should apply across a WAF's whole rule set.

### RegexSet: many patterns, one pass
Matching P patterns by looping P engines is O(P × n). `RegexSet` compiles
the alternation of all patterns into one automaton and reports which
matched in a single pass:

```rust
use regex::RegexSet;

// built once at config load, shared behind an Arc
let set = RegexSet::new(&[r"(?i)<script", r"(?i)union\s+select", r"\.\./"])?;
let hits: Vec<usize> = set.matches(input).into_iter().collect();
```

Gotcha: `RegexSet` tells you *which* patterns matched, not *where*. For
anomaly scoring (`07-security/06-waf.md`) that is sufficient and much faster.
Only re-run individual patterns for match positions when you actually need
to log the offending span.

### Compiling untrusted patterns
If WAF rules come from config that anyone but you can edit, the pattern is
untrusted input to the compiler. Enforce a compiled-size limit, a pattern
length limit, and reject on compile error at *load* time with the old rule
set left running (`09-architecture/03-config.md`) — never let a bad rule take
effect or take the proxy down at reload.

## Practice
1. Demonstrate ReDoS: match `(a+)+b` against `"a"*n + "X"` with a
   backtracking engine (`fancy-regex`) for n = 20, 25, 30 and plot the
   time. Repeat with the `regex` crate and confirm it stays linear.
2. Build an NFA by Thompson construction for `a(b|c)*d` on paper, then
   trace the active state set byte by byte over `"abccd"`.
3. In `labs/12-waf`, replace per-rule `Regex` loops with a single
   `RegexSet`; benchmark at 10, 100, and 1000 rules against a 100 KB body.
4. Add an Aho-Corasick literal prefilter in front of the `RegexSet` and
   measure the fraction of benign requests that never reach the engine.
5. Set `size_limit`/`dfa_size_limit` explicitly, then feed a pattern
   engineered to exceed them; confirm you get a clean load-time error
   rather than unbounded memory growth.
6. Load a deliberately invalid rule via config reload and confirm the
   previous rule set stays active.
