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
of ambiguity `05-http-stack/parser.md` and `labs/01-http-parser` force you
to confront by hand.

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

### Mitigations
1. **Reject ambiguity outright**: if a request has both `Content-Length`
   and `Transfer-Encoding`, reject it with 400 — don't try to guess which
   one "wins" (RFC 9112 says to reject this exact case).
2. **Normalize before forwarding**: strip/reject duplicate or malformed
   framing headers rather than passing them through unchanged.
3. **Prefer HTTP/2 to upstreams** where possible — HTTP/2's length-prefixed
   framing has no `Content-Length`-vs-`Transfer-Encoding` ambiguity to
   begin with, which is why "downgrade smuggling" (HTTP/2 front-end,
   HTTP/1.1 back-end) is its own attack class to watch for during
   protocol translation.
4. Prefer per-request upstream connections (or aggressive connection
   draining on any parse anomaly) over long-lived reused connections when
   you can't fully trust upstream parser consistency.

## Practice
1. In `labs/01-http-parser`, add a test case with both `Content-Length`
   and `Transfer-Encoding: chunked` present and confirm your parser rejects
   it with an error rather than picking one.
2. Add a test with a malformed `Transfer-Encoding` value (trailing
   whitespace, duplicate header) and confirm it's rejected, not silently
   normalized to "not chunked."
3. In `proxy`, add a request-framing validation
   stage before the upstream call that rejects ambiguous CL/TE
   combinations, and log a security event when it fires.
4. (Stretch) Read a public HTTP request smuggling writeup (e.g. from
   PortSwigger) and reproduce one CL.TE example end-to-end against your
   own proxy + a toy upstream to confirm your mitigation actually blocks it.
