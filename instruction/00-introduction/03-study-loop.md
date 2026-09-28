# The Study Loop

How to work through the labs so the handbook actually turns into skill,
how long it realistically takes, and where open-source contribution fits.
Read this once before [`labs/00-tcp-server`](../../labs/00-tcp-server), and again whenever you notice
you've been reading for a week without writing code.

## What to learn

### The loop, once per lab
Every lab follows the same six steps. Skipping any of them is the most
common way self-study stalls.

1. **Read** the handbook files the lab's README lists — only those. Resist reading ahead.
2. **Restate the spec** in your own words: re-read the lab's `## Done when` checklist and write down, before coding, how you'll *demonstrate* each item (a `curl` command, a load test, a test case).
3. **Build** it yourself. No copying from the internet, from pingora, or from an AI. Looking up an API is fine; looking up the design is not.
4. **Prove** it: run your demonstrations from step 2. An item isn't done because the code looks right; it's done because you watched it pass.
5. **Get reviewed.** Share the code with Claude in this repo — [`CLAUDE.md`](../../CLAUDE.md) instructs it to act as a strict reviewer, not to hand you fixes. Fix what it finds, then ask again. This is the substitute for the senior engineer self-study lacks.
6. **Compare with production.** Read the matching part of real source using the guides in [`19-reading-source/`](../19-reading-source), and write down one thing they did that you didn't, and why.

Gotcha: step 5 only works if you ask for review, not solutions. "Why is
this wrong?" and "what edge case am I missing?" produce learning; "write
this for me" produces a lab you can't reproduce next week.

### When you're stuck
Stuck is normal and informative. A useful protocol:
- **30 minutes in:** shrink the problem. Write the smallest program that shows the confusing behavior. Half the time, building the reproduction answers the question.
- **Still stuck:** gather evidence with the tools in [`12-testing/05-debugging.md`](../12-testing/05-debugging.md) — a `tracing` log, a `tcpdump`, an `strace` — instead of re-reading your code.
- **Still stuck:** ask for a *hint* at the concept level ("which handbook section explains why this future isn't `Send`?"), not for the fix.
- **Stuck on the same concept across two labs:** go back a directory. The gap is upstream of the lab.

### A realistic timeline
At about ten hours a week, starting with serious gaps:

| Phase | Covers | Time |
| --- | --- | --- |
| 0 | [`00-introduction/02-prerequisites.md`](02-prerequisites.md): Rust from zero, networking/OS beginner tracks | 3-4 months |
| 1 | `labs/00`-`05`: TCP, parsing, HTTP server, routing, static files, reverse proxy — the core proxy loop | 4-6 months |
| 2 | `labs/06`-`12`: load balancing, TLS, HTTP/2, HTTP/3, cache, rate limiting, WAF | 6-9 months |
| 3 | `labs/13`-`17`, assembling [`proxy/`](../../proxy), [`12-testing/`](../12-testing) load and chaos runs, reading pingora | 4-6 months |

Roughly eighteen months to two years. With a solid Rust background, phase
0 disappears and phase 1 halves. Anyone promising much faster from a
standing start is describing something other than being able to build
this yourself.

### What "as good as nginx" can and can't mean
nginx has two decades of development and deployment behind it; pingora is
years of production use at Cloudflare. No individual matches their feature
breadth or battle-testing in spare time, and that isn't the goal. The
achievable goal is narrower and more valuable to you: a proxy with a
focused feature set (HTTP/1.1 and HTTP/2, TLS, load balancing, health
checks, rate limiting, hot reload, metrics) that is *correct*, resists the
attacks in [`07-security/`](../07-security), benchmarks within the same order of magnitude as
nginx on the same hardware, and whose every design choice you can defend
against how nginx and pingora made the same choice.

### Where open-source contribution fits
"Contribute to anything" is breadth, not a level — senior Rust engineers
specialize too, and compiler work (`rustc`) is a separate track entirely.
The realistic and valuable target is the ecosystem this project stands
on: tokio, hyper, h2, rustls, quinn, pingora. Rough milestones:

- **After phase 1:** you can read issues in tokio and hyper and follow the discussion. Start by reproducing bug reports — confirming or narrowing a bug is a real contribution.
- **After phase 2:** small fixes and documentation improvements in the crates you've used most.
- **After phase 3:** substantive contributions — you now know the problem domain as well as many contributors.

[`19-reading-source/contributing-upstream.md`](../19-reading-source/contributing-upstream.md) covers the mechanics.

## Practice
1. Pick your starting phase honestly using the self-check in [`00-introduction/02-prerequisites.md`](02-prerequisites.md), and write down a target date for finishing it.
2. Start a learning log (a plain markdown file is enough). After each lab, record: what broke, what the review found, and the one thing production source did differently. **Done when** the log has an entry per finished lab.
3. For [`labs/00-tcp-server`](../../labs/00-tcp-server), run the full loop including step 5 review. **Done when** the review finds nothing you disagree with and you've fixed everything else.
4. Every quarter, re-read this file's timeline and adjust it to your actual pace instead of abandoning the plan when it slips.
