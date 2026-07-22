# Distributed

Multi-node coordination — out of scope for a single L7 proxy instance, but
foundational if this ever grows toward a distributed cache or CDN. Treat
this folder as optional/advanced relative to `proxy/README.md`.

## Status: index only — not written, genuinely optional

Nothing here is written yet, nothing links to it, and unlike the other
unwritten folders this one is outside the scope of the deliverable by
design: `proxy/` is a single-instance L7 proxy, and nothing in it requires
consensus or multi-node coordination.

The one place the boundary actually touches your work is distributed rate
limiting (`07-security/ratelimit.md`'s stretch exercise) — and that is
solved with a shared Redis counter, not with Raft.

When to come back: only if you decide to extend past `proxy/README.md`
toward a multi-node cache or CDN. Skipping this folder entirely is a valid
way to finish this handbook.

## Planned topics

- `raft.md` — leader-based consensus, the mechanism behind most production coordination systems
- `gossip.md` — epidemic-style state propagation, used by systems like Consul/Cassandra for membership
- `leader-election.md` — the specific sub-problem Raft (and simpler alternatives) solve
- `distributed-cache.md` — sharding/replicating a cache across nodes, once a single-node cache (`05-http-stack/cache.md`) isn't enough

Consistent hashing is covered in `13-algorithms/consistent-hash.md`, and
single-service-instance discovery in `06-proxy/service-discovery.md` — not
duplicated here.
