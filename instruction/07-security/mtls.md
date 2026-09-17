# mTLS (Mutual TLS)

Authenticating the *connection* with a client certificate, before any HTTP
request is parsed. `01-network/tls.md` covers the handshake mechanics;
`07-security/auth.md` covers how this combines with request-level identity.

## What to learn
### mTLS at the proxy layer
With mutual TLS, the proxy's TLS server (see `01-network/tls.md`) requests
and verifies a client certificate during the handshake, before any HTTP
request is even parsed. The proxy checks the cert chains to a trusted CA
and optionally checks specific fields (CN/SAN) against an allowlist. This
authenticates the *connection*, not necessarily the end user — often
combined with JWT (`07-security/jwt.md`) for user-level identity on top of
service-level mTLS.

Gotcha: client cert verification happens at the TLS layer (rustls
`WebPkiClientVerifier` or similar) — if you check certs only after
accepting the connection at the HTTP layer, you've already spent
resources on an unauthenticated peer, which is itself a DoS vector.

### "Chains to a trusted CA" is weaker than it sounds
If the trusted CA issues certs to anyone — a public CA, or a corporate CA
used fleet-wide — then *any* valid cert authenticates, and you have
verified that the client exists rather than that it is authorized. Pin on
specific SAN/CN values, or use a dedicated CA that signs only for this
trust domain.

Gotcha: matching on CN is legacy and ambiguous; match on SAN. And match
the *whole* value rather than a substring — a check for "contains
`payments`" is satisfied by `payments.evil.example.com`.

### Expiry is the outage you can schedule
Certificates expire, and an expired *client* cert fails at handshake time
with an error that never reaches your HTTP-layer logging — there is no
request to log. Overall traffic looks fine while one client is completely
locked out.

Track certificate expiry as a metric with an alert well before the date
(`08-observability/alerting.md`); "the client cert expired overnight" is a
top-tier cause of morning outages, and the signal is invisible unless you
went looking for it. The same applies per-tenant on the server side
(`05-http-stack/vhost-routing.md`), where one expired cert fails only that
tenant's handshakes.

### Revocation has the same problem JWT does, with worse ergonomics
CRLs are large and stale; OCSP adds a network dependency to the handshake
path, and an OCSP responder outage becomes a handshake outage unless you
soft-fail — at which point revocation isn't enforced anyway.

Short-lived client certs (hours, auto-renewed, as SPIFFE and most service
meshes do) sidestep it the same way short `exp` does for JWT: the window
during which a compromised credential works is bounded by its lifetime
rather than by a revocation mechanism you have to operate.

### What the proxy does with the verified identity
The certificate's subject is an identity the upstream usually wants. It
travels the same way JWT claims do — a header the proxy sets, after
stripping any client-supplied version of it. See
`07-security/auth.md`'s identity-propagation section; the stripping rule
is unconditional and applies here identically.

Gotcha: when the proxy terminates mTLS and opens its *own* connection
upstream, the upstream sees the proxy's identity, not the client's. If the
upstream needs to make authorization decisions on the original client, the
proxy must forward that identity explicitly — and the upstream must trust
only the proxy to set it.

## Practice
Build these in order.

1. Configure `proxy`'s TLS listener (`01-network/tls.md`) to require and
   verify a client certificate for one route, using `tokio-rustls`.
   **Done when** a client with no cert is rejected at handshake, before
   any request is parsed.
2. Pin on a specific SAN. **Done when** a certificate issued by the right
   CA but carrying the wrong SAN is rejected — the test that proves CA
   trust alone was never enough.
3. Export per-certificate time-to-expiry as a metric. **Done when** a cert
   7 days from expiry is visible on a dashboard without anyone checking
   manually.
4. Set the client cert's identity as a header after stripping any
   inbound copy. **Done when** a client sending a forged identity header
   alongside a valid cert reaches the upstream with only the
   cert-derived value.
5. (Stretch) Issue short-lived certs (1 hour) with automated renewal and
   confirm a rotation happens with zero failed handshakes.
