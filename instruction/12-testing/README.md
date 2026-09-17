# Testing

Phase 10 in practice, even though it's numbered 12. Everything here is
run *against* `proxy/` once it exists — these are the exercises that tell
you whether the previous nine phases actually work.

## Files

- `load-testing.md` — generating realistic load, what to measure, open vs closed models
- `fuzzing.md` — fuzzing the parser and any attacker-facing input
- `chaos.md` — injecting upstream failures, latency, and partial degradation
- `ci-tooling.md` — clippy, miri, sanitizers, and what to gate merges on

## Where it goes

Almost every `## Practice` section in this handbook ends with a step that
needs one of these: a load test to prove a change helped
(`06-proxy/load-balancer.md`), a fuzz target for an input parser
(`05-http-stack/parser.md`, `07-security/ip-filtering.md`), a chaos
injection to fire an alert (`08-observability/alerting.md`).

The ordering that matters: load-test *before* optimizing
(`17-performance/` is explicit that profiling comes after a measurement
pointing somewhere), and fuzz anything that parses attacker-controlled
bytes before shipping it.
