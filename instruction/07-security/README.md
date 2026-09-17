# Security

Phase 7. A proxy is the thing standing between the internet and everything
behind it, so most of these are about the gap between what *you* think a
request says and what the upstream will think it says.

## Files

**Identity**
- `auth.md` — where auth sits in the pipeline, propagating identity upstream, and stripping forged identity headers
- `jwt.md` — signature and claim validation, algorithm confusion, JWKS rotation, revocation
- `mtls.md` — client certificates, CA scoping, expiry as a scheduled outage

**Input handling**
- `normalization.md` — parser differentials: decode depth, Unicode, parameter precedence
- `request-smuggling.md` — CL.TE/TE.CL/TE.TE, downgrade smuggling, what an attacker actually gains
- `waf.md` — rule engines, signatures, anomaly scoring, detection-only rollout

**Abuse and overload**
- `ratelimit.md` — what to key on, cost-based charging, distributed limiting and its failure modes
- `ip-filtering.md` — CIDR matching, IPv4-mapped IPv6, trusting `X-Forwarded-For` correctly
- `ddos.md` — cost asymmetry, resource ceilings, accept-rate limiting, decompression bombs
- `slowloris.md` — the three slow-client variants and rate floors
- `load-shedding.md` — shed vs queue, time-based bounds, adaptive concurrency

## Reading order

`normalization.md` early — `router.md`, `waf.md`, and
`request-smuggling.md` are all applications of it, and none of them work
if it's wrong. `auth.md` before `jwt.md`/`mtls.md`, since it frames what
the mechanisms are for. `ddos.md` before `slowloris.md` and
`load-shedding.md`, which are its two largest sub-topics.

These back `labs/11-rate-limit`, `labs/12-waf`, and `labs/17-ebpf`, and
`proxy/README.md` lists most of this directory as required reading for the
final build.
