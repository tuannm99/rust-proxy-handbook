# HTTP/1.1 Wire Format

The exact byte grammar of an HTTP/1.1 message (RFC 9112, with character
classes from RFC 9110): what is legal, what must be rejected, and where
the spec leaves you a choice. [`01-network/10-http.md`](10-http.md) covers what a message
*means*; this file covers what it *looks like on the wire*, and it is the
spec [`labs/01-http-parser`](../../labs/01-http-parser) implements. You should not need to open the
RFC to finish that lab. Section numbers are given so you can check any
rule at the source.

## What to learn

### The shape of a message
Every HTTP/1.1 message is the same four parts (RFC 9112 §2.1):

```text
HTTP-message = start-line CRLF
               *( field-line CRLF )
               CRLF
               [ message-body ]
```

A start line, zero or more header lines, one empty line, then an optional
body whose length is decided by the headers (see "Message body length"
below), never by looking for a terminator. Here is a real request byte by
byte (`\r\n` is CRLF, the two bytes `0x0D 0x0A`):

```text
POST /upload?id=7 HTTP/1.1\r\n        <- request-line
Host: example.com\r\n                  <- field-line
Content-Length: 5\r\n                  <- field-line
\r\n                                   <- empty line: header section ends
hello                                  <- body: exactly 5 bytes, no CRLF after
```

The empty line is the only way to know the header section has ended. In
the byte buffer that is the sequence `\r\n\r\n`: the CRLF closing the last
field line immediately followed by the CRLF of the empty line. The body
has no terminator of its own. After its 5 bytes, the *next* byte on a
keep-alive connection is the first byte of the next request.

### Character classes: the alphabet of the grammar
The grammar is written in ABNF using a handful of byte classes. Every
validity check in your parser comes down to one of these:

| Name | Bytes | Used for |
|---|---|---|
| `CRLF` | `0x0D 0x0A` | line terminator |
| `SP` / `HTAB` | `0x20` / `0x09` | separators |
| `OWS` (optional whitespace) | zero or more `SP`/`HTAB` | around header values |
| `BWS` ("bad" whitespace) | same as `OWS` | allowed only for leniency, around `;` and `=` in chunk extensions |
| `DIGIT` | `0`-`9` | `Content-Length`, version |
| `HEXDIG` | `0`-`9`, `A`-`F`, `a`-`f` | chunk sizes |
| `VCHAR` | `0x21`-`0x7E` (visible ASCII) | header values |
| `obs-text` | `0x80`-`0xFF` | header values (legal, but not UTF-8 guaranteed) |
| `tchar` | ALPHA, DIGIT, and ``! # $ % & ' * + - . ^ _ ` \| ~`` | tokens |
| `token` | one or more `tchar` | methods, header names, coding names |

A `token` contains no whitespace, no `:`, no `"`, no `(),/;<=>?@[\]{}`,
and no control bytes. So "is this a valid header name?" is exactly "is it
one or more `tchar`?" (RFC 9110 §5.6.2). That check alone rejects
`Content-Length ` (trailing space), `Content Length`, and a name with a
NUL in it.

Gotcha: `tchar` is ASCII only. A parser that checks "not whitespace and
not `:`" instead of "is `tchar`" accepts bytes that other parsers reject,
and a disagreement between two parsers about what counts as a header is
the raw material of [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md).

### Request line
```text
request-line = method SP request-target SP HTTP-version
method       = token
HTTP-version = "HTTP" "/" DIGIT "." DIGIT
```

- **Method** is a `token` and is **case-sensitive** (RFC 9110 §9.1):
  `get` is not `GET`, it is an unknown method. A server that doesn't
  implement a method answers `501 Not Implemented`.
- **Separators** are exactly one `SP` each. RFC 9112 §3 lets a recipient
  *optionally* split on any run of whitespace (SP, HTAB, VT, FF, bare CR)
  instead, and in the same paragraph warns that this leniency causes
  smuggling. Being strict here costs you nothing with real clients.
- **Version** is literally `HTTP/` then one digit, `.`, one digit, and
  `HTTP` is case-sensitive. `HTTP/1.1` and `HTTP/1.0` are what you will
  see. An unsupported major version gets `505 HTTP Version Not Supported`.
