# Algorithms

The data structures and algorithms that the rest of the handbook builds on
top of. Routing needs a trie/radix tree, WAF needs Aho-Corasick, rate
limiting needs token bucket, caching needs LRU/TinyLFU, load balancing
needs smooth WRR/consistent hash/Maglev, DDoS mitigation needs
count-min-sketch/HyperLogLog. This folder covers the algorithm itself;
`05-http-stack/`, `06-proxy/`, and `07-security/` cover how it's wired into
the proxy and cross-reference back here for the deep dive.

## Written

Most of these underpin a specific `labs/` exercise directly; a few
(`dfa.md`, `fsm.md`, `skiplist.md`, `priority-queue.md`) are general
foundations without a dedicated lab of their own — their own file says
where they apply instead:

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
- `sliding-window.md` — fixed window's boundary-burst bug, sliding log, the two-counter approximation
- `leaky-bucket.md` — meter form (token bucket's mirror image) vs queue form (output smoothing)
- `lfu.md` — O(1) frequency-bucket structure, the stale-winner problem, aging
- `arc.md` — T1/T2/B1/B2, adapting the recency/frequency split from ghost-list hits
- `tinylfu.md` — admission over eviction, count-min-sketch frequency, doorkeeper, W-TinyLFU's LRU window
- `dfa.md` — table-driven matching, subset construction, minimization
- `fsm.md` — Mealy/Moore machines, enum+match vs the typestate pattern
- `trie.md` — segment-based prefix lookup, used by `05-http-stack/router.md`
- `radix-tree.md` — compressed trie, the longest-common-prefix split, what production routers use
- `ring-buffer.md` — fixed-size circular buffer, SPSC lock-free logging, used by `08-observability/logging.md`
- `heap.md` — array-backed binary heap, decrease-key via lazy deletion or an indexed heap
- `priority-queue.md` — timer wheels, why `tokio::time` doesn't use a heap for timeouts
- `hashmap.md` — chaining vs open addressing, SwissTable, hash flooding on attacker-controlled keys
- `skiplist.md` — probabilistic levels, why lock-free skip lists are common where lock-free trees aren't
- `bloom-filter.md` — membership with no false negatives, sizing, the TinyLFU doorkeeper use
- `hyperloglog.md` — cardinality estimation in fixed memory, mergeable across shards, used by `07-security/ddos.md`
