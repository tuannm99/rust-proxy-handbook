# Algorithms

The data structures and algorithms that the rest of the handbook builds on
top of. Routing needs a trie/radix tree, WAF needs Aho-Corasick, rate
limiting needs token bucket, caching needs LRU/TinyLFU, load balancing
needs smooth WRR/consistent hash/Maglev, DDoS mitigation needs
count-min-sketch/HyperLogLog. This folder covers the algorithm itself;
`05-http-stack/`, `06-proxy/`, and `07-security/` cover how it's wired into
the proxy and cross-reference back here for the deep dive.

## Written

These are the ones a `labs/` exercise currently depends on:

- `smooth-wrr.md` — nginx's current-weight algorithm, effective weight, passive failure response
- `consistent-hash.md` — virtual node sizing, the balance problem, bounded loads
- `rendezvous-hash.md` — HRW: minimal disruption with no ring and no shared state
- `maglev.md` — O(1) lookup table, permutation construction, the population loop
- `lru.md` — arena-backed LRU, the every-read-is-a-write lock problem, CLOCK, scan resistance
- `token-bucket.md` — lazy refill, GCRA, lock-free updates, bounding key growth
- `aho-corasick.md` — trie + failure links, output links, literal prefiltering
- `count-min-sketch.md` — approximate frequency in fixed memory, error bounds, decay
- `regex-engine.md` — backtracking vs automata, ReDoS, lazy DFA, `RegexSet`
- `slab.md` — intrusive free list, generational handles, the shrink problem

## Planned

- `dfa.md` / `fsm.md` — deterministic finite automata underlying regex engines and protocol parsers
- `trie.md` / `radix-tree.md` — prefix-based lookup structures, used by `05-http-stack/router.md`
- `ring-buffer.md` — fixed-size circular buffer, used by `08-observability/logging.md`
- `heap.md` / `priority-queue.md` — used by `06-proxy/load-balancer.md`'s least-response-time variant, timer wheels
- `hashmap.md` — open addressing vs chaining, used everywhere
- `skiplist.md` — ordered structure alternative to balanced trees
- `bloom-filter.md` / `hyperloglog.md` — probabilistic membership/cardinality at scale, used by `07-security/ddos.md`
- `sliding-window.md` / `leaky-bucket.md` — the rate limiting algorithms `token-bucket.md` contrasts against
- `lfu.md` / `arc.md` / `tinylfu.md` — the eviction policies `lru.md` introduces in outline
