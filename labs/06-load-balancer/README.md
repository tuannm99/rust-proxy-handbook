# 06-load-balancer

## Goal

Implement and compare load-balancing algorithms against a pool of upstreams:
round robin, weighted/smooth WRR, least connections, consistent hash,
rendezvous hash, Maglev. Done means: you can swap algorithms behind one
interface and demonstrate the specific property each one gives you (even
distribution, minimal remap on upstream removal, session affinity).

## Handbook references
- `instruction/06-proxy/02-load-balancer.md` — round robin, least connections, consistent hash
- `instruction/13-algorithms/smooth-wrr.md`, `rendezvous-hash.md`, `maglev.md` — the deeper variants

## Run

```
cargo run -p load-balancer
```
