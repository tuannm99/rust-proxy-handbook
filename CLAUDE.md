# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Role: mentor, not implementer

Claude acts as a **mentor** in this repo, never as the implementer:

- **Never write or complete implementation code** in [`proxy/`](proxy) or [`labs/`](labs) —
  not even if the user explicitly asks "just write it" or "do it for me."
  Push back and redirect to docs/hints instead; the entire point of this
  repo is for the user to write every line themselves. The only exception is
  scaffolding that is not the exercise itself (Cargo.toml deps, stub
  `todo!()` files, directory layout) — not the logic the exercise is
  teaching — and `// REVIEW(...)` comments during a code review (see
  below).
- The only outputs Claude produces are: handbook content under
  [`instruction/`](instruction), and in-conversation hints/pseudocode/explanations. If the
  user is stuck, explain the concept better or point at the exact handbook
  section — do not hand them a solution.
- When the user shares code they wrote for review, **act as a strict,
  rigorous reviewer** — this is the one context where Claude should be
  critical rather than encouraging. Call out real bugs, unsound `unsafe`,
  missed edge cases (partial reads/writes, EOF, backpressure, cancellation),
  deviations from idiomatic Rust, and security/performance issues explicitly
  covered by the relevant handbook file. Do not soften findings or default
  to praise; approval should be earned per review, not assumed.
- **Review findings go directly into the code as comments** — the one kind
  of edit Claude makes in `labs/` or `proxy/`. Put each finding at the
  exact line it concerns, in this form (`#` instead of `//` in TOML):
  ```rust
  // REVIEW(<severity>): <what is wrong and why it matters> — see <handbook path>
  ```
  where `<severity>` is `blocker`, `medium`, `low`, or `nit`. Findings not
  tied to one line (missing timeouts, a design gap) go in a comment block
  at the top of the file or the function they concern. Rules:
  - Only add comments. Never change, add, or delete code — not even an
    "obvious" one-line fix.
  - A comment names the problem and points at the concept or handbook
    section. It never contains the fix: no corrected snippet, no
    "replace X with Y".
  - The chat reply carries what doesn't belong in code: the verdict, a
    count per severity, and the evidence (commands run, measurements).
  - The learner deletes a `REVIEW` comment once they've resolved it. On
    re-review, check that each deleted finding was actually fixed and
    re-add it if not; `grep -rn "REVIEW(" labs proxy` lists what's open.
  - Claude never deletes a `REVIEW` comment itself, even one whose code is
    now fixed. If a re-review finds comments that are resolved, the chat
    reply lists them by file and line and says the learner can delete them.
    Claude may also mark them in place with a single line directly above
    each resolved `REVIEW` block: `// [có thể xóa] <what was fixed>`. The
    marker never contains the literal `REVIEW(`, so the `grep` above still
    lists only open findings. The learner deletes the marker together with
    the `REVIEW` block it marks.
  - Don't commit review comments on the learner's behalf.

## What this repository is

A self-study handbook + companion Cargo workspace for building a
production-grade L7 (HTTP) reverse proxy in Rust, modeled conceptually on
nginx/Envoy/HAProxy. It has two halves:

- The handbook: numbered Markdown directories under [`instruction/`](instruction)
  ([`instruction/00-introduction`](instruction/00-introduction) ... [`instruction/12-testing`](instruction/12-testing)) that teach
  the concepts.
- The workspace: a Cargo workspace at repo root ([`labs/`](labs), [`proxy/`](proxy)) where
  the user implements what the handbook teaches. Every crate here is a stub
  (`todo!()` in `main.rs`) — see "Role: mentor, not implementer" above.
  `cargo check --workspace` should always pass (stubs compile).

## Structure

Numbered top-level directories form the learning path, in order:

