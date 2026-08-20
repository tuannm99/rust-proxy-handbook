# Visitor

Walking and transforming an AST without scattering the traversal logic
across your codebase. Follows `15-parser/ast.md`.

## What to learn

### The problem the visitor pattern solves
Once you have a tree (`15-parser/ast.md`), you will walk it more than once:
to validate, to resolve references, to lower it to a runtime config, maybe
to pretty-print it back out. Writing the "recurse into every child" logic
afresh in each of those is where bugs breed — one pass forgets to descend
into a nested block and silently ignores half the config. A visitor
factors the *traversal* out once, leaving each pass to supply only what it
does at each node.

```rust
trait Visitor {
    fn visit_block(&mut self, b: &Block) { walk_block(self, b); }
    fn visit_entry(&mut self, e: &Entry) { walk_entry(self, e); }
}
// walk_* functions own the recursion; default methods call them,
// so an impl overrides only the nodes it cares about.
fn walk_block<V: Visitor + ?Sized>(v: &mut V, b: &Block) {
    for e in &b.entries { v.visit_entry(e); }
}
```

Gotcha: the recursion must live in the free `walk_*` functions, not in the
trait method bodies. If a visitor overrides `visit_block` and forgets to
call `walk_block`, traversal stops there. Making `walk` separate lets an
override do its work *and* delegate: `fn visit_block(&mut self, b) { /* my
stuff */ walk_block(self, b); }`.

### Shared vs mutable walks
Two shapes, and they are genuinely different in Rust:

- **`&self` walk** for read-only passes — validation, collecting names,
  computing a metric. Multiple of these can even run without conflict.
- **`&mut self` walk** for transformation — constant folding, applying
  defaults, rewriting. Here Rust's aliasing rules bite: you cannot hold a
  mutable borrow of the tree while also mutating a node inside it. The
  usual escape is the arena form from `15-parser/ast.md` — walk indices and
  index back into `&mut nodes[i]`, so the borrow is one node at a time, not
  the whole tree.

### Passes over one giant match
For a small AST you do not need the trait at all — a single recursive `fn
lower(node) -> Runtime` with a `match` is clearer than the visitor
machinery. The visitor earns its complexity when there are *many* node
kinds and *many* passes; below that threshold it is over-engineering, the
same judgement call as "do I even need an AST" in `15-parser/ast.md`.

### Where a proxy actually meets this
Config processing is the honest use case: parse (`15-parser/parser.md`) →
AST (`15-parser/ast.md`) → a validation visitor (every `upstream`
referenced by a `route` exists, no duplicate `listen`) → a lowering visitor
that produces the immutable runtime config the proxy swaps in on hot reload
(`09-architecture/config.md`). Keeping validation and lowering as separate
visitors means an invalid config is rejected *whole*, before any part of
the new config goes live — which is the atomicity `09-architecture/config.md`
requires.

## Practice
1. Add a read-only validation visitor over the config AST from
   `15-parser/ast.md` that checks cross-references (every `route`'s
   `upstream` is defined) and reports *all* violations with spans, not just
   the first.
2. Add a mutating visitor that applies inherited defaults (a child block
   without `timeout` gets the parent's), using the arena/index form so the
   `&mut` borrow stays per-node.
3. Write a lowering visitor that turns the validated AST into the immutable
   runtime config struct the proxy will swap in — the tail end of
   `labs/13-hot-reload`.
4. Deliberately break traversal: override one `visit_*` without calling its
   `walk_*`, feed a nested config, and confirm the pass silently skips the
   subtree — then fix it and note why the walk/visit split prevents this
   class of bug.
5. For a two-node toy AST, write the same lowering as a plain recursive
   `match` and compare; decide at what number of node kinds and passes the
   visitor stops being over-engineering.