- **Length**: the RFC recommends supporting request lines of at least
  8000 bytes. Longer than your limit → `414 URI Too Long`.
- Any other malformed request line → `400 Bad Request`.

### Request target: four forms
The request target is never whitespace-containing, and comes in four
forms (RFC 9112 §3.2). Which one is legal depends on the method:

| Form | Example | When |
|---|---|---|
| origin-form | `/where?q=now` | normal requests to an origin server. Must start with `/` |
| absolute-form | `http://example.com/where?q=now` | requests sent *to a proxy*. A server must accept it too |
| authority-form | `example.com:443` | `CONNECT` only |
| asterisk-form | `*` | `OPTIONS *` only (the whole server, not a resource) |

The target is bytes, not a decoded path. Percent-decoding (`%2F` → `/`)
and dot-segment removal (`/a/../b`) are a separate, later step with its
own security rules ([`07-security/04-normalization.md`](../07-security/04-normalization.md)). A parser should
hand the raw target to the next layer, not "clean it up".

### The `Host` header is part of framing
An HTTP/1.1 request must carry exactly one `Host` field (RFC 9112 §3.2).
A server must answer `400` to an HTTP/1.1 request that has no `Host`, has
more than one `Host` line, or has an invalid `Host` value. When the target
is in absolute-form, the authority inside the target wins and the `Host`
header is ignored. This matters to a proxy because `Host` picks the
virtual host ([`05-http-stack/12-vhost-routing.md`](../05-http-stack/12-vhost-routing.md)). Two `Host` values that two
layers resolve differently is a routing-confusion bug.

### Field lines (headers)
```text
field-line  = field-name ":" OWS field-value OWS
field-name  = token
field-value = *field-content      ; VCHAR, obs-text, and SP/HTAB between them
```

Rules, all from RFC 9112 §5 and RFC 9110 §5:

- **Nothing between the name and the colon.** `Host : x` must be rejected
  with `400` (§5.1). This is a MUST, not a choice. It exists because some
  parsers once treated `Transfer-Encoding : chunked` as a header and
  others didn't.
- **Whitespace around the value is not part of the value.** Strip leading
  and trailing `SP`/`HTAB`. Whitespace *inside* the value is kept.
- **An empty value is legal.** `X-Empty:` followed by CRLF is a valid
  field with value `""`.
- **CR, LF, or NUL inside a value** makes the message invalid. The
  recipient must reject it or replace each with `SP` (RFC 9110 §5.5).
  Rejecting is the safe choice: a stray CR is how response splitting and
  header injection get in.
- **A bare CR** anywhere outside the body (CR not followed by LF) is
  invalid, with the same reject-or-replace rule (§2.2).
- **Names are case-insensitive, and ASCII only.** `content-length`,
  `Content-Length` and `CONTENT-LENGTH` are the same field.
- **Values are bytes.** `obs-text` (`0x80`-`0xFF`) is legal, so a value is
  not guaranteed to be UTF-8. Don't make your header type a `String`.

### Duplicate fields
A sender may only repeat a field name if that field's value is a
comma-separated list (RFC 9110 §5.3), for example `Accept`, `Via`,
`Transfer-Encoding`, `Cache-Control`. For those, these two forms mean the
same thing, and order must be preserved:

```text
Cache-Control: no-cache\r\n            Cache-Control: no-cache, no-store\r\n
Cache-Control: no-store\r\n
```

Fields that are *not* lists must appear once, and the ones that decide
framing or routing are exactly the ones you must be strict about:
`Host` (reject duplicates, see above) and `Content-Length` (see below).
`Set-Cookie` is the famous exception: it repeats, but can't be joined
with commas, because cookie dates contain commas. That one only appears
in responses.

### Obsolete line folding and bare LF: your two decisions
Two old forms are still in the grammar for compatibility, and the lab asks
you to decide on each and write down why:

- **obs-fold** (§5.2): a header line that starts with SP or HTAB is a
  continuation of the previous header's value (`X-Long: a\r\n  b\r\n`).
  It was deprecated because parsers disagreed on it. A server receiving
  one in a request must either reject with `400` or replace the fold
  (CRLF plus the leading whitespace) with SP before interpreting the value.
