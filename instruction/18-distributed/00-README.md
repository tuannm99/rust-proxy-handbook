# Distributed

Multi-node coordination — out of scope for a single L7 proxy instance, but
foundational if this ever grows toward a distributed cache or CDN. Treat
this folder as optional/advanced relative to `proxy/00-README.md`.

## Status: written, but genuinely optional

The content below is written, but unlike the rest of the handbook it sits
*outside* the deliverable by design: `proxy/` is a single-instance L7
proxy, and nothing in it requires consensus or multi-node coordination.
Every file here leads with "you probably don't need this" and points at the
cheaper single-node answer. Skipping this folder entirely is a valid way to
finish the handbook.

The one place the boundary touches your work is distributed rate limiting
(`07-security/07-ratelimit.md`'s stretch exercise, solved with a shared Redis
counter, not Raft) and a shared cache (`04-distributed-cache.md`, the file
here closest to real proxy work). Come back only if you extend past
`proxy/00-README.md` toward a multi-node cache or CDN.

## Files

- `01-raft.md` — leader-based consensus, the mechanism behind most production coordination systems
- `02-gossip.md` — epidemic-style state propagation, used by systems like Consul/Cassandra for membership
- `03-leader-election.md` — the specific sub-problem Raft (and simpler alternatives) solve
- `04-distributed-cache.md` — sharding/replicating a cache across nodes, once a single-node cache (`05-http-stack/07-cache.md`) isn't enough

Consistent hashing is covered in `13-algorithms/consistent-hash.md`, and
single-service-instance discovery in `06-proxy/07-service-discovery.md` — not
duplicated here.
