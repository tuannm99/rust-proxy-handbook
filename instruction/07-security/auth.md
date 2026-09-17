# JWT & mTLS

## What to learn
### JWT validation
A JWT is a signed, base64url-encoded JSON structure (`header.payload.signature`).
Validating one at the proxy edge means: verify the signature against a
known key (HS256 shared secret or RS256/ES256 public key — never accept
`alg: none`, and never let the token's own `alg` header pick the
verification algorithm), then check claims: `exp` (expired?), `nbf` (not
yet valid?), `aud`/`iss` (issued for this service?).

```rust
struct Claims {
    exp: u64,
    nbf: Option<u64>,
    aud: String,
    sub: String,
}

fn validate_claims(claims: &Claims, expected_aud: &str, now: u64) -> Result<(), &'static str> {
    if claims.exp <= now { return Err("expired"); }
    if let Some(nbf) = claims.nbf { if now < nbf { return Err("not yet valid"); } }
    if claims.aud != expected_aud { return Err("wrong audience"); }
    Ok(())
}
```
Gotcha: signature verification and claim checks are two separate steps —
a library that "parses" a JWT without you explicitly calling verify may
hand you claims from a token whose signature was never checked. In Rust,
prefer a maintained crate (`jsonwebtoken`) over hand-rolling HMAC/RSA.

### Algorithm confusion, concretely
"Never let the token's own `alg` header pick the verification algorithm"
deserves the actual attack, because it explains the shape of the fix.

Your service verifies RS256 with a *public* key — public by definition,
so the attacker has it. The attacker rewrites the header to
`{"alg":"HS256"}` and signs the token using that public key's bytes as the
HMAC secret. A verifier that reads `alg` from the token and dispatches on
it then calls "HMAC-verify with the configured key", the key material
matches, and the forged token validates. One header change turns a
signature check into a rubber stamp.

The fix is not "reject `alg: none`" — it's to pin the accepted algorithm
in *your* configuration and ignore the token's claim about itself
entirely. `jsonwebtoken`'s `Validation { algorithms: vec![Algorithm::RS256], .. }`
does this; the failure mode appears when someone "helpfully" passes the
parsed header's algorithm into the validator.

Gotcha: the same rule extends to every attacker-controlled header field.
`kid` (key ID) is used to select a key — if you use it as a filesystem
path or a database key without validation, it's path traversal or
injection with extra steps. `jku`/`x5u` name a *URL* to fetch keys from;
honoring them is a server-side request forgery primitive that also lets
the attacker supply the verification key. Accept `kid` only as a lookup
into a fixed key set; never honor `jku`/`x5u`.

### Key rotation and JWKS
Production identity providers rotate signing keys and publish them at a
JWKS endpoint. The proxy fetches that key set, caches it, and looks up the
right key by `kid`. Two failure modes follow directly:

**The unknown-kid stampede.** When rotation happens, tokens arrive with a
`kid` you don't have, and the natural implementation refetches JWKS to
find it. An attacker who sends tokens with random `kid` values then drives
one outbound JWKS fetch per request — you've built a request amplifier
pointed at your identity provider, which will start rate-limiting you, at
which point *all* auth fails. Rate-limit the refetch itself (at most one
per N seconds regardless of how many unknown kids arrive) and serve a 401
in the meantime.

**Fetch failure.** If JWKS is unreachable, fail static on the cached key
set (`06-proxy/service-discovery.md` — same principle): keep validating
with the last known good keys rather than rejecting all traffic. A
rotation you missed will produce 401s for genuinely new tokens; an empty
key cache produces 401s for *everything*.

### Clock skew
`exp` and `nbf` are absolute timestamps compared against your clock, and
your clock disagrees with the issuer's. Without leeway, a token minted
milliseconds ago fails `nbf` on a server running a couple of seconds
behind, and the resulting 401s are intermittent, unreproducible, and
correlate with nothing an application developer can see.

Allow a small leeway (30-60 seconds is conventional) on both `exp` and
`nbf`. Note the asymmetry in risk: leeway on `nbf` costs nothing, leeway
on `exp` extends the life of an expired token by that much — which is
fine at 60 seconds and not fine at an hour.

Gotcha: the leeway hides clock drift rather than fixing it. Monitor actual
skew (`08-observability/metrics.md`); a server drifting past your leeway
fails every token at once, and you want the alert before that.

### Revocation: the thing JWT can't do
A stateless signed token is valid until `exp` because validating it
requires no server state — that is the entire performance argument for
JWT, and it means you cannot revoke one. A compromised token, a logged-out
session, a fired employee: all still authenticate until expiry.

The practical answers, in increasing cost: keep `exp` short (minutes, with
a refresh token flow for renewal), maintain a denylist of revoked `jti`
values (which reintroduces shared state, but only for the small set of
explicitly revoked tokens), or accept the window deliberately and document
it. Choosing "short expiry" is not a cop-out — it's the standard answer —
but it must be an actual decision, because the default of a 24-hour `exp`
means a 24-hour compromise window.

### mTLS at the proxy layer
With mutual TLS, the proxy's TLS server (see `01-network/tls.md`) requests
and verifies a client certificate during the handshake, before any HTTP
request is even parsed. The proxy checks the cert chains to a trusted CA
and optionally checks specific fields (CN/SAN) against an allowlist. This
authenticates the *connection*, not necessarily the end user — often
combined with JWT for user-level identity on top of service-level mTLS.

