# Authentication

Where auth sits in a proxy's pipeline, and what the proxy does with an
identity once it has one. The two mechanisms have their own files:
`07-security/02-jwt.md` (bearer tokens) and `07-security/03-mtls.md` (client
certificates). They are frequently combined — mTLS authenticating the
calling *service*, a JWT authenticating the *user* on top of it.

## What to learn
### Where auth belongs in the pipeline
Auth should run as early as possible in the component pipeline (see
`09-architecture/01-components.md`: right after routing determines which
route's auth policy applies, before any upstream call or expensive
processing like WAF body inspection). Reject unauthenticated/invalid
requests before they consume upstream capacity.

"After routing" is not an accident — the policy is per route, so you must
know the route before you know which policy applies. That ordering has a
consequence worth planning for: anything running *before* routing (IP
filtering, connection limits) cannot depend on identity, and anything that
needs identity necessarily runs after the router has already done work.

### Fail closed on an unconfigured route
A request that matches no route, or matches a route whose auth policy was
never configured, must not fall through to "no auth required" — that turns
a config typo into an open door.

Make the policy type non-optional so the compiler forces a decision:

```rust
enum AuthPolicy {
    Public,             // a variant you write deliberately
    Jwt { audience: String },
    MutualTls { allowed_sans: Vec<String> },
    Both { .. },
}

struct Route {
    // ...
    auth: AuthPolicy,   // not Option<AuthPolicy>
}
```
Gotcha: this is the difference between "we forgot to configure auth" being
a compile error and being a production incident. `Option<AuthPolicy>` with
a `None => allow` arm is the same bug written in a way that looks
deliberate.

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
`07-security/08-ip-filtering.md`. Any header your infrastructure treats as
trusted must be stripped at the edge, every time, on every path —
including error paths and any route that skips auth.

Gotcha: stripping must happen at a single, early point, alongside
hop-by-hop header removal (`05-http-stack/02-hop-by-hop-headers.md`), not
inside the auth module. A route configured `Public` skips the auth module
entirely — and if stripping lived there, that route forwards forged
identity headers straight through.

### Comparing secrets takes constant time
Any path that compares a static API key, an HMAC digest, or a shared
secret must use a constant-time comparison (`subtle`'s `ConstantTimeEq`),
not `==`. A short-circuiting compare returns faster the earlier it finds a
mismatch, which leaks the correct prefix through timing — recoverable one
byte at a time.

Gotcha: a good JWT crate already does this inside signature verification.
The vulnerable code is almost always the hand-written fallback path — the
"simple API key" auth someone added for an internal integration.

### Authentication is not authorization
The proxy answering "who is this?" does not answer "may they do this?"
Coarse authorization (this route requires this scope or this SAN) belongs
at the edge because it lets you reject cheaply; fine-grained
authorization (may this user edit *this* document) requires data only the
upstream has.

Gotcha: a proxy that enforces coarse rules can lull upstreams into
skipping their own checks — and then an internal caller that bypasses the
proxy has no enforcement at all. Treat edge authorization as defense in
depth, and say so explicitly to the teams behind it.

## Practice
Build these in order.

1. Make route auth policy a non-optional enum. **Done when** adding a new
   route without specifying a policy fails to compile rather than
   defaulting to public.
2. Add identity-header stripping at the same early point as hop-by-hop
   stripping, and injection after successful validation. **Done when** a
   request carrying `X-User-Id: admin` and no token is rejected with 401
   and the upstream receives no `X-User-Id` at all — write the
   passthrough version first and confirm the upstream sees `admin`, so
   you've seen the hole.
3. Verify stripping on the paths that skip auth. **Done when** a route
   configured `Public` still strips forged identity headers, and so does
   the 404 path.
4. Implement the mechanisms: `07-security/02-jwt.md` for bearer tokens,
   `07-security/03-mtls.md` for client certs. **Done when** a route
   configured `Both` requires a valid cert *and* a valid token.
5. Add a constant-time comparison to any static-secret path. **Done when**
   a timing test over many samples cannot distinguish a wrong-first-byte
   key from a wrong-last-byte key.
6. Add a pipeline-ordering test. **Done when** it proves an invalid JWT is
   rejected before upstream selection *and* before WAF body inspection
   runs.
