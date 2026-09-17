# Input Normalization & Parser Differentials

Two components parse the same bytes differently, and the attacker lives in
the gap. This is the structural weakness behind WAF bypasses
(`07-security/waf.md`), path-based access-control bypasses
(`05-http-stack/router.md`), and — in its purest form — request smuggling
(`07-security/request-smuggling.md`). Normalization is the defense, and
it is a security boundary rather than a preprocessing detail.

## What to learn
### The shape of the problem
Any time your proxy interprets input to make a decision, and then forwards
the *original* bytes for something else to interpret again, you have two
parsers that must agree. They won't, unless you make them:

```
client bytes ──► proxy parses ──► decision (allow/block/route)
             └─► forwarded ────► upstream parses ──► different interpretation
```

The attacker's goal is any input where those two interpretations differ.
Your goal is to eliminate the second interpretation: normalize once,
decide on the normalized form, and **forward the normalized form** so
there is nothing left to disagree about.

Gotcha: this is why "forward the request exactly as received" — which
sounds conservative and respectful — is the dangerous choice for a
security-enforcing proxy. Faithful byte-forwarding preserves the
ambiguity you just resolved.

### Decode depth
You URL-decode once; the attacker sends `%252e%252e%252f`, which decodes
once to `%2e%2e%2f` (no match) and twice to `../`. If the upstream
framework decodes twice, it sees traversal and you didn't.

Decoding repeatedly until stable has the opposite failure: you now flag
input the upstream treats literally, which is a false positive rather than
a breach — usually the better error to make. Either way, the actual
requirement is to know how many times your upstream decodes and match it,
then document the choice where the rule set lives.

Gotcha: decode depth must be consistent across *every* component that
inspects the same input. If the router decodes once and the WAF decodes
twice, they disagree with each other, which is the same bug with the
attacker one hop closer.

### Case and Unicode
Lowercasing handles `<ScRiPt>`. It does not handle:
- **Full-width and homoglyph characters** that a framework may normalize
  into ASCII equivalents after you've inspected them.
- **Overlong UTF-8 encodings**, where a character is encoded in more bytes
  than necessary — historically a reliable way to smuggle `/` or `.` past
  byte-comparison checks.
- **Unicode case folding** differences: the Turkish dotless ı, the German
  ß, and characters whose uppercase form is multiple characters.

Apply Unicode normalization (NFKC) before matching if any upstream in your
fleet does, and reject input that isn't valid UTF-8 rather than
lossy-converting it — `String::from_utf8_lossy` replaces invalid sequences
with U+FFFD, which changes the bytes and can make a hostile payload look
benign.

### Charset and content-type
A body declared `charset=utf-16` that your WAF scans as UTF-8 is, to your
rules, meaningless noise — and to a framework honoring the declared
charset, a clean payload. The same applies to a JSON body sent as
`text/plain` that the upstream parses as JSON anyway.

Gotcha: decide whether you trust the declared content-type or sniff it,
and know which one the upstream does. If they differ, that difference is
the bypass.

### Parameter semantics differ between frameworks
`?id=1&id=2' OR '1'='1` is one request with two `id` values, and what
"the value of `id`" means depends entirely on who's asking:

| Framework family | Takes |
| --- | --- |
| PHP, Rails | last |
| ASP.NET | all, comma-joined |
| Many Go/Rust routers | first |
| Some parsers | array of both |

A WAF that inspects only the first value misses a payload the upstream
will execute; one that inspects only the last misses the reverse.

The rule: inspect **every occurrence of every parameter**, and inspect the
raw query string as well. The same applies to duplicate headers, and to
JSON bodies with duplicate keys (`15-parser/` — parser behavior there
varies too).

Gotcha: the framing headers are the extreme case of this, where the
disagreement gets you a whole smuggled request rather than one bad
parameter value. `07-security/request-smuggling.md` covers it; the
reasoning is identical, one layer down.

### Paths are their own normalization problem
Dot segments, encoded separators, duplicate slashes, trailing slashes, and
case-insensitive filesystems all make the path you routed on differ from
the path the upstream resolves. `05-http-stack/router.md` covers the
variants and the canonical-form rule; it is the same discipline applied to
the one input a proxy always parses.

### Normalize once, early, in one place
The failure mode that survives all of the above is normalizing in several
places with slightly different rules. Put normalization at a single point
in the pipeline (`09-architecture/components.md`, before routing), store
the canonical form on the request, and have every later component —
router, WAF, logger, upstream forwarder — read *that* rather than
re-deriving its own.

Gotcha: keep the original bytes available for logging
(`08-observability/logging.md`) so an investigation can see what was
actually sent, but make the canonical form the only thing any *decision*
reads. Two representations is fine; two decision inputs is not.

## Practice
Build these in order.

1. In `labs/12-waf`, write the bypass tests before any normalization:
   mixed case, double URL-encoding, an overlong UTF-8 encoding of `/`, a
   payload in the *second* of two same-named parameters, and a body whose
   declared charset differs from its actual encoding. **Done when** every
   one of them gets through — this is the baseline.
2. Add a single normalization stage: URL-decode to a documented depth,
   lowercase, NFKC, reject invalid UTF-8. **Done when** the first three
   tests are caught and you can state your decode depth and why.
3. Inspect all occurrences of every parameter plus the raw query string.
   **Done when** the duplicate-parameter test is caught regardless of
   which position the payload is in.
4. Determine empirically what your upstream does. **Done when** you have
   tested — not assumed — how many times it decodes and which duplicate
   parameter it takes, and your normalization matches.
5. Make normalization single-source. **Done when** the router, the WAF,
   and the upstream forwarder all read one canonical form, and a test
   proves the forwarded request carries the normalized path rather than
   the original bytes.
6. Keep the original for logs only. **Done when** a blocked request's log
   line shows the raw input as sent, while no decision path can read it.
