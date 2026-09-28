# Prerequisites: Phase 0

What you need *before* [`labs/00-tcp-server`](../../labs/00-tcp-server), and how to get it if you
don't have it. The handbook teaches systems, networking, and async Rust
from a practical starting point, but it deliberately does not teach Rust
syntax — [`03-rust/`](../03-rust) says so in its first paragraph. If you are starting
with serious gaps in Rust, networking, or operating systems, this file is
where you start, and it is fine to spend three or four months here.

## What to learn

### Self-check: are you ready for lab 00?
Answer these without looking anything up. Each "no" points at the section
below that fixes it.

**Rust:**
- Can you write a `struct` and an `enum` with data, and `match` on the enum?
- Can you write a function returning `Result<T, E>` and use `?` inside it?
- Given a borrow-checker error ("cannot borrow `x` as mutable because it is also borrowed as immutable"), can you explain *why* the compiler refused, not just make it go away with `.clone()`?
- Can you use `Vec`, `HashMap`, `String` vs `&str`, iterators with `.map()`/`.filter()`, and closures?
- Have you written a trait and implemented it for two types?

**Networking:**
- Can you describe, step by step, what happens between typing `curl http://example.com` and seeing HTML — DNS lookup, TCP handshake, HTTP request, response?
- Do you know what a port is, and why two programs can't both listen on port 8080?

**Operating systems:**
- Can you explain the difference between a process and a thread?
- Do you know what a system call and a file descriptor are, at least roughly?

If every Rust answer is yes, skip to lab 00 and use the beginner tracks of
[`01-network/`](../01-network) and [`02-linux/`](../02-linux) alongside it. If most are no, do the rest of
this file first.

### Rust from zero: the minimum path
Work through *The Rust Programming Language* ("the Book", free at
doc.rust-lang.org/book) in order through the chapters on generics, traits,
lifetimes, closures, iterators, smart pointers, and fearless concurrency.
Type every example yourself — reading Rust without compiling it teaches
you almost nothing, because the compiler's error messages are half the
curriculum.

Run **Rustlings** (github.com/rust-lang/rustlings) in parallel: small
exercises that don't compile until you fix them. It is the fastest way to
turn "I read about ownership" into "I can satisfy the borrow checker."

Then read two chapters that bridge directly into this handbook:
- The Book's async chapter — the vocabulary [`03-rust/05-async.md`](../03-rust/05-async.md) builds on.
- The Book's final project, "Building a Multithreaded Web Server" — a TCP
  server with a hand-rolled thread pool, which is precisely the design
  [`labs/00-tcp-server`](../../labs/00-tcp-server) replaces with tokio. Building the thread-pool version
  first makes it obvious *why* async exists.

Finally, the official **Tokio tutorial** (tokio.rs/tokio/tutorial), which
builds a mini-Redis step by step. It covers `spawn`, shared state, channels,
and framing — the exact toolkit of labs 00-05.

Gotcha: do not start [`03-rust/`](../03-rust) or [`04-runtime/`](../04-runtime) before the Book's traits and
lifetimes chapters. Those directories explain *why* `Pin` and `Send` bounds
exist; without the basics, every sentence reads as noise and you will
conclude, wrongly, that you are not capable of this.

### Networking and OS: use the beginner tracks here
You don't need an external course. [`01-network/`](../01-network) and [`02-linux/`](../02-linux) each have a
"How to read this directory" section with a beginner track: read every
file in order, starting with `01-fundamentals.md`, and do its Practice
before moving on. Those two fundamentals groups were written as
from-scratch primers for exactly this situation.

If you want a second voice alongside them, two free resources match their
depth: *Beej's Guide to Network Programming* (sockets in C, the layer
tokio hides) and *Operating Systems: Three Easy Pieces* (the virtualization
and concurrency parts). Both are listed in [`21-reading-list/`](../21-reading-list).

### Readiness gates
Don't move past a gate until you pass it. These are checks, not reading
assignments.

| Before | You can |
| --- | --- |
| [`labs/00-tcp-server`](../../labs/00-tcp-server) | Finish the Book's multithreaded web server project from memory; explain every borrow error you hit |
| [`labs/01-http-parser`](../../labs/01-http-parser) | Rewrite lab 00 from scratch without looking at your old code; explain what each `.await` is waiting for |
| [`labs/02-http-server`](../../labs/02-http-server) | Explain why a TCP read can return half an HTTP request, and how your parser handled it |
| [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) | Explain `Arc<Mutex<T>>` vs a channel for shared state, and why a future must be `Send` to be spawned |

### Time budget
At roughly ten hours a week: the Book plus Rustlings takes two to three
months for someone starting from zero; the beginner tracks of
[`01-network/`](../01-network) and [`02-linux/`](../02-linux) take another month or so and can overlap.
[`00-introduction/03-study-loop.md`](03-study-loop.md) has the rest of the timeline.

## Practice
1. Answer the self-check above in writing, honestly. Keep the answers — you will re-take it after phase 0 and compare.
2. Complete the Book's multithreaded web server project. **Done when** it serves concurrent requests through your own thread pool and shuts down cleanly on drop.
3. Finish Rustlings. **Done when** every exercise passes and you could explain the fix for any ten of them to someone else.
4. Complete the Tokio tutorial's mini-Redis. **Done when** two clients can set and get keys concurrently against your server.
5. Read [`01-network/01-fundamentals.md`](../01-network/01-fundamentals.md) and [`02-linux/01-fundamentals.md`](../02-linux/01-fundamentals.md) and do their Practice sections, then re-take the self-check. **Done when** every answer is yes.
