# Architecture

Phase 9. How the pieces from `05-http-stack/`, `06-proxy/`, and
`07-security/` fit together into one program — and how that program gets
reconfigured, deployed, and restarted without dropping traffic.

## Files

- `components.md` — the Listener → ConnMgr → Codec → Router → Modules pipeline, and why module order is a security decision
- `plugin.md` — compile-time composition vs runtime plugins, sandboxing a guest, fail-open vs fail-closed
- `config.md` — validate-then-swap reload, what can't be hot-reloaded, snapshotting config per request
- `graceful-shutdown.md` — the drain sequence, the pre-stop delay, long-lived connections, flushing telemetry
- `rolling-restart.md` — `SO_REUSEPORT`, fd handoff, socket activation, and the state a restart loses
- `canary-deploy.md` — weighted splitting, sticky routing, automated rollback and its sample-size problem

## Reading order

`components.md` first — it's the map the rest hangs off, and its module
ordering table collects constraints scattered through `05-http-stack/` and
`07-security/`. Then `config.md` and `graceful-shutdown.md`, which back
`labs/13-hot-reload` and are prerequisites for `rolling-restart.md`.
`plugin.md` backs `labs/14-plugin`. `canary-deploy.md` last; it builds on
`06-proxy/load-balancer.md`.

This is the phase where `proxy/` stops being a collection of labs and
becomes something you could run.
