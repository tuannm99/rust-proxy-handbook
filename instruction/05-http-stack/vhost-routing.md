# Virtual Host / Multi-Tenant Routing

Routing to different backends by *which site* a request is for, not just its path — the thing `router.md` assumes has already been decided.

## What to learn
### Host-header routing (post-TLS, HTTP layer)
`router.md` covers matching path and method within one backend's route table. A proxy fronting multiple sites/tenants first has to pick *which* route table to use at all, based on the `Host` header (HTTP/1.1) or the `:authority` pseudo-header (HTTP/2, see `01-network/http2.md`) — both carry the same information, just framed differently. This lookup happens after TLS termination, since the header is inside the encrypted request.

```rust
use std::collections::HashMap;

struct VirtualHost {
    upstream_pool: UpstreamPoolId,
}

fn route_by_host<'a>(
    vhosts: &'a HashMap<String, VirtualHost>,
    host_header: &str,
) -> Option<&'a VirtualHost> {
    // strip a trailing :port — clients often send "example.com:443"
    let host = host_header.split(':').next().unwrap_or(host_header);
    vhosts.get(host)
}
```
Gotcha: default to rejecting (404, or a dedicated catch-all vhost) any `Host` that doesn't match a known entry. Silently falling through to some "default" backend is exactly how Host-header injection and cache-poisoning bugs happen — an attacker sends an unexpected `Host` and gets routed somewhere unintended, or a cache keyed loosely on Host serves the wrong tenant's response to someone else.

Gotcha: normalize before lookup, the same way `router.md` normalizes
paths. Hostnames are case-insensitive (`EXAMPLE.com` must match
`example.com`), a trailing dot is legal and means the same thing
(`example.com.`), and IDN/punycode forms (`xn--...`) must map to one
canonical representation. A lookup table keyed on raw bytes treats each
variant as a different tenant — which is either a 404 for a legitimate
user or, if one variant falls through to a default, a routing bypass.

Gotcha: the port split above is wrong for IPv6 literals (`[::1]:8443`),
which contain colons. Handle the bracketed form explicitly rather than
splitting on the first colon.

### The Host vs SNI mismatch (domain fronting)
Under TLS, the client tells you the hostname **twice**: once in the
ClientHello's SNI extension, in the clear, before the handshake completes;
and once in the `Host`/`:authority` header, encrypted, after. Nothing in
the protocol forces them to agree.

An attacker exploits the gap: connect with `SNI: allowed.example.com` —
which is what any network middlebox, SNI-based firewall, or SNI-routing
layer sees and permits — then send `Host: internal.example.com` inside the
encrypted request, which is what your proxy actually routes on. This is
domain fronting, and it turns an SNI-based access control into decoration.

If your proxy terminates TLS and routes on `Host`, validate that the two
match and reject when they don't. If a legitimate client genuinely needs
them to differ, that should be an explicit configuration, not an accident.

Gotcha: this is the same check `router.md` describes for
`Host`/`:authority`/SNI consistency. Do it once, in one place, at the
point where the connection's TLS metadata is still available alongside the
request — not in two components that can disagree about which is
authoritative.

### SNI-based routing (pre-TLS, without decrypting)
TLS's ClientHello carries the target hostname in cleartext in the SNI extension — even under TLS 1.3, until Encrypted Client Hello (ECH) is common. A proxy can peek at just the ClientHello, read SNI, and decide *which TLS-terminating backend* to forward the still-encrypted bytes to, without holding any of that backend's private key itself. This is how a passthrough layer in front of several independent TLS terminators (each with its own cert) routes traffic without becoming a fourth party to the TLS session.

Gotcha: peeking means reading bytes off the socket that you must then
forward *including* the bytes you consumed — the backend needs the
complete ClientHello. Buffer and replay it rather than consuming it, and
bound both the buffer and the time you'll wait for a complete ClientHello,
or a client that connects and sends 3 bytes forever is a
connection-exhaustion vector (`07-security/ddos.md`).

Gotcha: SNI is optional. A client connecting by IP, an old client, or a
deliberate probe may send none at all — decide whether that's a default
backend or a rejection, and be aware that "default backend" is the same
fall-through risk as an unmatched `Host`.

### SNI routing vs Host-header routing are not interchangeable
SNI routing decides *before* decryption and only sees the hostname — it can't see path, method, or any header. Host-header routing decides *after* decryption and sees the full request, but requires the proxy doing the routing to also be the one terminating TLS (holding the cert). A proxy can do either, or both in sequence (SNI-route to the right TLS-terminating instance, which then Host-routes to the right tenant's upstream pool) — know which one a given deployment actually needs before building it.

