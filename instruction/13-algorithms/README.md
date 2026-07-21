# Algorithms

The data structures and algorithms that the rest of the handbook builds on
top of. Routing needs a trie/radix tree, WAF needs Aho-Corasick, rate
limiting needs token bucket, caching needs LRU/TinyLFU, load balancing
needs smooth WRR/consistent hash/Maglev, DDoS mitigation needs
count-min-sketch/HyperLogLog. This folder covers the algorithm itself;
`05-http-stack/`, `06-proxy/`, and `07-security/` cover how it's wired into
the proxy and cross-reference back here for the deep dive.

## Planned topics

- `aho-corasick.md` — multi-pattern string matching, used by `07-security/waf.md`
- `dfa.md` / `fsm.md` — deterministic finite automata underlying regex engines and protocol parsers
- `regex-engine.md` — how a regex engine is actually built (NFA/DFA construction, backtracking vs linear-time)
- `trie.md` / `radix-tree.md` — prefix-based lookup structures, used by `05-http-stack/router.md`
- `ring-buffer.md` — fixed-size circular buffer, used by `08-observability/logging.md`
- `slab.md` — fixed-size-object storage, complements `14-memory/slab-allocator.md`
- `heap.md` / `priority-queue.md` — used by `06-proxy/load-balancer.md`'s least-response-time variant, timer wheels
- `hashmap.md` — open addressing vs chaining, used everywhere
- `skiplist.md` — ordered structure alternative to balanced trees
- `bloom-filter.md` / `count-min-sketch.md` / `hyperloglog.md` — probabilistic structures for membership/frequency/cardinality at scale, used by `07-security/ddos.md`
- `token-bucket.md` / `sliding-window.md` / `leaky-bucket.md` — rate limiting algorithms, used by `07-security/ratelimit.md`
- `smooth-wrr.md` / `consistent-hash.md` / `rendezvous-hash.md` / `maglev.md` — load balancing algorithms beyond what `06-proxy/load-balancer.md` covers inline
- `lru.md` / `lfu.md` / `arc.md` / `tinylfu.md` — cache eviction algorithms beyond what `05-http-stack/cache.md` covers inline
