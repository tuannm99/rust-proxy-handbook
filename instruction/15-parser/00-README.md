# Parser

General parsing theory. `05-http-stack/01-parser.md` is the HTTP-specific
application of these ideas (and `labs/01-http-parser` is where you
implement it) — this folder is the reusable foundation underneath it,
also applicable to `09-architecture/03-config.md`'s config file parsing.

## Status: written, but optional for the labs

The theory below is written, but no `labs/` crate strictly requires it:
`05-http-stack/01-parser.md` is deliberately self-contained for
`labs/01-http-parser`. Read this folder when you reach `labs/13-hot-reload`
and decide to write your own config format rather than leaning on `serde` +
`toml` — that is the point where lexer/AST/visitor stop being theory
(`05-config-parser.md` makes that "do you even need a parser?" call
explicitly). Otherwise treat it as background that deepens the HTTP parser.

## Files

- `01-lexer.md` — tokenizing raw bytes/text into a token stream
- `02-parser.md` — turning a token stream into structured data (recursive descent vs parser combinators)
- `03-ast.md` — representing parsed structure as a tree, and why you'd want one vs parsing straight into your target type
- `04-visitor.md` — the visitor pattern for walking/transforming an AST
- `05-config-parser.md` — applying the above to a proxy's own config format, feeds `09-architecture/03-config.md`