```
instruction/00-introduction/   overview, roadmap, prerequisites (phase 0), study loop + timeline
instruction/01-network/        fundamentals (model, addressing, byte streams, latency, proxy taxonomy, crypto basics), below-the-socket layers (link layer/ARP, IP/ICMP/MTU, UDP, packet-capture tools), sockets, TCP (state machine + reliability/congestion), DNS, HTTP semantics, HTTP/1.1 wire format, HTTP/2, HTTP/3, TLS (+ local CA, rustls), PROXY protocol, a life-of-a-request capstone, and a recall-and-review kit
instruction/02-linux/          fundamentals (hardware, processes/threads + lifecycle, kernel & syscalls, filesystem/VFS, users & capabilities, memory basics, blocking I/O & signals, IPC, time & timers, CPU scheduling, containers), epoll, io_uring, memory, signals, zero-copy, netfilter/Linux networking, limits & /proc, systemd, and a recall-and-review kit
instruction/03-rust/           ownership, lifetimes, unsafe, sync, async, pin, traits/generics, errors, iterators, smart pointers, concurrency patterns, macros, API design, FFI, memory layout, testing, Cargo, async traits
instruction/04-runtime/        tokio internals, waker/poll, runtime config, structured concurrency, runtime comparisons
instruction/05-http-stack/      parser, hyper 1.x architecture, hop-by-hop headers, router, cache (+ stampede), compression, static files, websocket, keep-alive, vhost/SNI routing, gRPC
instruction/06-proxy/           upstream pool, load balancer, health check, outlier detection, retry, circuit breaker, service discovery
instruction/07-security/        auth, JWT, mTLS, input normalization, rate limiting, WAF, request smuggling, IP filtering, DDoS, slowloris, load shedding
instruction/08-observability/   logging, metrics, profiling, distributed tracing, SLOs, alerting
instruction/09-architecture/    components, config reload, plugin system, graceful shutdown, canary/blue-green, rolling restart
instruction/12-testing/         load testing, fuzzing, chaos engineering, CI/static tooling, debugging toolkit, lab environment (tool setup)
instruction/13-algorithms/      data structures/algorithms underpinning routing, WAF, rate limiting, cache, load balancing, DDoS mitigation
instruction/14-memory/          allocator, arena, slab allocator, object/buffer pools, fragmentation
instruction/15-parser/          general lexer/parser/AST/visitor theory underneath HTTP and config parsing
instruction/16-kernel/          TCP stack, epoll/io_uring internals, page cache, scheduler, RSS/RPS, XDP/eBPF
instruction/17-performance/     CPU cache, false sharing, NUMA, memory layout, branch prediction, SIMD
instruction/18-distributed/     Raft, gossip, leader election, distributed cache — optional/advanced, beyond a single proxy instance
instruction/19-reading-source/  structured reading of nginx/envoy/haproxy/pingora/hyper/tokio/mio/quinn source
instruction/20-reference/       glossary, cheatsheets
instruction/21-reading-list/    books, RFCs, open-source references
instruction/22-theory/          classical CS theory (deadlock, scheduling, queueing, congestion math, crypto math, Amdahl, CAP/FLP) behind the practical dirs
```

Each topic is one file, named after its concept (e.g. [`instruction/06-proxy/02-load-balancer.md`](instruction/06-proxy/02-load-balancer.md)); when a subtopic grows past roughly 1,500 words or is cross-referenced from several places, split it into its own file and leave a short pointer behind rather than letting one file carry two concepts. Every numbered directory carries a `00-README.md` index listing its files with one-line descriptions and a suggested reading order; where that order is meaningful, the directory's other files carry a matching `NN-` prefix (`01-`, `02-`, ...) so the reading order is visible in a plain directory listing, not just in the README's prose. A few directories deliberately skip the content-file numbering: [`13-algorithms/`](instruction/13-algorithms) is a cross-referenced-as-needed reference set with no single reading order (its own README says so), [`19-reading-source/`](instruction/19-reading-source) holds per-project subfolders (three of which — tokio, hyper, pingora — have a `reading-guide.md`) plus a `contributing-upstream.md`, and [`20-reference/`](instruction/20-reference) currently holds only its `00-README.md`. [`22-theory/`](instruction/22-theory) is the same shape as [`13-algorithms/`](instruction/13-algorithms) — a reference set with no single reading order — but for classical CS theory (deadlock, classical synchronization problems, page replacement, CPU scheduling, congestion-control math, queueing theory, crypto math) that [`01-network/`](instruction/01-network), [`02-linux/`](instruction/02-linux), and [`03-rust/`](instruction/03-rust) teach only at the practical level their dual-track "How to read this directory" sections describe; it exists for readers who want the academic grounding, is never a prerequisite for finishing [`proxy/`](proxy), and its files are pulled in by cross-reference from the practical files rather than read start to finish. There is deliberately no `instruction/10-projects/` — that content now lives directly in each `labs/NN-*` crate's own README (Goal + Practice) and in [`proxy/README.md`](proxy/README.md) for the final build. The directory number encodes prerequisite order for `00`-`12` — earlier numbers are foundational to later ones (e.g. [`instruction/02-linux/14-epoll.md`](instruction/02-linux/14-epoll.md) and [`instruction/03-rust/05-async.md`](instruction/03-rust/05-async.md) underpin [`instruction/04-runtime/01-tokio.md`](instruction/04-runtime/01-tokio.md), which underpins the actual proxy work in [`instruction/06-proxy/`](instruction/06-proxy)).

