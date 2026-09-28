# 14-plugin

## Goal

A minimal plugin/middleware system: request/response hooks that can be
composed and reordered without recompiling the core proxy logic.

## Done when

- [ ] At least three plugins (for example request-ID injection, a header rewrite, an auth check) run as a chain whose order comes from config.
- [ ] Changing the order in config changes the behavior without recompiling.
- [ ] A plugin can short-circuit with its own response, and later plugins in the chain don't run.
- [ ] A plugin that errors or panics produces a `500` for that request; the process and other connections are unaffected.
- [ ] The per-request cost of the chain is benchmarked, and you've written down whether `dyn` dispatch with boxed futures or a compile-time enum fits your plugin set ([`instruction/03-rust/18-async-traits.md`](../../instruction/03-rust/18-async-traits.md)).
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/09-architecture/02-plugin.md`](../../instruction/09-architecture/02-plugin.md)
- [`instruction/03-rust/18-async-traits.md`](../../instruction/03-rust/18-async-traits.md), [`instruction/03-rust/07-traits-and-generics.md`](../../instruction/03-rust/07-traits-and-generics.md)

## Run

```
cargo run -p plugin
```
