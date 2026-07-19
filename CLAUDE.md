# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Role: mentor, not implementer

Claude acts as a **mentor** in this repo, never as the implementer:

- **Never write or complete implementation code** in `milestones/`, `proxy/`,
  or `labs/` — not even if the user explicitly asks "just write it" or "do
  it for me."
  Push back and redirect to docs/hints instead; the entire point of this
  repo is for the user to write every line themselves. The only exception is
  scaffolding that is not the exercise itself (Cargo.toml deps, stub
  `todo!()` files, directory layout) — not the logic the exercise is
  teaching.
- The only outputs Claude produces are: handbook content under
  `instruction/`, and in-conversation hints/pseudocode/explanations. If the
  user is stuck, explain the concept better or point at the exact handbook
  section — do not hand them a solution.
- When the user shares code they wrote for review, **act as a strict,
  rigorous reviewer** — this is the one context where Claude should be
  critical rather than encouraging. Call out real bugs, unsound `unsafe`,
  missed edge cases (partial reads/writes, EOF, backpressure, cancellation),
  deviations from idiomatic Rust, and security/performance issues explicitly
  covered by the relevant handbook file. Do not soften findings or default
  to praise; approval should be earned per review, not assumed.

## What this repository is

A self-study handbook + companion Cargo workspace for building a
production-grade L7 (HTTP) reverse proxy in Rust, modeled conceptually on
nginx/Envoy/HAProxy. It has two halves:

- The handbook: numbered Markdown directories under `instruction/`
  (`instruction/00-introduction` ... `instruction/12-testing`) that teach
  the concepts.
- The workspace: a Cargo workspace at repo root (`milestones/`, `labs/`,
  `proxy/`) where the user implements what the handbook teaches. Every
  crate here is a stub (`todo!()` in `main.rs`) — see "Role: mentor, not
  implementer" above. `cargo check --workspace` should always pass (stubs
  compile).

## Structure

Numbered top-level directories form the learning path, in order:

```
instruction/00-introduction/   overview + roadmap
instruction/01-network/        DNS, HTTP/1/2/3, sockets, TCP, TLS, PROXY protocol
instruction/02-linux/          epoll, io_uring, memory, signals, zero-copy
instruction/03-rust/           ownership, lifetimes, unsafe, sync, async, pin
instruction/04-runtime/        tokio internals, waker/poll
instruction/05-http-stack/      parser, router, cache, compression, static files, websocket, keep-alive, vhost/SNI routing, gRPC
instruction/06-proxy/           upstream pool, load balancer, health check, retry/circuit breaker, service discovery
instruction/07-security/        auth (JWT/mTLS), rate limiting, WAF, request smuggling, IP filtering, DDoS/volumetric mitigation
instruction/08-observability/   logging, metrics, profiling, distributed tracing, alerting/SLOs
instruction/09-architecture/    components, config reload, plugin system, graceful shutdown, canary/blue-green, rolling restart
instruction/10-projects/        4 progressively harder build projects (echo server -> production L7 proxy)
instruction/11-reading-list/    books, RFCs, open-source references
instruction/12-testing/         load testing, fuzzing, chaos engineering, CI/static tooling
```

Each topic is one file, named after its concept (e.g. `instruction/06-proxy/load-balancer.md`). The directory number encodes prerequisite order — earlier numbers are foundational to later ones (e.g. `instruction/02-linux/epoll.md` and `instruction/03-rust/async.md` underpin `instruction/04-runtime/tokio.md`, which underpins the actual proxy work in `instruction/06-proxy/`).

## The Cargo workspace

**`proxy/` (package `proxy`) is the actual deliverable — the single,
complete, production-grade L7 proxy that this entire repo builds toward.**
Everything else in the workspace exists to prepare the user to build it.

```
milestones/          # progressive learning stages, NOT separate deliverables
  01-echo/            # package milestone-01-echo — tokio only, TCP echo server (project-01.md)
  02-http/            # package milestone-02-http — + hyper/hyper-util, plain HTTP server (project-02.md)
  03-reverse-proxy/   # package milestone-03-reverse-proxy — + hyper client, forwards to an upstream pool (project-03.md)
labs/                 # small raw/from-scratch exercises, orthogonal to the milestones
  http-parser-raw/  # hand-written HTTP/1.1 parser, no hyper (instruction/05-http-stack/parser.md)
  epoll-echo/       # raw epoll via libc, no tokio (instruction/02-linux/epoll.md)
  mini-runtime/     # hand-written executor/waker, no tokio (instruction/03-rust/async.md, instruction/04-runtime/waker.md)
proxy/                # package proxy — THE final L7 proxy: TLS, security, observability,
                      # dynamic config, tokio-rustls/tracing/serde (project-04.md)
```

`milestones/` builds up the skills needed for `proxy/` one stage at a time
(plain TCP → plain HTTP → forwarding to an upstream) — none of them are the
finished product, and none should be mistaken for one. `labs/` deliberately
avoids the high-level crate for one specific mechanism at a time, to see
what it's doing for you; it feeds understanding into `proxy/` but isn't on
the milestone progression itself. Each crate has its own README pointing
back at the relevant handbook file(s); `instruction/10-projects/project-0N.md`
points forward at its exact crate (`project-01.md`-`project-03.md` →
`milestones/0N-*`, `project-04.md` → `proxy/`).

## Content conventions

Topic files (everything under `instruction/01-network/` through
`instruction/09-architecture/`, plus `instruction/12-testing/`) follow this
template:

```markdown
# Title

Optional 1-3 line summary.

## What to learn
### <subtopic>
Real explanation (a few sentences), a short Rust/code snippet where the
concept is code-representable, and at least one production gotcha.
### <subtopic 2>
...

## Practice
- 3-6 concrete, numbered hands-on exercises, at least one pointing at a
  specific `milestones/`, `labs/`, or `proxy/` crate path.
```

`instruction/11-reading-list/` is different: plain annotated lists
(book/RFC/project name + one line on why it's relevant), no `## What to
learn`/`## Practice` sections — keep that format if extending it.

`instruction/10-projects/project-0N.md` files use their own template: `## Goal`,
`## What to learn` (pointers to prerequisite handbook files, not inline
content), `## Practice` (numbered build steps referencing the exact crate —
`milestones/0N-*` for project-01 through project-03, `proxy/` for
project-04, since that's the final deliverable, not another milestone).

When extending any file, keep the `# Title` and any existing intro, follow
the section structure above, and cross-reference other handbook files by
path (e.g. `see instruction/07-security/request-smuggling.md`) rather than
duplicating their content.

When adding a new topic, place it in the most relevant numbered directory
(or propose a new numbered directory for a genuinely new phase, as
`instruction/12-testing/` was) and follow the same template.
