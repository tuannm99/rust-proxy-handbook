# Radix Tree (Compressed Trie)

`13-algorithms/trie.md` covers the plain trie and its memory cost: long
chains of single-child nodes. A radix tree (also called a Patricia trie)
is the fix, and it's what production HTTP routers (`httprouter`, `gin`,
`actix-web`'s router) actually implement rather than a plain trie.

## What to learn

### Edges hold strings, not single units
Where a trie's edge is labeled by one character or segment, a radix
tree's edge is labeled by an arbitrary substring — a whole run of nodes
that would otherwise chain with no branching gets collapsed into one
edge:

```rust
struct RadixNode {
    prefix: String,             // the shared substring this edge represents
    children: Vec<RadixNode>,   // edges diverge here
    handler: Option<RouteHandler>,
}
```

Inserting `/users` then `/user` produces one node for the shared prefix
`/user` with a branch: one child ending the string (`/user`'s own
handler, if registered) and one child continuing with `s` (`/users`'s
handler). No wasted single-child chain in between.

### Insertion: find the longest common prefix, then split
Inserting a new key walks down comparing against each edge's `prefix`
until it finds the longest common prefix with an existing edge. Three
cases follow: the new key exactly matches an edge (attach a handler
there), the new key extends past an edge (descend and repeat), or the new
key diverges partway through an edge (split that edge into a shared
prefix node with two children — the existing continuation and the new
one). Getting the split case right, including handling a handler that
was attached to the node being split, is where most from-scratch radix
tree bugs live.

### Parameter and wildcard segments still need explicit precedence
Compression is a memory/lookup optimization; it doesn't change the
routing semantics from `13-algorithms/trie.md` — static, parameter
(`:id`), and wildcard (`*rest`) segments still need the same explicit,
order-independent precedence rule. A radix tree usually keeps parameter
and wildcard branches uncompressed (as distinct children at the branch
point) precisely because they can't be merged into a literal prefix with
a static sibling.

### Why this is the production choice
Fewer nodes means fewer pointer hops per lookup and better cache
locality — for a proxy checking a route on every single request, this is
a hot-path structure, not a one-time setup cost. The complexity is
concentrated entirely in insertion (which happens once, at startup or on
config reload); lookup is the same segment-by-segment descent as a plain
trie, just over fewer, longer edges.

## Practice
1. Convert your `labs/03-router` trie into a radix tree: implement the
   longest-common-prefix insertion with the split case, and register the
   same route set from `trie.md`'s exercises.
2. Count nodes in both representations for a realistic route set (a
   REST API with `/api/v1/users`, `/api/v1/users/:id`,
   `/api/v1/orders`, `/api/v1/orders/:id/items`, ...) and confirm the
   radix tree uses meaningfully fewer.
3. Deliberately trigger the split-with-existing-handler case (insert
   `/users` after `/user` already has a handler) and write a test that
   both handlers remain reachable afterward.
4. Re-run `trie.md`'s benchmark (lookup time at 10/100/1000 routes)
   against the radix tree and compare both lookup latency and memory.
