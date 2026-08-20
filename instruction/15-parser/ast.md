# AST

The tree a parser produces, and the question of whether you need one at
all. Follows `15-parser/parser.md`.

## What to learn

### What an AST is, and what it deliberately omits
An Abstract Syntax Tree represents the *structure* of parsed input,
stripped of syntax that carried no meaning: parentheses, commas,
whitespace, the exact spelling of a keyword. `(a + b)` and `a + b` produce
the same AST because the parentheses only guided parsing; the tree already
encodes the grouping.

```rust
enum Expr {
    Number(u64),
    Ident(String),
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
}
```

Gotcha: the recursive `Box<Expr>` is unavoidable for a tree of unknown
depth, but it means every node is a separate heap allocation. For a config
parsed once at startup this is irrelevant; for anything parsed per-request
it is a reason to consider the arena representation below.

### Parse straight to your type, or build a tree first?
The central decision. Two valid answers:

- **Parse directly into the target struct.** The parser's actions build
  your `Config`/`Request` as it goes; there is no intermediate tree. This
  is what `05-http-stack/parser.md` does — a parsed HTTP request *is* the
  useful structure, and interposing an AST would be pure overhead. Choose
  this when the parsed form is the form you use.
- **Build an AST first, then process it.** Choose this when the same parsed
  input feeds *several* consumers, or needs multiple passes: validate,
  then resolve references, then lower to a runtime form. A config that
  supports `include` directives, variable interpolation, or defaults
  inherited from a parent block is far cleaner as a tree you walk (see
  `15-parser/visitor.md`) than as something assembled in one pass.

The failure mode is building an AST reflexively because tutorials do. If
there is exactly one consumer and one pass, the AST is a layer of
indirection that buys nothing.

### Arena trees: the Rust-idiomatic shape
A tree of `Box`ed nodes with parent pointers fights the borrow checker
(`03-rust/ownership.md`) and fragments the heap. The idiomatic answer,
identical to the trick in `13-algorithms/lru.md`, is to store all nodes in
one `Vec` and link them by `usize` index:

```rust
struct Ast { nodes: Vec<Node> }
struct Node { kind: NodeKind, children: Vec<u32> } // indices, not Box
```

This makes the whole tree one allocation, cache-friendly to walk, trivially
`Clone`, and free of lifetime puzzles. Downside: indices are not
type-checked the way references are, so a stale index is a logic bug the
compiler will not catch — bound their lifetime to the arena's.

### Spans: keep the source location on every node
Attach the byte range each node came from. Semantic errors found *after*
parsing ("upstream `web` referenced here was never defined") can then point
at the exact line, just like the lexer's errors (`15-parser/lexer.md`). A
tree without spans forces every later error to say only "somewhere in your
config."

## Practice
1. For the config grammar from `15-parser/parser.md`, decide explicitly
   whether to parse straight into your `Config` struct or via an AST —
   write down the reason. If the format has no includes or interpolation,
   the honest answer is usually "no AST."
2. Add one feature that forces the tree: `include "other.conf"` or
   `$var` interpolation. Now build the AST and observe why a single-pass
   parse could not have handled it cleanly.
3. Represent that AST as an arena (`Vec` + `u32` indices), not `Box`ed
   nodes, and walk it to confirm no lifetime friction.
4. Attach a source span to every node and produce one post-parse semantic
   error ("duplicate `listen` directive") that points at the offending
   line.
5. Contrast this with `05-http-stack/parser.md`: articulate why an HTTP
   request is parsed directly into a struct with no AST, and what would
   have to change about the problem for a tree to be worth it.
