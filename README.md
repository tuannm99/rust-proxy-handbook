# Rust Nginx Handbook

A structured handbook + implementation workspace for building a production-grade L7 proxy in Rust.

- [`instruction/00-introduction/`](instruction/00-introduction) ... [`instruction/12-testing/`](instruction/12-testing) — the handbook topics, in learning order, plus [`instruction/13-algorithms/`](instruction/13-algorithms) ... [`instruction/20-reference/`](instruction/20-reference) — a deep-dive/foundations layer cross-referenced from the topics above (see [`CLAUDE.md`](CLAUDE.md))
- [`proxy/`](proxy) — **the actual deliverable**: the single, complete L7 proxy this whole repo builds toward
- [`labs/`](labs) — 18 numbered, progressively harder exercises (`00-tcp-server` through `17-ebpf`) that build up the skills [`proxy/`](proxy) needs, one mechanism at a time; not deliverables in themselves

Each crate under [`labs/`](labs) and [`proxy/`](proxy) is a stub (`todo!()` in `main.rs`)
with a README pointing back to the relevant handbook file(s) — implement
them yourself as you work through the handbook.

```
cargo check --workspace   # confirm everything still builds
cargo run -p tcp-server
```
