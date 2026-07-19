# Rust Nginx Handbook

A structured handbook + implementation workspace for building a production-grade L7 proxy in Rust.

- `instruction/00-introduction/` ... `instruction/12-testing/` — the handbook topics, in learning order (see `CLAUDE.md`)
- `proxy/` — **the actual deliverable**: the single, complete L7 proxy this whole repo builds toward
- `milestones/` — 3 progressive learning stages from `instruction/10-projects/` that build up to `proxy/` (plain TCP echo → plain HTTP → forwarding to an upstream); not deliverables in themselves
- `labs/` — small standalone exercises that implement a piece of the stack raw (no hyper/tokio-reactor), to understand what the libraries do for you

Each crate under `milestones/`, `labs/`, and `proxy/` is a stub (`todo!()` in
`main.rs`) with a README pointing back to the relevant handbook file(s) —
implement them yourself as you work through the handbook.

```
cargo check --workspace   # confirm everything still builds
cargo run -p milestone-01-echo
```
