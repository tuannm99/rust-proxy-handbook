# Lexer

Turning a flat stream of bytes into a stream of meaningful tokens — the
stage before any structure exists. `05-http-stack/01-parser.md` does this
inline for HTTP; this file is the reusable theory underneath, equally
applicable to the config parser in `09-architecture/03-config.md`.

## What to learn

### What a token is, and why the split helps
A lexer (scanner, tokenizer) collapses runs of raw bytes into a small set
of typed tokens: the bytes `Host:` become one `HeaderName("Host")` token,
`{` becomes `LBrace`, `3600` becomes `Number(3600)`. The parser then works
over tokens rather than characters, so it never has to think about
whitespace, digit-by-digit number assembly, or where one word ends.

```rust
enum Token {
    Ident(Range<usize>),   // a slice into the input, not an owned String
    Number(u64),
    LBrace,
    RBrace,
    Colon,
    Eof,
}
```

Gotcha: make tokens borrow the input (store byte ranges), not own copies.
A config file or HTTP message is parsed once and thrown away; copying every
identifier into a `String` doubles allocation for no benefit. This is the
same "bytes, not String" discipline as `05-http-stack/01-parser.md`.

### The core loop: one byte of lookahead
Most lexers are a single loop with a cursor and a `peek()` of the next
byte. You classify by the current byte, then consume a maximal run of the
same class. This is a hand-written DFA — each `match` arm is a state.

```rust
fn next_token(&mut self) -> Token {
    self.skip_whitespace();
    match self.peek() {
        None => Token::Eof,
        Some(b'{') => { self.bump(); Token::LBrace }
        Some(c) if c.is_ascii_digit() => self.lex_number(),
        Some(c) if is_ident_start(c) => self.lex_ident(),
        Some(c) => self.error_unexpected(c),
    }
}
```

Gotcha: decide up front whether whitespace and comments are *skipped* or
*emitted as tokens*. A config format usually skips them; a formatter or a
linter needs them preserved. Retrofitting this later means touching every
call site, so choose deliberately.

### Maximal munch and its ambiguities
The rule "consume the longest valid token" (maximal munch) is what makes
`3600ms` lex as `Number(3600)` then `Ident("ms")` rather than erroring at
the `m`. It also creates classic traps: `>=` must be one token, so seeing
`>` you must peek for `=` before committing. Getting this wrong turns `a>=b`
into `>` `=` and a syntax error.

### Position tracking is not optional
A lexer that reports "unexpected `}`" without a line and column is useless
on a real config file. Track byte offset always, and line/column if errors
are user-facing. The cheap version: carry the current offset in the token's
range and compute line/column lazily only when an error is actually
raised — do not pay per-byte for position bookkeeping on the happy path.

### Where lexing and parsing blur
Not every format has a clean lexer/parser split. HTTP/1.1 request lines are
so simple that `05-http-stack/01-parser.md` scans bytes directly with no token
type — introducing a lexer there would add ceremony without value. The
split earns its keep when the grammar has real nesting and precedence (a
config language, an expression language), where a token stream genuinely
simplifies the parser. Reach for a separate lexer when the parser would
otherwise be tangled with whitespace and character classification, not
before.

## Practice
1. Write a lexer for a tiny config grammar — `key value;` lines, `{}`
   blocks, `#` comments, numbers with optional unit suffixes (`10s`,
   `4k`) — emitting borrowed-range tokens. This is the front half of the
   config work in `labs/13-hot-reload`.
2. Handle maximal munch correctly for a two-character operator (`>=` or
   `//`): write the failing test first (`a>=b` must lex to three tokens),
   then the peek logic that passes it.
3. Add byte-offset tracking and turn one lexer error into a message with
   line and column, computed lazily from the offset rather than tracked per
   byte.
4. Benchmark borrowed-range tokens against a version that allocates a
   `String` per identifier on a large config; confirm the allocation count
   difference.
5. Compare your hand-written scanner against `05-http-stack/01-parser.md`'s
   inline HTTP scanning and articulate why HTTP does *not* use a separate
   token type — when the split helps and when it is ceremony.
