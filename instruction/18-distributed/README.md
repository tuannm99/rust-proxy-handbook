# Distributed

Multi-node coordination — out of scope for a single L7 proxy instance, but
foundational if this ever grows toward a distributed cache or CDN. Treat
this folder as optional/advanced relative to `proxy/README.md`.

## Planned topics

- `raft.md` — leader-based consensus, the mechanism behind most production coordination systems
- `gossip.md` — epidemic-style state propagation, used by systems like Consul/Cassandra for membership
- `leader-election.md` — the specific sub-problem Raft (and simpler alternatives) solve
- `distributed-cache.md` — sharding/replicating a cache across nodes, once a single-node cache (`05-http-stack/cache.md`) isn't enough

Consistent hashing is covered in `13-algorithms/consistent-hash.md`, and
single-service-instance discovery in `06-proxy/service-discovery.md` — not
duplicated here.