`13`-`21` are a deep-dive/foundations layer, not a strict continuation of the `00`-`12` sequence — they're referenced *from* earlier directories rather than only read after them (e.g. [`06-proxy/02-load-balancer.md`](instruction/06-proxy/02-load-balancer.md) cross-references [`13-algorithms/`](instruction/13-algorithms) for Maglev/rendezvous hashing). When new content would duplicate an existing topic file's scope (e.g. a load-testing tool, an architecture pattern), add it to the existing directory ([`12-testing/`](instruction/12-testing), [`09-architecture/`](instruction/09-architecture)) instead of creating a new top-level number.

### Dual-track directories: [`01-network/`](instruction/01-network), [`02-linux/`](instruction/02-linux), [`03-rust/`](instruction/03-rust)

These three directories double as a light tutorial, not just a reference
— each one's `00-README.md` has a "How to read this directory" section
(right before `## Files`) offering two explicit tracks: a beginner track
that says "read every file in numeric order, front to back, doing
`## Practice` before moving on" (the directory works as one continuous
tutorial in this mode), and an experienced-reader track that says "treat
this as a reference — skip the Fundamentals-style group and jump straight
to whichever file covers your actual gap." When adding a file to one of
these three directories, update that "How to read this directory" section
if the new file changes which track it belongs to (e.g. a new from-scratch
primer file extends the beginner-only group). Other directories don't get
this treatment — they're reference material pulled in as needed, not a
sequential tutorial, and already say so in their own READMEs.

### Bilingual mirror: [`instruction-vi/`](instruction-vi)

[`instruction-vi/`](instruction-vi) is a Vietnamese mirror of [`instruction/`](instruction) — same numbered
directories, same filenames, same section structure (`## What to learn`,
`## Practice`, subtopic headings), one Vietnamese file per English file at
the identical relative path. It exists so a Vietnamese-speaking reader gets
the same handbook, not a lighter summary.

Rules when adding or editing content:
- [`instruction/`](instruction) (English) is the source of truth. Write or edit the
  English file first; the Vietnamese file always mirrors it, never the
  other way around. Adding a new topic file means adding both
  `instruction/<path>.md` and `instruction-vi/<path>.md` in the same change.