- **Bare LF** (§2.2): a recipient *may* accept a lone `\n` as a line
  terminator. Two parsers where one accepts `\n` and the other only
  `\r\n` see different header boundaries in the same bytes.

A related MUST from §2.2: a line of whitespace between the request line
and the first header must be rejected or the whole whitespace-led line
ignored. And for robustness, a server *should* skip at least one empty
line (a stray CRLF left over from a previous request) *before* the
request line.

### Message body length: the ordered rules
This is the part that decides correctness and security. RFC 9112 §6.3
gives rules that are applied **in order**, first match wins. For a server
reading a request:

1. **Both `Transfer-Encoding` and `Content-Length` present.** TE overrides
   CL, and the message "ought to be handled as an error" because this is
   the classic smuggling setup. A server *may* reject it, or process it
   by TE alone, but either way *must* close the connection after
   responding (§6.1). A proxy that forwards it must drop the
   `Content-Length` first. Rejecting it is a stricter policy than the RFC
   requires, and a defensible one.
2. **`Transfer-Encoding` present.** If `chunked` is the **last** coding in
   the list, the body is chunked (see below). If it isn't, a request's
   length can't be determined: answer `400` and close.
3. **`Content-Length` present but invalid.** Invalid means: not
   `1*DIGIT`, or several values that disagree. Answer `400` and close.
   This is unrecoverable, because you don't know where the next request
   starts.
4. **Valid `Content-Length`.** The body is exactly that many bytes. If
   the connection closes first, the message is incomplete, which is an
   error, not a short body.
5. **Neither present.** A request has **no body** (length 0).

Details that the rules depend on:

- `Content-Length = 1*DIGIT`: decimal digits only. No sign (so no `-1`
  and no `+5`), no hex, no spaces inside, no empty value. Watch for
  overflow: `99999999999999999999999` is all digits and still invalid for
  you. The RFC tells you to expect huge numerals.
- Repeated identical values (`Content-Length: 5, 5`, or two lines both
  `5`) *may* be accepted as a single `5`, or rejected (RFC 9110 §8.6).
  Differing values are always invalid.
- `Transfer-Encoding` is a comma-separated list of coding names,
  case-insensitive (`Chunked` is `chunked`). Multiple TE lines join into
  one list. `chunked` must be last and must not appear twice. An unknown
  coding in a request → `501`.
- `Transfer-Encoding` in an **HTTP/1.0** message means the framing is
  faulty even if a `Content-Length` is also there. Process it and close
  (§6.1). TE did not exist in 1.0.

The response side (what [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) needs when it reads upstream
replies) adds three things that come *before* the rules above: a response
to `HEAD`, or with status `1xx`, `204` or `304`, never has a body whatever
its headers say. A `2xx` to `CONNECT` turns the connection into a tunnel.
And a response with neither TE nor CL runs until the connection closes.
That last rule is response-only, which is why "read until close" never
applies to a request.

### Chunked transfer coding
When the sender doesn't know the length up front, the body is a series of
chunks (RFC 9112 §7.1):

```text
chunked-body    = *chunk last-chunk trailer-section CRLF
chunk           = chunk-size [ chunk-ext ] CRLF chunk-data CRLF
chunk-size      = 1*HEXDIG
last-chunk      = 1*("0") [ chunk-ext ] CRLF
chunk-ext       = *( BWS ";" BWS ext-name [ BWS "=" BWS ext-val ] )
ext-name        = token
ext-val         = token / quoted-string
trailer-section = *( field-line CRLF )
```

The same body, `hello world`, byte by byte:

```text
5\r\n                  <- chunk-size (hex 5 = 5 bytes)
hello\r\n              <- 5 bytes of data, then a mandatory CRLF
6;note=x\r\n           <- size 6 with a chunk extension
 world\r\n             <- 6 bytes: " world", then CRLF
0\r\n                  <- last-chunk: size zero
Checksum: abc\r\n      <- trailer-section: zero or more field lines
\r\n                   <- empty line: the chunked body ends here
```

What each piece requires:

- **chunk-size** is hexadecimal, case-insensitive, and may have leading
  zeros (`000a` is 10). There is no `0x` prefix, no sign, and no
  whitespace before it. It can be arbitrarily long in the grammar, so cap
  the digit count before converting, or `ffffffffffffffffff` overflows.
