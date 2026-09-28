# Reading guide: hyper

A route through hyper's HTTP/1 implementation, with questions to answer as
you go. This is a guide, not the notes: the per-project files listed in
[`00-README.md`](00-README.md) are yours to write. Answers aren't given here — comparing
hyper's choices with the ones you made in [`labs/01-http-parser`](../../../labs/01-http-parser) is the
exercise.

Repository: `github.com/hyperium/hyper` (1.x), directory `src/`. hyper
delegates raw header tokenizing to the separate `httparse` crate
(`github.com/seanmonstar/httparse`), which you'll visit too. Paths match
the versions current at the time of writing; if one has moved, search for
the named type.

## When to read which part

| Read | After | Why then |
| --- | --- | --- |
| Stop 1-3: parsing and framing | [`labs/01-http-parser`](../../../labs/01-http-parser), [`05-http-stack/01-parser.md`](../../05-http-stack/01-parser.md) Practice step 8 | Every difference from your parser is a case you missed or chose differently |
| Stop 4: the connection state machine | [`labs/02-http-server`](../../../labs/02-http-server), [`05-http-stack/04-keepalive.md`](../../05-http-stack/04-keepalive.md) | You've debugged keep-alive yourself |
| Stop 5: the dispatcher | [`03-rust/18-async-traits.md`](../../03-rust/18-async-traits.md), [`labs/02-http-server`](../../../labs/02-http-server) | You've implemented a `Service` |

## The route

### Stop 1: tokenizing headers in `httparse`
Read `httparse`'s `Request::parse` and the header parsing it calls.
- How does it signal "I need more bytes" versus "this is malformed," and how does that compare to your parser's return type?
- Where does it use SIMD, and for which part of the request?
- It takes a caller-provided array of headers rather than allocating. What does that force the caller to decide up front, and what happens if a request has more headers than the array holds?

### Stop 2: turning tokens into a request: `proto/h1/role.rs`
The server side's parse function builds a request head from `httparse`'s
output.
- Find exactly where `Content-Length` and `Transfer-Encoding` are both present. What does hyper do, and does it match [`07-security/05-request-smuggling.md`](../../07-security/05-request-smuggling.md)?
- What does it do with multiple `Content-Length` headers that agree? That disagree?
- Where are header size limits enforced, and what's the error a client sees?

### Stop 3: body framing: `proto/h1/decode.rs` and `encode.rs`
The body decoder has a small set of kinds (fixed length, chunked, read
until EOF).
- Walk the chunked decoder's state machine. Which states exist, and which malformed inputs does each state reject?
- How are chunk extensions and trailers handled — accepted, ignored, or rejected?
- When is "read until EOF" framing chosen, and why is it only valid in some cases?

### Stop 4: the connection: `proto/h1/conn.rs` and `proto/h1/io.rs`
`conn.rs` tracks reading and writing state and keep-alive; `io.rs` owns
the buffering.
- What decides whether a connection can be reused after a response? List every condition you find.
- How does the read buffer grow, what is its maximum, and what happens when a request exceeds it?
- What happens if the client pipelines a second request before the first response is written?

### Stop 5: the dispatcher: `proto/h1/dispatch.rs`
This is where your `Service` gets called.
- How does the dispatcher interleave reading the next request, polling your service's future, and writing the response?
- Where is backpressure applied when the client reads the response body slowly ([`01-network/11-http2.md`](../../01-network/11-http2.md) covers the HTTP/2 version of this problem)?

## What to write in your notes
`interesting-code.md` should list, for each stop, one decision hyper made
differently from your [`labs/01-http-parser`](../../../labs/01-http-parser) and which one you now think is
right. `what-to-learn.md` should map what you found back to
[`05-http-stack/01-parser.md`](../../05-http-stack/01-parser.md), [`05-http-stack/04-keepalive.md`](../../05-http-stack/04-keepalive.md), and
[`07-security/05-request-smuggling.md`](../../07-security/05-request-smuggling.md).
