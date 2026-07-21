# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Role: mentor, not implementer

Claude acts as a **mentor** in this repo, never as the implementer:

- **Never write or complete implementation code** in `proxy/` or `labs/` —
  not even if the user explicitly asks "just write it" or "do it for me."
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
- The workspace: a Cargo workspace at repo root (`labs/`, `proxy/`) where
  the user implements what the handbook teaches. Every crate here is a stub
  (`todo!()` in `main.rs`) — see "Role: mentor, not implementer" above.
  `cargo check --workspace` should always pass (stubs compile).

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
instruction/12-testing/         load testing, fuzzing, chaos engineering, CI/static tooling
instruction/13-algorithms/      data structures/algorithms underpinning routing, WAF, rate limiting, cache, load balancing, DDoS mitigation
instruction/14-memory/          allocator, arena, slab allocator, object/buffer pools, fragmentation
instruction/15-parser/          general lexer/parser/AST/visitor theory underneath HTTP and config parsing
instruction/16-kernel/          TCP stack, epoll/io_uring internals, page cache, scheduler, RSS/RPS, XDP/eBPF
instruction/17-performance/     CPU cache, false sharing, NUMA, memory layout, branch prediction, SIMD
instruction/18-distributed/     Raft, gossip, leader election, distributed cache — optional/advanced, beyond a single proxy instance
instruction/19-reading-source/  structured reading of nginx/envoy/haproxy/pingora/hyper/tokio/mio/quinn source
instruction/20-reference/       glossary, cheatsheets
instruction/21-reading-list/    books, RFCs, open-source references
```

Each topic is one file, named after its concept (e.g. `instruction/06-proxy/load-balancer.md`). There is deliberately no `instruction/10-projects/` — that content now lives directly in each `labs/NN-*` crate's own README (Goal + Practice) and in `proxy/README.md` for the final build. The directory number encodes prerequisite order for `00`-`12` — earlier numbers are foundational to later ones (e.g. `instruction/02-linux/epoll.md` and `instruction/03-rust/async.md` underpin `instruction/04-runtime/tokio.md`, which underpins the actual proxy work in `instruction/06-proxy/`).

`13`-`21` are a deep-dive/foundations layer, not a strict continuation of the `00`-`12` sequence — they're referenced *from* earlier directories rather than only read after them (e.g. `06-proxy/load-balancer.md` cross-references `13-algorithms/` for Maglev/rendezvous hashing). When new content would duplicate an existing topic file's scope (e.g. a load-testing tool, an architecture pattern), add it to the existing directory (`12-testing/`, `09-architecture/`) instead of creating a new top-level number.

## The Cargo workspace

**`proxy/` (package `proxy`) is the actual deliverable — the single,
complete, production-grade L7 proxy that this entire repo builds toward.**
`labs/` is the only other member of the workspace, and exists entirely to
prepare the user to build it.

```
labs/                  # 18 numbered, progressively harder exercises — the only route to proxy/
  00-tcp-server/       # package tcp-server — tokio only, TCP echo server
  01-http-parser/      # package http-parser — hand-written HTTP/1.1 parser, no hyper
  02-http-server/      # package http-server — hyper/hyper-util, plain HTTP server
  03-router/           # package router — method+path routing
  04-static-server/    # package static-server — streaming static files
  05-reverse-proxy/    # package reverse-proxy — hyper client, forwards to an upstream pool
  06-load-balancer/    # package load-balancer — RR/least-conn/consistent-hash/smooth-WRR/Maglev
  07-tls/              # package tls — tokio-rustls termination, ALPN
  08-http2/            # package http2 — multiplexing/flow-control specifics
  09-http3/            # package http3 — QUIC via quinn
  10-cache/            # package cache — HTTP response caching
  11-rate-limit/       # package rate-limit — token bucket / sliding window
  12-waf/              # package waf — rule-based filtering, Aho-Corasick
  13-hot-reload/       # package hot-reload — config reload without dropping connections
  14-plugin/           # package plugin — request/response middleware
  15-prometheus/       # package prometheus-lab — metrics export
  16-opentelemetry/    # package opentelemetry-lab — distributed tracing export
  17-ebpf/             # package ebpf-lab — XDP/eBPF packet filtering
proxy/                 # package proxy — THE final L7 proxy: TLS, security, observability,
                       # dynamic config, tokio-rustls/tracing/serde
```

`labs/` builds up the skills needed for `proxy/` one crate at a time, each
focused on a single mechanism, often deliberately avoiding the high-level
crate that would normally hide it (`00-tcp-server` and `01-http-parser` in
particular reach for raw `libc`/manual parsing precisely to expose what
tokio/hyper otherwise do for you) — none of them are the finished product,
and none should be mistaken for one. Each crate has its own README pointing
back at the relevant handbook file(s) and stating its own "done" criteria
(Goal + Practice); there is no separate `instruction/10-projects/` layer on
top of them anymore.

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
  specific `labs/` or `proxy/` crate path.
```

`instruction/21-reading-list/` is different: plain annotated lists
(book/RFC/project name + one line on why it's relevant), no `## What to
learn`/`## Practice` sections — keep that format if extending it.

Each `labs/NN-*` crate's own README carries its `## Goal` (a concrete "done"
definition) and Handbook-references list directly — there is no separate
`instruction/10-projects/project-0N.md` layer restating it; don't recreate
one. `proxy/README.md` plays the same role for the final build.

When extending any file, keep the `# Title` and any existing intro, follow
the section structure above, and cross-reference other handbook files by
path (e.g. `see instruction/07-security/request-smuggling.md`) rather than
duplicating their content.

When adding a new topic, place it in the most relevant numbered directory
(or propose a new numbered directory for a genuinely new phase, as
`instruction/12-testing/` was) and follow the same template.
