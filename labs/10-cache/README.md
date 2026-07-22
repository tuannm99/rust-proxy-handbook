# 10-cache

## Goal

An in-memory HTTP response cache keyed by method+URI (folding in
`Vary`-listed headers), with freshness/revalidation and a purge path.

## Handbook references
- `instruction/05-http-stack/cache.md` — proxy caching semantics
- `instruction/13-algorithms/lru.md`, `lfu.md`, `arc.md`, `tinylfu.md` — eviction algorithms

## Run

```
cargo run -p cache
```