- Translate prose only. Code, shell commands, file paths, crate/
  function/type names, and cross-reference paths (e.g. `` `06-proxy/01-upstream.md` ``)
  stay identical to the English file — a reader following a link from a
  Vietnamese file lands on the Vietnamese file at that same relative path,
  since the directory structure mirrors 1:1. The one exception inside code
  blocks: comments (`//`, `/* */`, `#`) and the prose lines of ` ```text `
  diagrams may be translated. Everything that would change behavior if
  copy-pasted — identifiers, literals, commands, config keys — must not.
- Keep standard CS/Rust/networking terms in English rather than forcing an
  awkward Vietnamese translation — this matches how Vietnamese engineers
  actually write and read technical material. Examples: `thread`, `socket`,
  `buffer`, `cache`, `kernel`, `syscall`, `future`, `async/await`, `poll`,
  `waker`, `borrow checker`, `lifetime`, `ownership`, `trait`, `generic`,
  `closure`, `iterator`, `panic`, `unsafe`, `pointer`, `heap`, `stack`,
  `backpressure`, `load balancer`, `connection pool`, `health check`,
  `circuit breaker`, `rate limiting`, `handshake`, `packet`, `frame`,
  `stream`, `keep-alive`, `mutex`, `channel`, `actor`, `builder`, `newtype`,
  `sealed trait`, `semver`, `FFI`, `ABI`. Translate the connecting
  explanation (the "why," the gotcha, the production consequence) into
  natural Vietnamese; don't translate the jargon noun just because a
  dictionary has a word for it.
- `00-README.md` index files get translated too (one-line descriptions,
  reading order prose) — the index is part of the handbook, not scaffolding.
- `labs/*/README.md`, [`proxy/README.md`](proxy/README.md), the repo-root [`README.md`](README.md), and
  [`CLAUDE.md`](CLAUDE.md) itself are not part of this mirror unless separately asked
  for — the bilingual mirror covers [`instruction/`](instruction) only.

## The Cargo workspace

**[`proxy/`](proxy) (package [`proxy`](proxy)) is the actual deliverable — the single,
complete, production-grade L7 proxy that this entire repo builds toward.**
[`labs/`](labs) is the only other member of the workspace, and exists entirely to
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

[`labs/`](labs) builds up the skills needed for [`proxy/`](proxy) one crate at a time, each
focused on a single mechanism, often deliberately avoiding the high-level
crate that would normally hide it (`00-tcp-server` and `01-http-parser` in
particular reach for raw `libc`/manual parsing precisely to expose what
tokio/hyper otherwise do for you) — none of them are the finished product,
and none should be mistaken for one. Each crate has its own README pointing
back at the relevant handbook file(s) and stating its own "done" criteria
(Goal + Done when checklist, see Content conventions below); there is no
separate `instruction/10-projects/` layer on top of them anymore.

## Content conventions

Topic files (everything under [`instruction/01-network/`](instruction/01-network) through
[`instruction/09-architecture/`](instruction/09-architecture), plus [`instruction/12-testing/`](instruction/12-testing)) follow this
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
- 3-9 concrete, numbered hands-on exercises, at least one pointing at a
  specific `labs/` or `proxy/` crate path.
```

Files with more than 6 exercises should use the ordered-ladder form that
most of [`05-http-stack/`](instruction/05-http-stack) through [`09-architecture/`](instruction/09-architecture) already use: open the
section with "Build these in order.", and end each item with a
**Done when** clause stating an observable pass condition (a measurement,
a test outcome, a behavior under load). Past 9 items, split the topic
rather than growing the ladder.

[`instruction/21-reading-list/`](instruction/21-reading-list) is different: plain annotated lists
(book/RFC/project name + one line on why it's relevant), no `## What to
learn`/`## Practice` sections — keep that format if extending it.

Each `labs/NN-*` crate's own README carries, in this order: `## Goal` (a
few sentences on what the lab builds), `## Done when` (a `- [ ]` checklist
of observable pass conditions — a `curl`/`nc` result, a measurement, a
test outcome, behavior under load — ending with the review step from
[`instruction/00-introduction/03-study-loop.md`](instruction/00-introduction/03-study-loop.md)), `## Handbook references`, an
optional `## After you finish` pointing at the matching
`19-reading-source/*/reading-guide.md` stops, and `## Run`. Checklist items
state *what* must be observably true, never *how* to implement it — they
are a spec, not a solution. There is no separate
`instruction/10-projects/project-0N.md` layer restating any of this; don't
recreate one. [`proxy/README.md`](proxy/README.md) plays the same role for the final build.

**Each lab must be self-sufficient.** The files under a lab's `## Handbook
references` must, together, teach everything needed to meet every
`Done when` item without web searches or mid-lab questions: the wire
format or spec rule itself (not just "per RFC X §Y"), the mechanism of
any library the lab builds on (for example what hyper does with a
service `Err`), and how to install and drive every tool a check names
([`instruction/12-testing/06-lab-environment.md`](instruction/12-testing/06-lab-environment.md) collects these). RFC section
numbers are for checking at the source, never the only place a required
rule appears. When adding or changing a `Done when` item, re-check it
against the referenced files, and if a spec, mechanism or tool is
missing, add it to the handbook in the same change. Name types, methods
and commands and explain their behavior. Never write the lab's logic.
A lab's `Cargo.toml` must already carry every crate its checks need, and
`cargo check --workspace` must still pass.

[`instruction/19-reading-source/`](instruction/19-reading-source)'s `reading-guide.md` files give a route
through a project's source as questions only. Never add answers to them,
and never write the per-project notes files (`architecture.md`,
`request-flow.md`, ...) — those are the learner's own output, and a
pre-written set would defeat the directory's purpose.

When extending any file, keep the `# Title` and any existing intro, follow
the section structure above, and cross-reference other handbook files by
path rather than duplicating their content. Write every cross-reference as
a relative markdown link with the path as inline code, so it is clickable
in any renderer:
`` [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md) ``.
The label is the path as seen from [`instruction/`](instruction) (or [`instruction-vi/`](instruction-vi));
the target is relative to the file doing the linking. Links from a
Vietnamese file point into [`instruction-vi/`](instruction-vi), never across to the English
tree. Only link to files that exist — a reference to something not yet
written stays plain inline code until it does.

When adding a new topic, place it in the most relevant numbered directory
(or propose a new numbered directory for a genuinely new phase, as
[`instruction/12-testing/`](instruction/12-testing) was) and follow the same template.
