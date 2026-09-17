# Slow-Client Attacks

The cheapest denial of service available: hold connections open by being
slow, not by being loud. `07-security/ddos.md` covers the volumetric and
connection-flood side; this file covers the family of attacks that spend
almost no attacker bandwidth at all.

## What to learn
### Why slowness is an attack
Every open connection costs the proxy a file descriptor, kernel socket
buffers, and per-connection state (`07-security/ddos.md`'s resource
ceiling arithmetic). An attacker who opens connections and keeps them
*technically alive* — sending just enough to avoid any timeout — consumes
those resources at essentially zero cost to themselves. A few thousand
connections from one host can exhaust a proxy that would shrug off a
million requests per second.

The cost asymmetry is the worst of any attack in this section: one
attacker socket sending one byte per minute versus one of your
connections, indefinitely.

### The three variants, and why defending one isn't enough
- **Slow headers (classic Slowloris).** The attacker opens a connection
  and dribbles request headers a byte at a time, never sending the blank
  line that ends them. The request never completes, so a "request
  timeout" measured from request *completion* never starts.
- **Slow body (R-U-Dead-Yet).** Headers complete normally and declare a
  large `Content-Length`; the body then arrives one byte per interval. A
  header-phase deadline doesn't apply anymore — the headers were fine.
  This also pins any body buffering you do for WAF inspection
  (`07-security/waf.md`) or retry replay (`06-proxy/retry.md`) for the
  entire slow upload.
- **Slow read.** The attacker sends a perfectly normal request for a large
  response, then reads the response at one byte per interval — pinning
  your send buffers and any response you buffered. This one is invisible
  to every *request*-side timeout you have, because the request was
  flawless.

Defending only the first is the common state of affairs, and it's why
these still work.

### A deadline is not enough; you need a rate floor
The obvious fix is a total timeout on the request. A well-paced attacker
dodges it by simply finishing just under it, then opening the next
connection. And raising the total timeout is exactly backwards: it makes
the attack cheaper.

What actually characterizes the attack is **throughput**, not duration. So
enforce a minimum data rate on each phase — "at least N bytes must arrive
within T seconds, and again after every read" — alongside a phase
deadline. A client transferring meaningful data continues; a client
holding a socket open with a trickle is disconnected regardless of how
patiently it waits.

```rust
// conceptually, per phase: bytes must keep arriving fast enough
// to be plausibly a real client, not just fast enough to dodge a deadline
struct RateFloor {
    min_bytes: usize,
    window: std::time::Duration,
}
```

Gotcha: a rate floor calibrated in a datacenter will disconnect real
users. Mobile networks, congested links, and clients on the other side of
the world are genuinely slow. Set the floor low enough to be obviously
abnormal (tens of bytes per second, not kilobytes), and rely on the phase
deadline to catch the rest — the two together are far more discriminating
than either alone.

Gotcha: apply the floor to the *write* side too, or slow read stays
unaddressed. That means a progress deadline on response writes: if the
socket hasn't accepted any bytes in T seconds despite having data pending,
the client isn't reading.

### Where these limits live
This is a connection-lifecycle concern, not a request-handling one, so it
belongs in the connection manager (`09-architecture/components.md`) — it
must apply before and independently of anything that assumes a complete
request exists. `05-http-stack/parser.md` and `labs/01-http-parser` are
where a request actually gets fed a byte at a time, and where the header
phase's floor has to be enforced.

Gotcha: exempt upgraded and streaming connections
(`05-http-stack/websocket.md`, `05-http-stack/grpc.md`) from the
request-shaped deadlines but *not* from liveness checking — an idle
WebSocket is legitimate, an unresponsive one is not, which is what
ping/pong deadlines are for.

### HTTP/2 has its own version
Multiplexing changes the shape but not the principle. An attacker can open
many streams on one connection and leave them incomplete, or manipulate
flow-control windows to make the server hold data it cannot send. HTTP/2's
`SETTINGS_MAX_CONCURRENT_STREAMS` bounds the first; per-connection memory
accounting bounds the second. See `01-network/http2.md`, which also covers
Rapid Reset — the inverse attack, where streams are opened and cancelled
as fast as possible.

## Practice
Build these in order.

1. Write the three attackers as test clients against `proxy`: slow
   headers, slow body, slow read. **Done when** all three can hold a
   connection open for minutes against your current configuration — you
   need the working attack before the defense means anything.
2. Add a header-phase deadline plus a byte-rate floor. **Done when** the
   slow-headers client is disconnected within the deadline and a normal
   client is unaffected.
3. Extend the floor to the body phase. **Done when** the slow-body client
   is disconnected, and a legitimately large upload at a few hundred KB/s
   completes.
4. Add a write-progress deadline. **Done when** the slow-read client is
   disconnected and a large streaming download to a normal client still
   works.
5. Verify against a simulated slow-but-legitimate client (throttle to a
   few KB/s with `tc` or a proxy that rate-limits). **Done when** it is
   *not* disconnected — if it is, your floor is set from datacenter
   assumptions.
6. Run all three attacks concurrently with a normal load test
   (`12-testing/load-testing.md`). **Done when** legitimate p99 latency is
   unchanged and connection count stays bounded.
7. Confirm exemptions. **Done when** an idle WebSocket survives
   indefinitely while an unresponsive one (no pong) is closed.
