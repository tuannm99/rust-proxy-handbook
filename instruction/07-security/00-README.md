# Security

Phase 7. A proxy is the thing standing between the internet and everything
behind it, so most of these are about the gap between what *you* think a
request says and what the upstream will think it says.

## Files

**Identity**
- `01-auth.md` — where auth sits in the pipeline, propagating identity upstream, and stripping forged identity headers
- `02-jwt.md` — signature and claim validation, algorithm confusion, JWKS rotation, revocation
- `03-mtls.md` — client certificates, CA scoping, expiry as a scheduled outage

**Input handling**
- `04-normalization.md` — parser differentials: decode depth, Unicode, parameter precedence
- `05-request-smuggling.md` — CL.TE/TE.CL/TE.TE, downgrade smuggling, what an attacker actually gains
- `06-waf.md` — rule engines, signatures, anomaly scoring, detection-only rollout

**Abuse and overload**
- `07-ratelimit.md` — what to key on, cost-based charging, distributed limiting and its failure modes
- `08-ip-filtering.md` — CIDR matching, IPv4-mapped IPv6, trusting `X-Forwarded-For` correctly
- `09-ddos.md` — cost asymmetry, resource ceilings, accept-rate limiting, decompression bombs
- `10-slowloris.md` — the three slow-client variants and rate floors
- `11-load-shedding.md` — shed vs queue, time-based bounds, adaptive concurrency

## Reading order

`04-normalization.md` early — `router.md`, `06-waf.md`, and
`05-request-smuggling.md` are all applications of it, and none of them work
if it's wrong. `01-auth.md` before `02-jwt.md`/`03-mtls.md`, since it frames what
the mechanisms are for. `09-ddos.md` before `10-slowloris.md` and
`11-load-shedding.md`, which are its two largest sub-topics.

These back `labs/11-rate-limit`, `labs/12-waf`, and `labs/17-ebpf`, and
`proxy/00-README.md` lists most of this directory as required reading for the
final build.