- **The CRLF after chunk-data is mandatory**, and it must be exactly
  CRLF. A parser that counts out `size` bytes and then skips "whatever
  line ending is there" disagrees with a strict one about where the next
  chunk-size starts. That is a documented smuggling variant.
- **Chunk extensions** (`;name=value` after the size) carry no meaning you
  need: a recipient must ignore ones it doesn't recognize. You still have
  to *parse* them, so you can find the CRLF, and cap their total length,
  since they are otherwise an unbounded buffer an attacker controls
  (§7.1.1). `ext-val` can be a `quoted-string`: `"` ... `"`, where `\`
  escapes the next byte, and the content may include `;` and `=`.
- **The last chunk** is one or more `0` digits (optionally with
  extensions) and CRLF. It carries no data and no data-CRLF of its own.
- **The trailer section** is zero or more field lines with the same
  grammar and rules as headers, ended by an empty line. With no trailers,
  the body ends `0\r\n\r\n`. Trailers must not be merged into the header
  section unless a field's definition allows it (§7.1.2). Keep them
  separate: a `Content-Length` or `Host` arriving as a trailer must never
  change framing or routing.
- **Decoded length** is the sum of the chunk sizes. That sum is what your
  maximum body size applies to, not any one chunk.

Read it as a small state machine: read size line → read that many data
bytes → expect CRLF → repeat, until a zero size switches you to reading
trailer lines until an empty line. Each state can run out of input
mid-way, which is why [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md)'s incremental-parsing
section applies to the body too.

### What to answer when you reject
The status code is part of getting a rejection right:

| Situation | Status |
|---|---|
| malformed request line, header syntax, `Host` rule, CL/TE framing | `400 Bad Request` |
| request line/target longer than your limit | `414 URI Too Long` |
| header section too big or too many fields | `431 Request Header Fields Too Large` |
| body larger than your limit | `413 Content Too Large` |
| unknown method, or unknown transfer coding | `501 Not Implemented` |
| unsupported HTTP major version | `505 HTTP Version Not Supported` |

After any framing error (`400` for framing, and every CL/TE conflict),
close the connection. You no longer know where the next message starts,
so reusing the connection means parsing an attacker's bytes as a request.

### Response line (for later)
Responses start with `status-line = HTTP-version SP status-code SP
[ reason-phrase ]`, where status-code is exactly three digits and the
reason phrase (`OK`, `Not Found`) is free text a client should ignore.
Everything after the start line (fields, empty line, body length) uses
the rules above.

## Practice
1. Send `printf 'GET / HTTP/1.1\r\nHost: localhost\r\n\r\n' | nc localhost 80` (or against any local server) and view both the request and the reply with `| xxd`. Point at every CRLF and at the `\r\n\r\n` that ends each header section.
2. For each of these requests, write down from this file alone whether it is valid and, if not, which rule it breaks and what status it earns: `get / HTTP/1.1`, `GET  / HTTP/1.1` (two spaces), `GET / HTTP/1.1` with `Host : a`, no `Host` at all, two `Host` lines, `Content-Length: +5`, `Content-Length: 5, 6`, `Transfer-Encoding: gzip`, `Transfer-Encoding: chunked, gzip`.
3. Hand-decode `4\r\nWiki\r\n5;x="a;b"\r\npedia\r\n0\r\nX-T: 1\r\n\r\n`: list each chunk's size, extension, data, and the trailer. Then find the error in `4\r\nWikiXX\r\n0\r\n\r\n`.
4. Send the malformed requests from exercise 2 to two different real servers (for example nginx and a hyper-based one) with `printf ... | nc`, and record where their answers differ. Every difference is a place a proxy in front of one of them could be smuggled past.
5. In [`labs/01-http-parser`](../../labs/01-http-parser), turn exercises 2 and 3 into a test table: input bytes → expected parse result or expected status code. Write the table *before* the parser.
6. Write down your decisions for obs-fold and bare LF in [`labs/01-http-parser`](../../labs/01-http-parser)'s README, each with the rule from this file and the smuggling risk it avoids or accepts.
