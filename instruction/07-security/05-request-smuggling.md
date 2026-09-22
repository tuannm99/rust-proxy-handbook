# Request Smuggling

## What to learn
### Why it exists: two parsers, one disagreement
A reverse proxy parses an HTTP/1.1 request to decide where it ends, then
forwards bytes to an upstream, which parses the *same* bytes again with
its own parser. Request smuggling happens when the proxy and the upstream
disagree about where one request ends and the next begins — an attacker
crafts a request that one parser reads as "one request" and the other
reads as "one request plus the start of a second, smuggled request" that
gets processed against the next unlucky client's connection (on a reused
keep-alive/pooled connection to the upstream). This is precisely the class
of ambiguity `05-http-stack/01-parser.md` and `labs/01-http-parser` force you
to confront by hand.

The precondition worth noticing: this attack exists *because* the proxy
pools and reuses upstream connections (`06-proxy/01-upstream.md`). The
smuggled bytes sit at the front of a connection's buffer waiting for
whoever uses it next. A proxy that opened a fresh connection per request
and closed it after would be immune — and far slower, which is why nobody
does that, and why this attack class persists.

### What an attacker actually gets
Worth stating plainly, because the mitigations look like pedantry until
you see the payoff:
- **Bypassing front-end security entirely.** The proxy enforces auth
  (`01-auth.md`), IP filtering, and WAF rules on requests it can *see*. A
  smuggled request is never seen by the proxy as a request — it's body
  bytes — so it arrives at the upstream having skipped every check. An
  attacker reaches `/admin` through a proxy explicitly configured to block
  `/admin`.
- **Capturing other users' requests.** The smuggled prefix can be crafted
  so that the *next* real request on that connection gets appended into it
  as body content — and echoed back in a response the attacker can read.
  Session cookies and auth headers included.
- **Cache poisoning.** Combined with a cache (`05-http-stack/07-cache.md`),
  a desynced response gets stored against the wrong key and served to
  everyone.

Single attacker, no credentials, and the damage scales with how much
traffic shares the poisoned connection.

### CL.TE, TE.CL, TE.TE
- **CL.TE**: request has both `Content-Length` and `Transfer-Encoding:
  chunked`. The front-end (proxy) uses `Content-Length` to frame the body;
  the back-end (upstream) uses `Transfer-Encoding` instead. Bytes the
  proxy thought were "past the request" are actually inside the chunked
  body as far as the upstream is concerned — or vice versa — and get
  reinterpreted as the start of a new request.
- **TE.CL**: the reverse — front-end honors `Transfer-Encoding`, back-end
  honors `Content-Length`.
- **TE.TE**: both honor `Transfer-Encoding`, but one of them can be tricked
  into ignoring it via a malformed/obfuscated header value (e.g.
  `Transfer-Encoding: chunked ` with a trailing space, or a duplicate
  header) that only one of the two parsers treats as invalid and falls
  back to `Content-Length` for.

The bytes make it concrete. A CL.TE payload:

```http
POST / HTTP/1.1
Host: example.com
Content-Length: 6
Transfer-Encoding: chunked

0

G
```

The proxy reads `Content-Length: 6` and forwards exactly six bytes of body
(`0\r\n\r\nG`), considering the request complete. The upstream reads
`Transfer-Encoding: chunked`, sees the `0`-length chunk terminator, and
considers the body finished *before* the `G` — which is left in its
buffer. The next real request on that pooled connection gets `G` glued to
its front, becoming `GPOST / HTTP/1.1...` — and that victim gets an error,
while a more carefully crafted prefix gets the attacker something useful.

### Downgrade smuggling (H2.CL / H2.TE)
The modern variant, and the one most relevant to a proxy that terminates
HTTP/2 and speaks HTTP/1.1 upstream (`01-network/11-http2.md`). HTTP/2 frames
carry their own explicit lengths, so there is no ambiguity *in* HTTP/2 —
but `content-length` still exists as an ordinary header, and an attacker
can send an HTTP/2 request whose declared `content-length` disagrees with
the actual frame length.

If the proxy translates that request into HTTP/1.1 by copying headers
verbatim, it emits an HTTP/1.1 request with a `Content-Length` that
doesn't match the body it writes — manufacturing exactly the desync the
HTTP/1.1 parsers were careful to avoid. The same happens when an attacker
smuggles `transfer-encoding: chunked` through as an HTTP/2 header.

The rule when downgrading: **regenerate framing headers from the actual
data you are about to write, never copy them from the inbound request.**
And reject inbound HTTP/2 requests whose `content-length` disagrees with
the summed DATA frame lengths, rather than trusting either.