Gotcha: client cert verification happens at the TLS layer (rustls
`WebPkiClientVerifier` or similar) — if you check certs only after
accepting the connection at the HTTP layer, you've already spent
resources on an unauthenticated peer, which is itself a DoS vector.

Gotcha: "chains to a trusted CA" is a weaker statement than it sounds. If
the trusted CA issues certs to anyone (a public CA, or a corporate CA used
fleet-wide), then *any* valid cert authenticates, and you've verified that
the client exists rather than that it's authorized. Pin on specific
SAN/CN values, or use a dedicated CA that signs only for this trust
domain.

Gotcha: certificates expire, and an expired *client* cert fails at
handshake time with an error that never reaches your HTTP-layer logging.
Track certificate expiry as a metric with an alert well before the date
(`08-observability/alerting.md`); "the client cert expired overnight" is a
top-tier cause of morning outages, and the signal is invisible unless you
went looking for it.

Gotcha: revocation for mTLS has the same problem JWT does, with worse
ergonomics — CRLs are large and stale, OCSP adds a network dependency to
the handshake path. Short-lived client certs (hours, auto-renewed, as SPIFFE
and most service meshes do) sidestep it the same way short `exp` does for
JWT.

### Passing identity upstream — and stripping it first
This is the proxy-specific half of auth, and the most commonly botched.
Once the proxy validates identity, upstreams need to know who the caller
is, conventionally via a header (`X-User-Id`, `X-Auth-Subject`). Upstreams
then trust that header, because "the proxy set it."

Which means: **if a client can send that header and the proxy passes it
through, the client is authenticated as anyone.** The attacker doesn't
break your JWT validation — they just skip it, sending `X-User-Id: admin`
with no token at all, and your proxy dutifully forwards it.

The rule is unconditional: strip every identity header from the inbound
request *before* auth runs, then set it yourself from validated claims.
Strip on an allowlist basis (remove anything in your identity namespace),
not a denylist of headers you remembered.

```rust
// before auth, unconditionally:
for name in IDENTITY_HEADERS {          // X-User-Id, X-Auth-*, etc.
    req.headers_mut().remove(name);
}
// after successful validation:
req.headers_mut().insert("x-user-id", claims.sub.parse()?);
```
Gotcha: this is the same trust-boundary bug as `X-Forwarded-For` in
`ip-filtering.md`. Any header your infrastructure treats as trusted must
be stripped at the edge, every time, on every path — including error
paths and any route that skips auth.

### Where auth belongs in the pipeline
Auth should run as early as possible in the component pipeline (see
`09-architecture/components.md`: right after routing determines which
route's auth policy applies, before any upstream call or expensive
processing like WAF body inspection). Reject unauthenticated/invalid
requests before they consume upstream capacity.

Gotcha: "fail closed" must be the default for an unrouted request. A
request that matches no route, or matches a route with no auth policy
configured, must not fall through to "no auth required" — that turns a
config typo into an open door. Make the policy type non-optional so the
compiler forces a decision (`Public` is a variant you write deliberately,
not the absence of configuration).

Gotcha: comparisons of secrets (static API keys, HMAC digests) must be
constant-time (`subtle`'s `ConstantTimeEq`), not `==`. A short-circuiting
compare leaks the correct prefix through timing, one byte at a time.
Signature verification inside a good JWT crate already handles this; your
hand-written API-key fallback path does not.

## Practice
Build these in order.

1. In `proxy`, add JWT validation with `jsonwebtoken`: pinned algorithm,
   signature + `exp`/`aud`, 401 otherwise. **Done when** a valid token
   passes and a token with a tampered payload fails.
2. Mount the algorithm-confusion attack against your own endpoint: take
   your RS256 public key, sign a token with it as an HS256 secret, and
   send it. **Done when** it is rejected — and, to prove the test is real,
   temporarily configure the validator to accept the token's own `alg` and
   watch the forged token succeed.
3. Add JWKS fetching with `kid` lookup, a refetch rate limit, and
   fail-static on fetch failure. **Done when** 1000 requests with random
   `kid` values produce at most one outbound JWKS fetch, and blackholing
   the JWKS endpoint leaves existing keys working.
4. Add clock-skew leeway and a skew metric. **Done when** a token minted
   2 seconds in the future validates, and your metric reports the actual
   offset against the issuer.
5. Add identity-header stripping before auth and injection after. **Done
   when** a request carrying `X-User-Id: admin` and no token is rejected
   with 401 and the upstream receives no `X-User-Id` at all — write the
   passthrough version first and confirm the upstream sees `admin`, so
   you've seen the hole.
6. Make route auth policy a non-optional enum. **Done when** adding a new
   route without specifying a policy fails to compile rather than
   defaulting to public.
7. Configure the TLS listener to require and verify a client cert for one
   route (`01-network/tls.md`), pinned to a specific SAN. **Done when** a
   cert from the right CA but wrong SAN is rejected, and cert expiry is
   exported as a metric.
8. Add a pipeline-ordering test. **Done when** it proves an invalid JWT is
   rejected before upstream selection *and* before WAF body inspection
   runs.
