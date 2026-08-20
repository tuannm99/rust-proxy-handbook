# Config Parser

Applying lexer → parser → AST → visitor to a proxy's own config format.
This ties `15-parser/lexer.md` through `15-parser/visitor.md` to the
runtime concern in `09-architecture/config.md`.

## What to learn

### First decision: do you even write a parser?
For most proxies the answer is no — you define the config as `serde`
structs and let `toml`/`yaml`/`json` do the parsing. `serde` gives you
parsing, type-checked deserialization, and good-enough error messages for
free, and `09-architecture/config.md` assumes exactly this. Hand-writing a
config parser is justified only when you need something `serde` cannot
express: a custom directive syntax (nginx's `location` blocks), includes,
variable interpolation, or conditional sections. Write the parser because
the *format* demands it, not to practice parsing.

```rust
#[derive(Deserialize)]           // this is the default — no parser at all
struct Config {
    listen: SocketAddr,
    upstreams: HashMap<String, Upstream>,
    routes: Vec<Route>,
}
```

### If you do write one: the pipeline
The stages compose exactly as the previous files describe: bytes → lexer
(`15-parser/lexer.md`) → recursive-descent parser (`15-parser/parser.md`)
→ AST if the format has includes/interpolation (`15-parser/ast.md`) →
validation and lowering visitors (`15-parser/visitor.md`) → an immutable
runtime `Config`. The output type is the whole point: everything upstream
exists to produce one validated, immutable struct the proxy can swap in.

### Validation is a distinct phase from parsing
Parsing answers "is this well-formed?"; validation answers "is this
*coherent*?" — does every `route` name an `upstream` that exists, is the
`listen` port in range, are there no duplicate server names. Keep them
separate: a parse error is a syntax mistake, a validation error is a
semantics mistake, and conflating them produces confusing messages. Both
must carry the source span (`15-parser/ast.md`) so the operator sees the
line.

### Atomicity: the reload constraint drives the design
`09-architecture/config.md` requires that a bad reload never takes the
proxy down — the running config keeps serving while the new one is
rejected. That means the *entire* pipeline, parse through validation, must
complete and produce a fully-built runtime `Config` before anything goes
live. Never mutate the live config incrementally as you parse. Build the
new one entirely off to the side; swap the pointer only on success.

```rust
fn reload(text: &str, live: &ArcSwap<Config>) -> Result<(), Vec<ConfigError>> {
    let next = parse_and_validate(text)?; // all-or-nothing
    live.store(Arc::new(next));           // atomic swap, only on success
    Ok(())
}
```

Gotcha: collect *all* errors before returning (error recovery from
`15-parser/parser.md`, multi-error validation from `15-parser/visitor.md`).
An operator reloading a 500-line config wants every problem at once, not a
fix-one-rerun loop — but the reload as a whole is still rejected atomically.

### Untrusted-ish input still needs bounds
A config file is more trusted than a network request, but a malformed or
malicious one should still fail gracefully, not crash the reload thread.
The nesting-depth bound from `15-parser/parser.md` applies here too — a
config with a million nested blocks must error, not overflow the stack and
abort the process mid-reload.

## Practice
1. Define your proxy config as `serde` structs first and deserialize from
   TOML — this is the `labs/13-hot-reload` baseline. Only proceed to a
   hand-written parser if you add a feature `serde` cannot express.
2. If you go custom: assemble the full pipeline from
   `15-parser/lexer.md`→`visitor.md` producing an immutable runtime
   `Config`, and keep parse errors and validation errors as distinct types.
3. Implement atomic reload with `arc-swap`: build the new config entirely
   before swapping, and prove with a test that a config failing validation
   leaves the live config untouched and serving.
4. Make reload report every error at once: feed a config with a syntax
   error, an undefined-upstream reference, and a duplicate directive, and
   confirm all three come back in one call, each with its line.
5. Feed a pathologically nested config and confirm the reload thread
   returns an error rather than aborting the process — then wire this
   reload into `proxy` and drive it under `12-testing/load-testing.md`
   traffic to confirm in-flight requests are never dropped by a reload.
