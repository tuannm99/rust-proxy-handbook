# Proxy Taxonomy: What Kind of Proxy Is This

Part of the from-scratch fundamentals series — see `01-network/01-fundamentals.md`
for the full index. The whole repo builds one specific point in this
space; this file is where "which one, and why" gets answered directly,
using the vocabulary from `02-addressing.md` and `03-byte-streams.md`.

## What to learn

### Forward proxy vs reverse proxy: whose interest does it serve
A **forward proxy** sits in front of *clients*, on their behalf: a
company's outbound web proxy that every employee's browser is configured
to route through, or a VPN-adjacent service that hides a client's real IP
from the sites it visits. The client knows the proxy exists and is
configured to use it; the destination server typically doesn't know a
proxy is involved at all (or only sees the proxy's IP as if it were the
client).

A **reverse proxy** — what this entire repo builds — sits in front of
*servers*, on their behalf. Clients connect to the proxy thinking they're
talking to the real service; the proxy decides which real backend
actually handles the request. The backend, not the client, is who the
proxy exists to protect and manage. This is the opposite trust
relationship from a forward proxy even though the wire-level mechanics
(accept a connection, parse, forward, relay a response) look similar.

Gotcha: "proxy" without qualification, in casual conversation, could mean
either — always resolve which one is meant from context. This handbook
never means the forward kind.

### NAT gateway: address translation without protocol awareness
A NAT gateway (`02-addressing.md`) rewrites addresses (SNAT/DNAT) without
understanding anything about the protocol riding on top — it doesn't
parse HTTP, doesn't know what a "request" is, and can't make a routing
decision based on a URL path or header. It operates purely on IP/port.
This is the same layer a plain **L4 load balancer** operates at: it picks
a backend based on the connection's 4-tuple (or a hash of it) and then
just forwards bytes, both directions, without looking inside them.

### Where an L7 (reverse) proxy differs
An **L7 proxy** — this repo's `proxy/` — terminates the connection
(ends the client's TCP/TLS session at the proxy itself), reads and
understands the application protocol inside it (HTTP), and *then* decides
what to do: which backend, whether to cache, whether to rate-limit,
whether to reject. This is strictly more expensive per-request (parsing,
possibly re-encrypting to the backend) and strictly more capable — it's
the entire reason `06-proxy/02-load-balancer.md` can route by path or
header and a NAT gateway can't, and the entire reason
`07-security/06-waf.md` can inspect a request body and a plain L4
balancer can't.

```
L4 (NAT gateway / TCP load balancer):
  client --[TCP]--> [rewrite addresses, forward bytes] --[TCP]--> backend
  (never parses what's inside; two ends of one effective connection)

L7 (this repo's proxy):
  client --[TCP+TLS+HTTP]--> [terminate, parse, decide] --[new TCP+TLS+HTTP]--> backend
  (two genuinely separate connections; proxy reads and can rewrite the request)
```

### API gateway: an L7 reverse proxy with more opinions
An **API gateway** is usually just an L7 reverse proxy with a specific
feature set bundled in by convention — auth, per-client rate limiting,
request/response transformation, sometimes protocol translation
(REST-to-gRPC). There's no hard technical line between "reverse proxy"
and "API gateway"; it's a marketing/scope distinction. Everything this
repo builds in `07-security/` and `09-architecture/02-plugin.md` is, in
aggregate, what turns a bare reverse proxy into something people would
call an API gateway.

### CDN: a reverse proxy at massive, distributed scale
A **CDN** is architecturally a reverse proxy too — it terminates client
connections and decides how to serve them — specialized for one thing:
caching content close to users across many geographic points of presence,
so that most requests never reach the origin at all.
`05-http-stack/07-cache.md` and `05-http-stack/08-cache-stampede.md`
describe the caching mechanics a single proxy instance needs; a CDN is
the same mechanics multiplied across thousands of edge locations with a
way to keep them (eventually) consistent.

### Sidecar proxy: one reverse proxy per process, not one per fleet
A **sidecar proxy** (Envoy in a service mesh, e.g. Istio) is the same L7
reverse-proxy mechanics again, deployed differently: instead of one
shared proxy fronting a whole fleet, every single service instance gets
its own tiny proxy instance running alongside it (same pod, in
Kubernetes terms — see `02-linux/06-containers.md`), handling that one
instance's inbound and outbound traffic. The mechanics this handbook
teaches — load balancing, retries, circuit breaking, mTLS — are identical;
only the deployment topology differs.

### Where this repo's `proxy/` sits
Concretely: `proxy/` is a **reverse, L7, terminating** proxy — it accepts
client connections, understands HTTP, and forwards decisions (not just
bytes) to an upstream pool. Nothing in this repo builds a forward proxy,
and nothing here builds pure L4 forwarding as the end goal (though
`06-proxy/02-load-balancer.md` and `07-security/09-ddos.md` both touch
L4-adjacent concerns — accept-rate limiting, connection-level shedding —
because a real L7 proxy still has to survive at the connection level
before it gets to parse anything).

## Practice
1. For each of the following, name whether it's a forward proxy, reverse
   proxy, L4 gateway, or none of those, and explain why: your company's
   corporate web filter; nginx in front of a web app; a home router's NAT;
   Cloudflare in front of a website; an Envoy sidecar next to a
   microservice.
2. Explain, in your own words, why an L4 load balancer cannot implement
   `06-proxy/02-load-balancer.md`'s consistent-hash-by-session-cookie
   routing, but `proxy/` can.
3. Read `07-security/06-waf.md`'s opening paragraph and explain which
   layer (L4 or L7) a WAF has to operate at, and why — tie your answer
   back to "terminates the connection" from this file.
4. Sketch (on paper, not code) the connection topology for a request that
   passes through a home NAT router, a CDN, and finally this repo's
   `proxy/` before reaching an application server — label each hop as L4
   or L7.
