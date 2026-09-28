# 03-router

## Goal

A router, mounted on an HTTP server ([`labs/02-http-server`](../02-http-server)), that matches
method + path to a handler.

## Done when

- [ ] Unmatched paths get `404`; a matched path with the wrong method gets `405` with an `Allow` header listing the permitted methods.
- [ ] Path parameters (`/users/:id`) are extracted and passed to the handler.
- [ ] When a static route and a parameter route both match (`/users/me` vs `/users/:id`), the winner is the one your documented precedence rule says, and a test proves it.
- [ ] Trailing-slash behavior (`/a` vs `/a/`) is a written decision — distinct routes, redirect, or equivalent — with a test.
- [ ] Percent-encoded paths are handled deliberately: `/files/a%2Fb` does not accidentally match a two-segment route.
- [ ] Lookup time stays flat as routes grow: a benchmark with 10 routes vs 1000 routes shows no linear slowdown ([`instruction/13-algorithms/radix-tree.md`](../../instruction/13-algorithms/radix-tree.md)).
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/05-http-stack/03-router.md`](../../instruction/05-http-stack/03-router.md) — matching method+path, trailing slashes, path params
- [`instruction/13-algorithms/trie.md`](../../instruction/13-algorithms/trie.md), [`instruction/13-algorithms/radix-tree.md`](../../instruction/13-algorithms/radix-tree.md) — the lookup structure
- [`instruction/03-rust/09-iterators-and-closures.md`](../../instruction/03-rust/09-iterators-and-closures.md) — storing handlers as `Box<dyn Fn ...>`

## Run

```
cargo run -p router
```
