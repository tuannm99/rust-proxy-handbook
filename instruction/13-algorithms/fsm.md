# FSM (Finite State Machines) as a Design Pattern

`13-algorithms/dfa.md` covers the accept/reject automaton theory. This
file covers finite state machines as something you deliberately reach for
when *writing* Rust — protocol parsers and connection lifecycles are
state machines whether or not you model them as one, and modeling them
explicitly is what prevents the invalid-state bugs that show up when you
don't.

## What to learn

### Beyond accept/reject: machines with actions
A DFA only answers "accept or reject." A **Mealy machine** (action on
each transition) or **Moore machine** (action on each state) attaches
behavior to the transitions themselves — which is what an HTTP/1.1
connection actually is: `Idle -> ReadingRequestLine -> ReadingHeaders ->
ReadingBody -> Idle` (keep-alive) or `-> Closed`, with real work
happening on each edge, not just a yes/no at the end. See
`05-http-stack/01-parser.md` and `05-http-stack/04-keepalive.md` for the
concrete states; this file is about how to encode the machine itself.

### Enum + match: the common case
```rust
enum ParseState {
    RequestLine,
    Headers { partial: Vec<u8> },
    Body { remaining: usize },
    Done,
}

fn advance(state: ParseState, byte: u8) -> ParseState {
    match state {
        ParseState::RequestLine => { /* ... */ ParseState::Headers { partial: vec![] } }
        // ...
        _ => state,
    }
}
```
Cheap to write, easy to read, and the compiler's exhaustiveness check on
`match` means adding a new state forces you to handle it everywhere —
real protection, for free.

### Typestate: pushing illegal transitions to compile time
The enum approach still lets *runtime* code call the wrong method on the
wrong state (nothing stops you from trying to read a body before headers
finish parsing except a `match` arm you have to remember to write). The
**typestate pattern** encodes each state as a distinct type, so illegal
calls are a compile error, not a runtime branch:

```rust
struct Idle;
struct HeadersRead { content_length: Option<usize> }

impl Idle {
    fn read_headers(self) -> HeadersRead { /* ... */ HeadersRead { content_length: None } }
}
impl HeadersRead {
    fn read_body(self) -> Body { /* only callable once headers exist */ Body }
}
```
`Idle` simply has no `read_body` method — calling it out of order doesn't
compile, rather than panicking or silently misbehaving in production. See
`03-rust/01-ownership.md` for why consuming `self` (not `&self`) is what
makes this pattern actually enforce one-way transitions.

### Choosing between them
Typestate's compile-time guarantee costs more boilerplate (a type per
state, conversions between them) and doesn't compose well when the state
must be stored in a struct field or passed across an `.await` boundary
inside a single future (the type would have to change mid-poll). Reach
for enum+match by default; reach for typestate specifically for a state
machine where an illegal transition is a bug class you've actually hit or
one that would be a security issue (e.g. sending a response before
validating the request is fully read).

## Practice
1. Model `labs/01-http-parser`'s request-parsing states as an explicit
   enum *before* writing the parsing logic; use `match` exhaustiveness to
   confirm every state has a defined transition for every byte class it
   can see.
2. Rewrite the keep-alive connection lifecycle from
   `05-http-stack/04-keepalive.md` (idle → reading → responding → idle/closed)
   as a typestate chain; try to call a method out of order and confirm it
   fails to compile.
3. Pick one state transition your enum-based parser handles with a
   `_ => unreachable!()` or silent fallthrough, and explain in writing why
   typestate would (or wouldn't) have caught a bug there at compile time
   instead of at runtime.
