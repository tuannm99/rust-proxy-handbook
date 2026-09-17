# DDoS & Volumetric Mitigation

The layer below `ratelimit.md` and `waf.md`: attacks that try to exhaust connections or bandwidth before any request is even parsed.

## What to learn
### The governing idea: cost asymmetry
Every defense in this file is an answer to one question — *what does this
request cost the attacker, and what does it cost me?* An attack works when
that ratio is lopsided: a SYN costs the attacker one packet and costs you a
socket; a gzip bomb costs them 10 KB of upload and costs you 10 GB of RAM;
a search query costs them a URL and costs you a full table scan.

Read every mitigation below as an attempt to restore symmetry — either by
making rejection cheap (drop at the kernel, reject before parsing) or by
making the expensive path unavailable until the client has proven
something. When you evaluate a defense of your own, price both sides
before deciding it works.

### Where this differs from rate limiting and WAF
`ratelimit.md` and `waf.md` operate on parsed HTTP requests — they assume the connection is already accepted and the proxy is reading bytes off it. A volumetric or connection-exhaustion attack (SYN flood, a flood of legitimate-looking connection attempts, slow-client attacks) tries to win *before* that point, by exhausting file descriptors, memory, or CPU on the accept path itself. Defenses here have to be cheaper per-attempt than an HTTP-layer rate limiter, because you can't afford to fully parse a request just to reject it.

### SYN floods are usually not the proxy's problem to solve
A SYN flood is answered by the kernel's SYN cookie mechanism (`net.ipv4.tcp_syncookies`) or by infrastructure in front of the proxy (a cloud provider's L3/L4 DDoS scrubbing, an anycast edge). A single Rust process cannot out-scale a real volumetric flood — trying to handle it entirely in application code is the wrong layer to fight at. The proxy's job is defense-in-depth for what actually reaches it as an established connection, not replacing a scrubbing layer.

Know the mechanism anyway, because it explains the boundary: SYN cookies
let the kernel stop allocating state for half-open connections by encoding
the connection parameters into the sequence number itself, so the SYN
backlog (`16-kernel/tcp-stack.md`) can't be exhausted. The cost is that
TCP options negotiated in the SYN are partially lost — which is why it's a
fallback triggered under pressure rather than a default.

