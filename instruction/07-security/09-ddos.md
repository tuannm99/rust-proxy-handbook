# DDoS & Volumetric Mitigation

The layer below `07-ratelimit.md` and `06-waf.md`: attacks that try to exhaust connections or bandwidth before any request is even parsed.

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
`07-ratelimit.md` and `06-waf.md` operate on parsed HTTP requests — they assume the connection is already accepted and the proxy is reading bytes off it. A volumetric or connection-exhaustion attack (SYN flood, a flood of legitimate-looking connection attempts, slow-client attacks) tries to win *before* that point, by exhausting file descriptors, memory, or CPU on the accept path itself. Defenses here have to be cheaper per-attempt than an HTTP-layer rate limiter, because you can't afford to fully parse a request just to reject it.

### SYN floods are usually not the proxy's problem to solve
A SYN flood is answered by the kernel's SYN cookie mechanism (`net.ipv4.tcp_syncookies`) or by infrastructure in front of the proxy (a cloud provider's L3/L4 DDoS scrubbing, an anycast edge). A single Rust process cannot out-scale a real volumetric flood — trying to handle it entirely in application code is the wrong layer to fight at. The proxy's job is defense-in-depth for what actually reaches it as an established connection, not replacing a scrubbing layer.

Know the mechanism anyway, because it explains the boundary: SYN cookies
let the kernel stop allocating state for half-open connections by encoding
the connection parameters into the sequence number itself, so the SYN
backlog (`16-kernel/03-tcp-stack.md`) can't be exhausted. The cost is that
TCP options negotiated in the SYN are partially lost — which is why it's a
fallback triggered under pressure rather than a default.

### Know your actual resource ceilings
"Exhaustion" is a specific number, and you should know yours before an
attacker finds it for you. Per connection, a proxy spends: one file
descriptor (`ulimit -n`, frequently still 1024 by default in a container —
check, don't assume), kernel socket buffers on both send and receive
(`16-kernel/03-tcp-stack.md`: tens of KB each, and *not* counted in your
process's RSS), your own read/write buffers (`14-memory/04-buffer-pool.md`),
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

### Slow-client attacks
An attacker that sends one byte every 10 seconds can hold a connection
open indefinitely under a naive "no total timeout" policy, at essentially
zero cost to themselves — the worst cost asymmetry in this file. The
family has three members (slow headers, slow body, slow read), and the
defense is a minimum data *rate* per phase rather than a total deadline a
well-paced attacker simply waits out.

See `07-security/10-slowloris.md` for the three variants and the rate-floor
design.

### Layer 7: the expensive-endpoint flood
The most efficient attack is usually not volumetric at all — it's finding
the endpoint where one cheap request costs you the most. A search query
with no index, an endpoint that renders a report, a regex over a large
input (`13-algorithms/regex-engine.md`), an image resize. A few hundred
requests per second — trivially below any sane rate limit — saturate the
upstream while looking like ordinary traffic.

Defenses are per-endpoint rather than global: separate, tighter rate
limits on expensive routes (`07-ratelimit.md` keyed by route, not just by
client), concurrency caps per route so one endpoint can't consume the
whole upstream pool, and — the structural fix — treating "which endpoints
are expensive" as something you *measure* (`08-observability/02-metrics.md`
per-route latency and upstream time) rather than guess.

### Decompression bombs
If the proxy accepts `Content-Encoding: gzip` on request bodies and
decompresses them to inspect (`06-waf.md`) or transform, then a 10 KB upload
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
a legitimately large upload. See `05-http-stack/06-compression.md` for the
response-side mirror of this.

### Load shedding beats queueing
When arriving work exceeds capacity, queueing the excess turns an overload
into a latency death spiral — by the time a queued request is served, its
client has timed out and retried. Rejecting immediately, cheaply, and as
early in the pipeline as possible is what keeps useful throughput from
collapsing.

See `07-security/11-load-shedding.md` for the shed-vs-queue argument,
time-based bounds, priority shedding, and adaptive concurrency limits.

### Pushing decisions down the stack
Everything above runs after a TCP (and often TLS) handshake you already
paid for. Once you have *identified* an attacker, the cheap place to drop
them is far lower: an `nftables`/`ipset` entry, or XDP at the driver
(`16-kernel/10-xdp.md`), where a packet dies before a socket exists.

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
   application-level 429s (`07-ratelimit.md`). **Done when** exceeding the cap
   closes new connections immediately and the rejection count is visible
   as its own metric.
3. Handle `EMFILE` properly in the accept loop. **Done when** lowering
   `ulimit -n` to a small number and flooding connections produces backoff
   and clean rejections rather than a 100%-CPU spin — watch `top` to
   confirm.
4. Work through `07-security/10-slowloris.md`'s exercises. **Done when** all
   three slow-client variants are shed and a genuinely slow legitimate
   client is not.
5. Add a decompression limit on request bodies with both an absolute cap
   and a ratio cap. **Done when** a gzip bomb is rejected having allocated
   only up to your cap (measure RSS during the test to prove it), and a
   legitimately compressible 50 MB body still works.
6. Work through `07-security/11-load-shedding.md`'s exercises. **Done when**
   overloading one expensive route returns 503 quickly instead of
   queueing, and other routes keep serving normally.
7. Write down, in `proxy`'s README, which attack classes `proxy` mitigates
   itself vs which require infrastructure in front of it. **Done when**
   the boundary is explicit — this is the actual design decision, not a
   detail to skip.
