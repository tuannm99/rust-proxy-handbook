# Contributing Upstream

How to go from reading tokio, hyper, rustls, quinn, or pingora to
contributing to them. This is the last step of the loop in
[`00-introduction/03-study-loop.md`](../00-introduction/03-study-loop.md): building your own proxy is what earns you enough
context to be useful to these projects, and contributing is what makes
that knowledge durable.

## What to learn

### Pick the ecosystem you already know
Contribute where you have context. After this handbook, that means the
crates your proxy is built on — tokio, hyper, h2, rustls, quinn, pingora,
and their smaller neighbors (`httparse`, `bytes`, `tokio-rustls`,
`hyper-util`). You've already hit their APIs, their error messages, and
probably their rough edges. Contributing to an unrelated crate you've
never used means learning its domain from scratch at the same time.

### Contributions that don't start with code
The fastest way to become useful is often not a pull request:
- **Reproduce bug reports.** Many issues sit unconfirmed. A minimal reproduction — a small program that triggers the bug reliably, with versions noted — is one of the most valued contributions a maintainer can receive.
- **Narrow a bug down.** "It happens with HTTP/2 but not HTTP/1.1, only when the body exceeds the initial window" turns a vague report into a fixable one.
- **Fix documentation you found confusing.** You are the ideal author: you just experienced the confusion.
- **Answer questions** in the project's discussions from your own recent experience.

### Finding a first code change
Look for issues labeled `good first issue`, `E-easy`, `help wanted`, or
`A-docs` in the project's tracker. Before writing code: comment that you'd
like to take it and outline your intended approach. Maintainers often
know a constraint you don't, and a two-line reply saving you a week of
work in the wrong direction is common.

Gotcha: a first PR that also "cleans up" unrelated code in the same files
is much harder to review and much more likely to stall. Keep the change to
exactly the issue; propose other cleanups separately.

### What a mergeable PR looks like in these projects
- **A test that fails before your change and passes after.** For a bug fix this is non-negotiable in tokio/hyper-class projects.
- **CI passing locally first:** `cargo fmt`, `cargo clippy --all-targets`, the full test suite, and — in crates with `unsafe` — Miri. tokio also tests concurrency with `loom` (a model checker that explores thread interleavings); if you touch synchronization code, expect to need a loom test.
- **A description that explains why**, links the issue, and states what you tested.
- **Patience with review.** Maintainers are volunteers or have other priorities; a week's silence isn't rejection. Respond to review comments by changing the code or explaining your reasoning, not by defending it.

### Reading a large codebase quickly
The reading guides in this directory ([`tokio/reading-guide.md`](tokio/reading-guide.md),
[`hyper/reading-guide.md`](hyper/reading-guide.md), [`pingora/reading-guide.md`](pingora/reading-guide.md)) apply one method you can reuse
anywhere: start from a public API you've called, follow it inward one
layer at a time, and keep a running list of questions. Two tools make this
far faster: an editor with rust-analyzer's "go to definition" and "find
references," and the repository's own tests — a test is a runnable example
of exactly how the authors expect a piece to be used.

### Design discussions and RFCs
Larger changes in these projects begin as an issue or design document,
not a PR. Writing one well is a skill in itself: state the problem with a
concrete use case, list the alternatives you considered and why you
rejected them, and call out what breaks. This is the same reasoning every
"why this design" section in this handbook asks of you — the habit
transfers directly.

## Practice
1. Pick one crate your proxy depends on and read its `CONTRIBUTING.md`. **Done when** you can run its full test suite and clippy locally with no failures.
2. Find one open, unconfirmed bug report in that crate and try to reproduce it. **Done when** you've posted a comment with a minimal reproduction, or with a note that it doesn't reproduce on current versions and what you tried.
3. Find one piece of documentation in that crate that confused you while building a lab, and open a PR improving it.
4. Take one `good first issue`, comment with your approach before coding, and carry it through review. **Done when** it's merged — or closed with an explanation you understand.
5. After finishing [`proxy/`](../../proxy), write a short design note on one thing you'd change about a pingora or hyper API, using the problem/alternatives/trade-offs structure above. You don't need to submit it; writing it is the exercise.
