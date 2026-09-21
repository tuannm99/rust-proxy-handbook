# Trie (Prefix Tree)

`05-http-stack/03-router.md` covers matching method+path at the proxy level.
This file covers the structure a real router's matching is built on:
lookup by shared prefix, which is exactly what a path (`/users/:id/posts`)
is made of.

## What to learn

### The structure
A trie is a tree where each edge is labeled by one unit of the key (a
character, or for a router, a path segment), and a lookup walks the tree
one unit at a time rather than comparing whole keys:

```rust
struct TrieNode {
    children: std::collections::HashMap<String, TrieNode>, // segment -> child
    handler: Option<RouteHandler>,
}
```

Registering `/users/:id` and `/users/:id/posts` shares the `users` and
`:id` nodes — the shared prefix is stored once, and a request path is
matched by descending one segment at a time until handlers run out or the
path is consumed.

### Why a trie beats scanning a route list
A linear scan of N registered routes is O(N) per request regardless of
path shape. A segment trie is O(depth) — proportional to the path's own
length, not the number of routes registered — which matters once a
service has hundreds of routes: the 500th route costs the same lookup
time as the first.

### Static vs parameter vs wildcard segments
A real router's trie needs three kinds of children at each level: an
exact-match segment (`/users`), a parameter capture (`:id`, matches any
single segment and binds a value), and a wildcard (`*rest`, matches
everything remaining). Precedence must be explicit and consistent —
static beats parameter beats wildcard at the same level — or two routes
registered in different orders match differently, which is a correctness
bug users experience as "the route worked yesterday."

### The memory cost, and its fix
A trie keyed byte-by-byte (rather than segment-by-segment) produces long
chains of single-child nodes — one node per character of `/users` is six
hops to store one segment. This is the exact problem
`13-algorithms/radix-tree.md` fixes by merging those chains into a single
edge; read that file once a segment-level trie's node count starts to
matter.

## Practice
1. In `labs/03-router`, implement a segment-based trie (map keyed by
   path segment, not by character) and register both static and
   parameterized routes.
2. Register `/users/:id` and `/users/new` and write the precedence rule
   that makes `/users/new` match the static route rather than binding
   `id = "new"`; test both orders of registration and confirm identical
   results.
3. Benchmark trie lookup against a linear scan of the same routes at
   10, 100, and 1000 registered routes; confirm the trie's lookup time
   stays flat while the scan's grows.
4. Add wildcard segment support (`/static/*path`) and confirm it only
   matches when no more specific static or parameter route exists at the
   same position.
