# 06-load-balancer

## Goal

Implement and compare load-balancing algorithms against a pool of
upstreams: round robin, smooth weighted round robin, least connections,
consistent hash, rendezvous hash, Maglev — behind one interface.

## Done when

- [ ] All strategies implement one trait and are selected from a config string at runtime ([`instruction/03-rust/07-traits-and-generics.md`](../../instruction/03-rust/07-traits-and-generics.md)).
- [ ] Smooth WRR with weights `5, 1, 1` produces the interleaved sequence `a a b a c a a` (nginx's), not `a a a a a b c`.
- [ ] Over 100,000 picks, each weighted strategy's distribution is within 1% of the configured weights.
- [ ] Least-connections sends measurably less traffic to an upstream you make artificially slow.
- [ ] For consistent hash, rendezvous hash, and Maglev: removing 1 of N upstreams remaps roughly `1/N` of keys (measured over 100,000 keys), and the same key always maps to the same upstream otherwise.
- [ ] A benchmark of `pick()` per strategy, with Maglev's lookup cost independent of upstream count.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/06-proxy/02-load-balancer.md`](../../instruction/06-proxy/02-load-balancer.md) — round robin, least connections, consistent hash
- [`instruction/13-algorithms/smooth-wrr.md`](../../instruction/13-algorithms/smooth-wrr.md), [`instruction/13-algorithms/consistent-hash.md`](../../instruction/13-algorithms/consistent-hash.md), [`instruction/13-algorithms/rendezvous-hash.md`](../../instruction/13-algorithms/rendezvous-hash.md), [`instruction/13-algorithms/maglev.md`](../../instruction/13-algorithms/maglev.md) — the deeper variants
- [`instruction/03-rust/18-async-traits.md`](../../instruction/03-rust/18-async-traits.md) — whether the strategy trait needs `dyn` at all

## Run

```
cargo run -p load-balancer
```
