# Parser

General parsing theory. `05-http-stack/parser.md` is the HTTP-specific
application of these ideas (and `labs/01-http-parser` is where you
implement it) — this folder is the reusable foundation underneath it,
also applicable to `09-architecture/config.md`'s config file parsing.

## Planned topics

- `lexer.md` — tokenizing raw bytes/text into a token stream
- `parser.md` — turning a token stream into structured data (recursive descent vs parser combinators)
- `ast.md` — representing parsed structure as a tree, and why you'd want one vs parsing straight into your target type
- `visitor.md` — the visitor pattern for walking/transforming an AST
- `config-parser.md` — applying the above to a proxy's own config format, feeds `09-architecture/config.md`
