# Cache Stampede

The failure where a cache, at the exact moment it stops helping, actively
makes things worse. `05-http-stack/07-cache.md` covers HTTP caching
semantics; this file covers the concurrency problem underneath any cache,
and why single-flight is a requirement rather than an optimization.

## What to learn
### The shape of the failure
A popular entry expires. In the next millisecond, 1,000 concurrent
requests all find a miss, and all 1,000 go to the origin — which was the
exact thing the cache existed to prevent, delivered at the worst possible
moment.

It compounds: the origin slows under that load, so the fetch takes longer,
so the miss window stays open longer, so more requests pile in. The more
popular the key, the worse the spike — so the entries your cache is
protecting best are the ones whose expiry hurts most.

Gotcha: this is not only an expiry problem. A cold start
(`09-architecture/05-rolling-restart.md` — every restart empties an in-memory
cache), a purge, or an eviction under memory pressure all produce the same
simultaneous-miss condition, for every key at once rather than one.

### Single-flight: one fetch per key
The fix is request coalescing: the first request for a key becomes the one
that fetches; everyone else waits on its result.

```rust
// one in-flight fetch per key; the rest await the same shared future
enum Entry {
    Ready(CachedResponse),
    InFlight(tokio::sync::broadcast::Sender<CachedResponse>),
}
```

The check-and-insert must be atomic against other requests for the same
key — two requests that both see "no entry" and both start a fetch have
defeated the mechanism. With a sharded map (`dashmap`), that means holding
the shard guard across the transition from absent to `InFlight`, not
checking and then inserting.

nginx spells this `proxy_cache_lock`; Go has `singleflight`; the pattern
is old and the absence of it is a recurring incident.

Gotcha: waiters need a **timeout**. A hung origin fetch must not park a
thousand requests indefinitely — each waiter should give up on its own
deadline (`06-proxy/01-upstream.md`'s request timeouts) and decide for itself
whether to fail or attempt its own fetch.

Gotcha: on fetch **failure**, every waiter must be woken with the error.
A leader that returns early — a panic, an unhandled branch, a dropped
future on client cancellation — leaves waiters blocked on a result that
will never arrive. Structure the leader so the notification happens in a
`Drop` guard, the same discipline as `06-proxy/01-upstream.md`'s connection
counter.

Gotcha: the in-flight map is keyed by attacker-influenceable data, so it
needs the same bound as any other such map
(`13-algorithms/count-min-sketch.md`).

### Serving stale while you refresh
Single-flight reduces N fetches to one, but the waiters still wait. With
`stale-while-revalidate` (`05-http-stack/07-cache.md`) the waiting disappears
entirely: serve the stale copy immediately to everyone, refresh once in
the background, swap it in when it arrives.

This is the strictly better combination, and it changes the failure mode
from "1,000 requests wait on the origin" to "1,000 requests get a slightly
old answer." Only requests for a key with *no* cached value at all —
genuinely cold, not merely stale — have to wait.

Gotcha: a background refresh that nobody is waiting for still needs a
timeout, a failure path, and a bound on how many can run at once. Otherwise
an origin outage leaves a growing pile of refresh tasks, each holding a
connection (`06-proxy/01-upstream.md`), all of them futile.

### Spreading expiry so it doesn't synchronize
Keys populated together expire together. Fill a cache from a cold start
and everything you loaded in the first second expires in the same second,
an hour later — a self-inflicted, recurring stampede with a period equal
to your TTL.

Add jitter to the TTL when storing (±10% is plenty) so expiries spread
across a window instead of landing in lockstep. This is the same
synchronization problem as probe intervals (`06-proxy/03-healthcheck.md`) and
retry storms (`06-proxy/05-retry.md`), with the same fix.

### Negative caching
An origin that returns 404 or 500 for a hot key, uncached, gets every
request for it forever — the stampede without even an expiry to trigger
it. Cache negative responses too, briefly (seconds), so a hot miss costs
one origin request per interval rather than all of them.

Gotcha: keep negative TTLs short and distinct from positive ones. A
5-minute cached 404 for a resource that was just created is a
user-visible bug, and the asymmetry is deliberate: being wrong about
absence is cheaper to fix than being wrong about content.

## Practice
Build these in order.

1. In `labs/10-cache`, reproduce the stampede: cache a deliberately slow
   origin response, expire it, and fire 500 concurrent requests. **Done
   when** you can show ~500 origin hits with an origin-side counter.
2. Add single-flight coalescing with an atomic check-and-insert. **Done
   when** the same test produces exactly one origin hit and all 500
   clients get a correct response.
3. Add waiter timeouts and error propagation. **Done when** an origin that
   hangs forever leaves waiters failing on their own deadlines rather than
   blocking indefinitely, and an origin that errors wakes every waiter
   with the error.
4. Kill the leader mid-fetch (cancel its future, simulating a client
   disconnect). **Done when** the waiters still get a result or a clean
   error, rather than hanging — write it without the `Drop` guard first
   and watch them hang.
5. Add `stale-while-revalidate` on top. **Done when** the same 500-request
   test returns immediately from stale cache with one background refresh,
   and only a genuinely cold key makes anyone wait.
6. Bound concurrent background refreshes. **Done when** an origin outage
   produces a capped number of in-flight refresh tasks rather than a
   growing pile.
7. Add TTL jitter. **Done when** a cache populated in one burst shows
   expiries spread across a window rather than a spike — chart origin
   request rate over a full TTL period to see it.
8. Add short negative caching. **Done when** a hot 404 costs one origin
   request per negative-TTL interval, and a resource created immediately
   after a cached 404 becomes visible within seconds.