### Certificate selection at multi-tenant scale
Terminating TLS for many hostnames means picking a certificate *during*
the handshake, from SNI, before you know anything else about the request.
rustls exposes this as a resolver callback (`ResolvesServerCert`), and
three practical concerns follow:
- **The callback is on the handshake hot path.** Loading and parsing a
  cert from disk there adds latency to every new connection; keep parsed
  certs in memory, keyed by hostname, and reload on config change
  (`09-architecture/config.md`) rather than per-handshake.
- **Wildcards and exact matches must have defined precedence.** With both
  `example.com` and `*.example.com` configured, an exact match should win;
  wildcards match exactly one label (`*.example.com` covers
  `a.example.com` but not `a.b.example.com`).
- **Expiry is per-tenant and silent.** One tenant's expired cert fails
  only that tenant's handshakes, so overall traffic looks fine. Export
  time-to-expiry as a per-certificate metric
  (`08-observability/alerting.md`); this is the mTLS gotcha from
  `07-security/auth.md` multiplied by tenant count.

### Wildcard/multi-domain certs interact with both
A wildcard cert (`*.example.com`) or a SAN cert covering many hostnames lets one TLS-terminating instance answer for many vhosts under one handshake — simplifying Host-header routing (one cert, many `Host` values) but making SNI routing moot for those hostnames (they're all the same backend by definition). See `01-network/tls.md` for the handshake mechanics this depends on.

### Isolation between tenants, not just routing
Routing separates tenants' *traffic*; it does nothing to separate their
*resource consumption*. One tenant's traffic spike consumes the shared
connection budget (`07-security/ddos.md`), the shared upstream pool
concurrency, the shared cache capacity (`05-http-stack/cache.md`), and the
worker threads — so every other tenant degrades. That's the noisy-neighbor
problem, and in a multi-tenant proxy it's the default behavior unless you
design against it.

The controls are per-tenant versions of things you already have: rate
limits keyed by tenant (`07-security/ratelimit.md`), a concurrency cap per
vhost so one tenant can't hold every upstream connection, and cache
accounting per tenant so one tenant's large objects don't evict another's
working set.

Gotcha: make the tenant identity part of every cross-cutting key, and do
it early. Retrofitting a tenant dimension into cache keys, metric labels
(`08-observability/metrics.md`), and rate-limit keys after the fact is a
large, error-prone change — and the failure mode of getting it wrong is
serving one tenant's cached response to another.

## Practice
Build these in order.

1. In `proxy`, add a vhost map keyed by normalized `Host` (lowercased,
   trailing dot stripped, port removed with IPv6 handled), loaded from
   config. **Done when** `EXAMPLE.com.`, `example.com:443`, and
   `example.com` all resolve to the same tenant.
2. Reject unmatched hosts explicitly. **Done when** a request with an
   unknown `Host` gets a 404 or catch-all response and never reaches any
   real tenant's backend — verify with upstream-side logging, not just the
   client's view.
3. Add `Host`/SNI consistency validation. **Done when** a connection with
   `SNI: a.example.com` carrying `Host: b.example.com` is rejected; use
   `openssl s_client -servername` to construct it.
4. Add per-hostname certificate selection via a rustls resolver backed by
   an in-memory map. **Done when** two hostnames present different certs
   on the same listener, exact matches beat wildcards, and the resolver
   makes no filesystem access per handshake.
5. Export per-certificate time-to-expiry as a metric. **Done when** a cert
   expiring in 7 days is visible without anyone having to check manually.
6. Add a config-reload test for the vhost map (`labs/13-hot-reload`).
   **Done when** swapping the map under concurrent load changes routing
   for new requests without affecting any in-flight one.
7. Add per-tenant concurrency caps and rate limits, with tenant as a label
   on metrics and a component of cache keys. **Done when** one tenant
   driving a saturating load test leaves another tenant's p99 unchanged —
   measure both, since this is the whole point.
8. (Stretch) Implement SNI-peeking passthrough: buffer the ClientHello,
   extract SNI, forward the raw bytes including what you buffered. **Done
   when** a TLS connection terminates at the correct backend with the
   proxy never holding its private key, and a client that stalls
   mid-ClientHello is timed out rather than held.