### Know your actual resource ceilings
"Exhaustion" is a specific number, and you should know yours before an
attacker finds it for you. Per connection, a proxy spends: one file
descriptor (`ulimit -n`, frequently still 1024 by default in a container —
check, don't assume), kernel socket buffers on both send and receive
(`16-kernel/tcp-stack.md`: tens of KB each, and *not* counted in your
process's RSS), your own read/write buffers (`14-memory/buffer-pool.md`),
and a task with its state machine.

At 100k concurrent connections, a modest 64 KB of kernel buffers per
connection is 6.4 GB of kernel memory alone. Do this multiplication for
your own configuration, then set the connection cap below to a number you
have actually proven you can hold — a cap set above your real ceiling is
not a cap.

### Accept-rate limiting: the proxy's actual line of defense
What the proxy *can* police cheaply: how fast it accepts new connections and how many it holds concurrently, independent of any per-client HTTP rate limit. A semaphore around the accept loop caps concurrent connections in flight; a token bucket on `accept()` itself caps the rate of new connections, rejecting (closing immediately) once either limit is hit — far cheaper than reading any bytes from the client first.

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

async fn accept_loop(listener: tokio::net::TcpListener, max_inflight: usize) {
    let permits = Arc::new(Semaphore::new(max_inflight));
    loop {
        let (socket, _addr) = match listener.accept().await {
            Ok(pair) => pair,
            Err(_) => continue,
        };
        let permits = permits.clone();
        match permits.clone().try_acquire_owned() {
            Ok(permit) => {
                tokio::spawn(async move {
                    let _permit = permit; // held until the connection task ends
                    handle_connection(socket).await;
                });
            }
            Err(_) => drop(socket), // over capacity: reject immediately, don't queue
        }
    }
}
```
Gotcha: rejecting by dropping the socket immediately is intentional — queuing rejected connections (or worse, doing any parsing before rejecting) just moves the exhaustion point from file descriptors to whatever resource the queue consumes.

Gotcha: `listener.accept()` returning `Err` deserves more care than
`continue`. `EMFILE`/`ENFILE` (out of file descriptors) is *persistent* —
the next `accept()` fails immediately too, and this loop spins at 100% CPU
producing nothing, turning fd exhaustion into a total CPU stall. Match on
the error kind: back off briefly on `EMFILE`, and consider keeping one
"sacrificial" fd open that you can close to accept-and-immediately-reject
a connection, which is the classic technique for degrading gracefully
instead of spinning.

Gotcha: when you stop accepting, connections queue in the kernel's accept
backlog and then get refused — that is correct behavior, not a bug to
paper over. Pushing backpressure into the kernel is the whole point;
accepting connections you cannot serve just relocates the failure into
your own memory.

### Slow-client attacks (Slowloris) need a data-rate floor, not just a total timeout
An attacker that sends one byte every 10 seconds can hold a connection open indefinitely under a naive "no total timeout" policy. The fix is a *minimum data rate* enforced on the read side — e.g. "must receive at least N bytes within T seconds of connecting, and again after every read" — not just a fixed overall deadline, which a well-paced slow client can dodge. Tie the header-read phase specifically to this: `05-http-stack/parser.md` and `labs/01-http-parser` are where a request actually gets slow-fed one byte at a time.

The family has three members, and defending only the first is common:
- **Slow headers** (classic Slowloris): headers dribbled in forever.
  Defended by a header-phase deadline plus a byte-rate floor.
- **Slow body** (R-U-Dead-Yet): a large `Content-Length` sent at one byte
  per interval. Needs the same floor applied to the body phase, and
  interacts with any body buffering you do for WAF (`waf.md`) or retries
  (`06-proxy/retry.md`) — those buffers are held for the entire slow
  upload.
- **Slow read**: the attacker sends a normal request for a large response,
  then reads the response at one byte per interval, pinning your send
  buffers and any response you've buffered. This one is invisible to every
  request-side timeout you have; it needs a write-side progress deadline.

Gotcha: a byte-rate floor must not break legitimate slow clients — mobile
networks are genuinely slow, and a hard floor set from datacenter testing
will disconnect real users. Set it low enough to be obviously abnormal
(tens of bytes per second), and combine it with a phase deadline rather
than relying on rate alone.

### Layer 7: the expensive-endpoint flood
The most efficient attack is usually not volumetric at all — it's finding
the endpoint where one cheap request costs you the most. A search query
with no index, an endpoint that renders a report, a regex over a large
input (`13-algorithms/regex-engine.md`), an image resize. A few hundred
requests per second — trivially below any sane rate limit — saturate the
upstream while looking like ordinary traffic.

Defenses are per-endpoint rather than global: separate, tighter rate
limits on expensive routes (`ratelimit.md` keyed by route, not just by
client), concurrency caps per route so one endpoint can't consume the
whole upstream pool, and — the structural fix — treating "which endpoints
are expensive" as something you *measure* (`08-observability/metrics.md`
per-route latency and upstream time) rather than guess.

### Decompression bombs
If the proxy accepts `Content-Encoding: gzip` on request bodies and
decompresses them to inspect (`waf.md`) or transform, then a 10 KB upload
can expand to 10 GB. The compression ratio is the attacker's leverage and
it is enormous — this is the single worst cost asymmetry available at
layer 7.

The fix is to bound the *output*, not the input: decompress through a
reader capped at a maximum decompressed size and abort the moment it's
exceeded, rather than decompressing into a `Vec` and checking afterward
(by which point you've already allocated it).

```rust
// cap what you are willing to materialize, before you materialize it
let mut limited = decoder.take(MAX_DECOMPRESSED_BYTES);
let n = limited.read_to_end(&mut buf)?;   // stops at the cap, not at the bomb's size
```
Gotcha: also bound the *ratio*, not only the absolute size. A body that
expands 1000:1 is hostile even if it lands under your cap, and the ratio
is a much better signal than size alone for distinguishing an attack from
a legitimately large upload. See `05-http-stack/compression.md` for the
response-side mirror of this.

### Load shedding beats queueing
When arriving work exceeds capacity, the two options are to queue it or
reject it. Queueing feels kinder and is worse: the queue grows, latency
grows with it, and by the time a request reaches the front, the client has
already timed out and retried (`06-proxy/retry.md`) — so you spend your
remaining capacity computing answers nobody is listening for. That is the
death spiral, and it's self-sustaining once entered.

Shed early instead: reject with 503 *immediately* when a concurrency or
queue-depth limit is hit, and shed cheapest-first (reject before auth,
before WAF, before the upstream call). The useful refinement is to bound
by *time* rather than count — CoDel-style, drop requests that have already
waited longer than a target — since a fixed queue depth is the wrong limit
when latency varies. Google's answer to the same problem is to shed by
request priority, so health checks and paying customers survive while
bulk traffic is dropped first.

Gotcha: a shed response must be cheap and must not retry. Returning 503
with `Retry-After` and ensuring your own retry logic doesn't retry *shed*
responses is what keeps shedding from amplifying the load it exists to
reduce.

### Pushing decisions down the stack
Everything above runs after a TCP (and often TLS) handshake you already
paid for. Once you have *identified* an attacker, the cheap place to drop
them is far lower: an `nftables`/`ipset` entry, or XDP at the driver
(`16-kernel/xdp.md`), where a packet dies before a socket exists.

This is the feedback loop `labs/17-ebpf` builds: the proxy has the
application context to decide who is abusive, the kernel has the position
to drop them for free. Keep the decision in the proxy and the enforcement
as low as you can reach.

## Practice
Build these in order.

1. Compute your ceilings first. **Done when** you have written down, for
   `proxy`'s configuration: `ulimit -n`, kernel socket buffer size per
   connection, your own per-connection buffer allocation, and the resulting
   maximum connections — and have verified the number by actually holding
   that many idle connections open.
2. Add the accept-rate/concurrency limiter, with metrics separate from
   application-level 429s (`ratelimit.md`). **Done when** exceeding the cap
   closes new connections immediately and the rejection count is visible
   as its own metric.
3. Handle `EMFILE` properly in the accept loop. **Done when** lowering
   `ulimit -n` to a small number and flooding connections produces backoff
   and clean rejections rather than a 100%-CPU spin — watch `top` to
   confirm.
4. Add minimum-data-rate enforcement to the header phase, then the body
   phase. **Done when** a client sending one byte every few seconds is
   disconnected in both phases, and a simulated slow-but-legitimate mobile
   client (a few KB/s) is not.
5. Write the three slow-client attackers (slow headers, slow body, slow
   read) as test clients. **Done when** all three are shed, and normal
   traffic load-tested concurrently (`12-testing/load-testing.md`) shows no
   p99 degradation while they run.
6. Add a decompression limit on request bodies with both an absolute cap
   and a ratio cap. **Done when** a gzip bomb is rejected having allocated
   only up to your cap (measure RSS during the test to prove it), and a
   legitimately compressible 50 MB body still works.
7. Add per-route concurrency caps and load shedding with a time-based
   queue bound. **Done when** overloading one expensive route returns 503
   quickly instead of queueing, other routes keep serving normally, and
   your own retry logic does not retry the shed responses.
8. Write down, in `proxy`'s README, which attack classes `proxy` mitigates
   itself vs which require infrastructure in front of it. **Done when**
   the boundary is explicit — this is the actual design decision, not a
   detail to skip.
