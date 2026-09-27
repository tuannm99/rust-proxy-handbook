# Macros

## What to learn

### Declarative macros: `macro_rules!`
`macro_rules!` pattern-matches on the token stream at the call site and expands to a fixed template — useful when the boilerplate is syntactic (repeated *shapes* of code) rather than logical (repeated *behavior*, which a plain function or generic already handles). Reach for a function or generic first; a macro earns its keep only once those genuinely can't express what you need.

```rust
macro_rules! metric_counter {
    ($name:ident) => {
        static $name: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    };
}
metric_counter!(REQUESTS_TOTAL);
metric_counter!(ERRORS_TOTAL);
```

### Hygiene
Macro-generated identifiers don't accidentally capture or collide with identifiers at the call site — Rust macros are hygienic, unlike C's textual `#define`. This is why they're far safer than preprocessor macros, and also why they can feel harder to debug: you can't just mentally text-substitute the expansion. `cargo expand` (an installable subcommand) is the tool for actually seeing what a macro produced.

### Where proc-macros actually show up here
You will use derive and attribute macros constantly without writing one: `#[derive(Serialize, Deserialize)]` (serde, config parsing — `09-architecture/03-config.md`), `#[derive(thiserror::Error)]` (`03-rust/08-error-handling.md`), `#[tokio::main]`/`#[tokio::test]` (attribute macros that rewrite `fn main()` into runtime setup plus your body). Writing a proc-macro from scratch — a separate crate type, using `syn`/`quote` to parse and regenerate token streams — is real but rare; most engineers, senior ones included, go years without authoring one. Recognizing what a derive/attribute macro expands to matters far more day to day.

```rust
#[tokio::main] // attribute macro: expands to runtime setup + calls your async body
async fn main() { /* ... */ }
```

### When not to reach for a macro
If a generic function, a trait, or a builder (`03-rust/13-api-design-and-modules.md`) can express what you want, prefer it. Macros are opaque to IDE tooling in ways generics aren't (weaker autocomplete, compiler errors pointing at expansion sites instead of your source), and they're harder for a future reader — including future you — to trace through than a named function call.

## Practice
1. Install `cargo expand` and run it against a small struct with `#[derive(Serialize, Deserialize)]`; read the generated `impl` and identify roughly what serde's derive produced.
2. Write a `macro_rules!` that generates a boolean-check function per enum variant (e.g. `is_502!(status)`); then rewrite the same thing as a plain function or trait method and decide which reads better.
3. Deliberately shadow a variable name inside a `macro_rules!` expansion and confirm, via `cargo expand` or a quick test, that it does not collide with a same-named variable at the call site — hygiene in action.
4. Find an attribute macro already in this workspace (`#[tokio::main]` in any `labs/*/src/main.rs`) and run `cargo expand --bin <name>` to see the runtime bootstrap it generates.
5. Write down, in your own words, why you would not reach for a proc-macro to solve a boilerplate problem in `proxy` before ruling out a `macro_rules!`, generic, or trait-based solution.
