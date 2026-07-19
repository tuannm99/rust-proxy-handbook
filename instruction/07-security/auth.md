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

### Where auth belongs in the pipeline
Auth should run as early as possible in the component pipeline (see
`09-architecture/components.md`: right after routing determines which
route's auth policy applies, before any upstream call or expensive
processing like WAF body inspection). Reject unauthenticated/invalid
requests before they consume upstream capacity.

## Practice
1. In `proxy`, add JWT validation using the
   `jsonwebtoken` crate: verify signature + `exp`/`aud`, reject otherwise
   with 401.
2. Configure the proxy's TLS listener (see `01-network/tls.md`) to require
   and verify a client certificate for one route, using `tokio-rustls`.
3. Add a pipeline stage ordering test: confirm an invalid JWT is rejected
   before the request reaches the upstream-selection code.
4. (Stretch) Support two auth modes per route (JWT-only vs mTLS-only vs
   both) driven by config (see `09-architecture/config.md`).
