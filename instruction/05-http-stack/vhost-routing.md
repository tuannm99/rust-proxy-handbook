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

### SNI-based routing (pre-TLS, without decrypting)
TLS's ClientHello carries the target hostname in cleartext in the SNI extension — even under TLS 1.3, until Encrypted Client Hello (ECH) is common. A proxy can peek at just the ClientHello, read SNI, and decide *which TLS-terminating backend* to forward the still-encrypted bytes to, without holding any of that backend's private key itself. This is how a passthrough layer in front of several independent TLS terminators (each with its own cert) routes traffic without becoming a fourth party to the TLS session.

### SNI routing vs Host-header routing are not interchangeable
SNI routing decides *before* decryption and only sees the hostname — it can't see path, method, or any header. Host-header routing decides *after* decryption and sees the full request, but requires the proxy doing the routing to also be the one terminating TLS (holding the cert). A proxy can do either, or both in sequence (SNI-route to the right TLS-terminating instance, which then Host-routes to the right tenant's upstream pool) — know which one a given deployment actually needs before building it.

### Wildcard/multi-domain certs interact with both
A wildcard cert (`*.example.com`) or a SAN cert covering many hostnames lets one TLS-terminating instance answer for many vhosts under one handshake — simplifying Host-header routing (one cert, many `Host` values) but making SNI routing moot for those hostnames (they're all the same backend by definition). See `01-network/tls.md` for the handshake mechanics this depends on.

## Practice
1. In `proxy`, add a `HashMap<String, UpstreamPoolId>` keyed by `Host` (stripped of port), loaded from config (`09-architecture/config.md`); route unmatched hosts to a 404 or a dedicated catch-all vhost, never to a default tenant's backend.
2. Add a config-reload test: change the vhost map via the hot-reload path and confirm in-flight routing decisions aren't affected mid-request.
3. Stretch goal: implement SNI-peeking passthrough — read just enough of the ClientHello to extract SNI, then proxy the raw TCP bytes to the matching backend without terminating TLS yourself.
4. Write a request with a `Host` header that doesn't match any configured vhost and confirm the proxy rejects it rather than falling through to any real tenant.
