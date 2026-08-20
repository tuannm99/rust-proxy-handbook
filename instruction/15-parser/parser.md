# Parser

Turning a flat token stream into structured data. This is the general
theory; `05-http-stack/parser.md` is its HTTP-specific application and
`labs/01-http-parser` is where you implement that.

## What to learn

### Recursive descent: the default you should reach for
Recursive descent writes one function per grammar rule, and nesting in the
grammar becomes nesting in the call stack. It is the technique behind most
production parsers (including Rust's own `rustc`) because it is readable,
debuggable with an ordinary stack trace, and gives precise control over
error messages.

```rust
// grammar:  block := '{' entry* '}'   entry := ident value ';'
fn parse_block(&mut self) -> Result<Block, ParseError> {
    self.expect(Token::LBrace)?;
    let mut entries = Vec::new();
    while !self.check(Token::RBrace) {
        entries.push(self.parse_entry()?);
    }
    self.expect(Token::RBrace)?;
    Ok(Block { entries })
}
```

Gotcha: recursive descent recurses on nesting depth, so deeply nested input
(`{{{{...}}}}`, or a JSON array a million deep) can overflow the stack —
which in Rust aborts the process, it cannot be caught. Any parser exposed
to untrusted input **must** bound nesting depth explicitly with a counter,
exactly as `05-http-stack/parser.md` bounds header count and body size.
This is a real DoS vector, not a theoretical one.

### Predictive parsing and the one-token lookahead
A grammar is easy to parse top-down when the current rule can be chosen by
looking at one token (LL(1)). Your `match self.peek()` picks the branch;
no backtracking needed. When one token is not enough to decide, you either
peek further, restructure the grammar, or accept backtracking — and
backtracking on untrusted input reintroduces the same exponential-blowup
risk as a backtracking regex (`13-algorithms/regex-engine.md`). Prefer a
grammar you can parse with fixed lookahead.

### Precedence: where naive recursive descent gets ugly
Expression grammars with precedence (`a + b * c`) written as pure recursive
descent need one function per precedence level, which is verbose. Pratt
parsing (precedence-climbing) collapses that into one loop driven by a
binding-power table — the standard trick when you need operator precedence.
A proxy rarely needs full expressions, but a WAF rule language or a config
with `a && b || c` conditions does.

### Parser combinators: the other idiom
Libraries like `nom` and `winnow` build a parser by composing small
functions (`tag`, `take_while`, `alt`, `many0`) rather than writing an
explicit state machine. They are excellent for binary and network formats —
`nom` is widely used for exactly the kind of byte-level protocol parsing a
proxy does. The trade-off: error messages are harder to make precise, and
the combinator types can get baroque. Reach for combinators for binary
framing, hand-written recursive descent for anything where error quality
matters to a human editing the input.

### Errors: recover, do not just bail
A parser that dies on the first error forces the user to fix a config one
line per run. Real parsers recover: on an error, skip tokens until a known
synchronization point (the next `;` or `}`), record the error, and keep
going so one pass reports several problems. For a hot-reloaded config
(`09-architecture/config.md`) this is the difference between a usable and
an infuriating tool — though note that for config you still reject the
*whole* reload atomically; you recover only to collect all the errors to
show at once.

## Practice
1. Extend the lexer from `15-parser/lexer.md` into a recursive-descent
   parser producing a typed config struct, one function per grammar rule.
   This is the parse stage of `labs/13-hot-reload`.
2. Add an explicit nesting-depth counter and a test that feeds deeply
   nested blocks, proving the parser returns a clean error instead of
   overflowing the stack.
3. Implement error recovery: on a bad entry, synchronize to the next `;`
   and continue, so a config with three mistakes reports all three in one
   run.
4. Write a Pratt parser for a small boolean expression grammar
   (`&&`/`||`/`!` with parentheses) of the kind a WAF condition
   (`07-security/waf.md`) would use, and verify precedence with tests.
5. Re-implement one rule with `nom` or `winnow` and compare it against your
   hand-written version on readability and error-message quality; decide
   which idiom fits config parsing versus binary framing.
