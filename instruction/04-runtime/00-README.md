# Async Runtime

Phase 4. How tokio turns `02-linux/01-epoll.md`'s readiness notifications and
`03-rust/05-async.md`'s state machines into a working scheduler — and what
that means for code you write on top of it.

## Files

- `01-tokio.md` — the multi-threaded scheduler, work stealing, `spawn`, blocking-pool offload
- `02-waker.md` — `Poll::Pending`, wakers, what `.await` actually registers

## Where it goes next

Everything from `05-http-stack/` onward runs on this. The two failure
modes to carry forward: blocking a worker thread stalls every connection
multiplexed on it (see `08-observability/04-profiling.md` and
`tokio-console` for finding it), and a dropped future is a cancelled
operation, which `06-proxy/01-upstream.md` turns into a concrete bug.