### CL.0 and connection-state desync
A quieter family: the upstream ignores the body entirely for some requests
(many servers discard a body on `GET`, or on a path that maps to a static
file), effectively treating `Content-Length` as `0`. The body the proxy
faithfully forwarded is then sitting in the upstream's buffer, and it gets
parsed as the next request — no header trickery needed at all, just an
endpoint that doesn't read what it was sent.

Gotcha: this one can't be fixed by validating headers, because the headers
are *valid*. It depends entirely on upstream behavior, which is why the
"drain or close the connection on any anomaly" mitigation below matters
even when your framing validation is perfect.

### Mitigations
1. **Reject ambiguity outright**: if a request has both `Content-Length`
   and `Transfer-Encoding`, reject it with 400 — don't try to guess which
   one "wins" (RFC 9112 says to reject this exact case).
2. **Normalize before forwarding**: strip/reject duplicate or malformed
   framing headers rather than passing them through unchanged. Reject
   rather than "clean up": a header you normalize into validity is one
   whose original form the upstream might still have accepted differently.
3. **Regenerate framing on every hop.** Whatever you forward upstream
   should have `Content-Length`/`Transfer-Encoding` written by *you*,
   derived from the body you're actually sending — not inherited.
4. **Prefer HTTP/2 to upstreams** where possible — HTTP/2's length-prefixed
   framing has no `Content-Length`-vs-`Transfer-Encoding` ambiguity to
   begin with, which is why "downgrade smuggling" (HTTP/2 front-end,
   HTTP/1.1 back-end) is its own attack class to watch for during
   protocol translation.
5. Prefer per-request upstream connections (or aggressive connection
   draining on any parse anomaly) over long-lived reused connections when
   you can't fully trust upstream parser consistency.
6. **Be strict about whitespace and line endings.** Accept only `\r\n` as
   a line terminator, reject a bare `\n`, reject whitespace between a
   header name and its colon, reject non-digit characters in
   `Content-Length`. Every one of these has been a real bypass, because
   "be liberal in what you accept" and "two parsers must agree" are
   directly contradictory goals — and for a proxy, agreement wins.

### Detecting it
You cannot rely on noticing the damage, since the victim is a different
client than the attacker. Signals worth wiring up
(`08-observability/01-logging.md`):
- **Upstream parse errors on pooled connections.** A 400 from the upstream
  on a request your proxy considered well-formed is the smoking gun for a
  desynced connection.
- **Requests with impossible prefixes.** Upstream logs showing methods
  like `GPOST` or paths glued to previous bodies.
- **Timing.** The standard detection technique (PortSwigger's) is to send
  a payload that makes the *upstream* wait for a body that never arrives:
  if the response hangs for the read timeout instead of returning
  promptly, the parsers disagreed. Worth building into your own test
  suite rather than only reading about.

When you detect an anomaly, **close the upstream connection** rather than
returning it to the pool — whatever is left in its buffer is the payload.

## Practice
Build these in order.

1. In `labs/01-http-parser`, add a test with both `Content-Length` and
   `Transfer-Encoding: chunked`. **Done when** the parser returns an error
   rather than picking one.
2. Add tests for the obfuscation variants: trailing whitespace after
   `chunked`, duplicate `Transfer-Encoding` headers, `Content-Length: 6 `
   with a trailing space, a bare `\n` line ending, and whitespace before
   the colon. **Done when** every one is rejected, and you can articulate
   for each what an upstream might otherwise have done with it.
3. Build the CL.TE payload above and send it through `proxy` to a toy
   upstream that uses a *different* parser (a Python or Node one-liner is
   ideal — different parser, different bugs). **Done when** you observe an
   actual desync: the toy upstream sees a mangled second request. You need
   to have seen it work before trusting that your fix stops it.
4. Add a framing-validation stage in `proxy` before the upstream call, and
   regenerate framing headers on forward. **Done when** step 3's payload
   is rejected with 400, a security event is logged, and the request your
   proxy emits upstream carries framing headers it computed itself.
5. Add the timing-based detector as a test. **Done when** a payload that
   would leave an upstream waiting for a phantom body is caught by your
   validation instead of hanging until the read timeout.
6. Make any parse anomaly poison the connection. **Done when** a
   connection that produced a framing error is closed rather than returned
   to the pool — verify with `ss -tan` that it does not reappear as an
   idle pooled connection.
7. (Stretch) If `proxy` terminates HTTP/2 (`labs/08-http2`), construct a
   downgrade payload where the HTTP/2 `content-length` disagrees with the
   DATA frames. **Done when** it is rejected at the h2 layer rather than
   translated into a malformed HTTP/1.1 request.
