# proxy

**This is the deliverable** — the single, complete, production-grade L7
proxy that the whole handbook builds toward (project 4 in
`instruction/10-projects/`). Everything from `milestones/01-echo` through
`milestones/03-reverse-proxy` plus TLS, security, observability, and dynamic
config.

Handbook references:
- `instruction/10-projects/project-04.md` — goal and scope
- `instruction/07-security/*.md`, `instruction/08-observability/*.md` (incl. `alerting.md`), `instruction/09-architecture/*.md` (incl. `rolling-restart.md`)
- `instruction/01-network/tls.md`, `instruction/01-network/proxy-protocol.md`

Run with:

```
cargo run -p proxy
```
