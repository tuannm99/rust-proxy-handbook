# TCP Reliability: Retransmission, Flow Control, Congestion Control

How TCP turns a lossy packet network into a dependable stream — and the
knobs and failure modes a proxy operator actually meets. Prerequisite:
[`12-tcp.md`](12-tcp.md). The mathematics behind congestion control is in
[`22-theory/05-congestion-control-math.md`](../22-theory/05-congestion-control-math.md); this file is the mechanism and the
operational consequences.

## What to learn

### Three separate problems, three separate mechanisms
TCP solves, independently: **loss** (a packet disappeared — retransmit it),
**the receiver being slower than the sender** (flow control: the receive
window) and **the network being slower than the sender** (congestion
control: the congestion window). A sender may have in flight at most
`min(receive window, congestion window)` unacknowledged bytes. Conflating
the two windows is the most common misunderstanding of TCP; keep them apart.

### Retransmission: timers and duplicate ACKs
The sender keeps every unacknowledged segment. Two triggers resend it:

- **Retransmission timeout (RTO).** The sender measures round-trip time
  per ACK (smoothed RTT plus variance) and sets RTO a bit above it
  (floor 200 ms on Linux). If no ACK arrives in time it resends and
  **doubles** the RTO (exponential backoff), up to ~15 retries
  (`tcp_retries2`, on the order of 15 minutes) before failing the
  connection with `ETIMEDOUT`. A lost packet with nothing behind it (the
  tail of a transfer) can cost a whole RTO — hundreds of milliseconds of
  dead air.
- **Fast retransmit.** If segment N is lost but N+1, N+2, N+3 arrive, the
  receiver keeps ACKing "still need N" (**duplicate ACKs**). Three
  duplicates tell the sender N is lost, not merely late, and it resends
  at once without waiting for the timer.

**SACK** (selective acknowledgment, negotiated in the SYN) lets the
receiver say "I have bytes 5000–8000 and 9000–12000, the hole is 8000–9000,"
so the sender resends only the hole instead of everything after it. On
Linux, SACK and **timestamps** (which give accurate RTT samples and guard
against old duplicates) are on by default.

```text
$ ss -ti dst 10.0.0.7
... rto:204 rtt:3.2/1.1 mss:1448 cwnd:10 ssthresh:7 bytes_retrans:2896 retrans:0/2
```

`retrans` and `bytes_retrans` are your first loss indicator, per
connection, without needing a packet capture.

### Head-of-line blocking
TCP delivers bytes **in order**. If segment N is lost, bytes after N that
already arrived sit in the kernel's receive buffer, **withheld from your
application** until N is retransmitted. Every logical stream multiplexed
on that connection stalls together. This is the problem HTTP/2 inherits
(many streams, one TCP connection, [`17-http2.md`](17-http2.md)) and the reason HTTP/3 moves
to QUIC, where each stream recovers independently ([`18-http3.md`](18-http3.md)).

### Flow control: the receive window
Every ACK carries the **window**: how many more bytes the receiver has
buffer space for. The receiver's kernel buffer fills when your application
doesn't `read` fast enough; the advertised window shrinks; at **zero
window** the sender stops entirely and sends periodic **zero-window
probes**. This is end-to-end **backpressure**: a slow client slows the
proxy's *upstream* read only if your code stops reading from the upstream
while the client-side write would block ([`04-runtime/`](../04-runtime)). Buffer
unboundedly instead, and the pressure is absorbed in your process's
memory — the classic way one slow client takes down a proxy.

The 16-bit window field caps at 64 KB, far too small for fast, long
links. **Window scaling** (negotiated in the SYN, a shift of up to 14 bits)
lifts the cap to ~1 GB. The **bandwidth-delay product**
([`04-latency-throughput.md`](04-latency-throughput.md)) is how big the window needs to be to fill a path:
a 1 Gbit/s link at 100 ms RTT needs ~12.5 MB in flight. Linux autotunes
buffers up to the `tcp_rmem`/`tcp_wmem` maximums; setting `SO_RCVBUF`/
`SO_SNDBUF` explicitly *disables* autotuning for that socket, usually a
mistake.

### Congestion control: probing for the capacity
The sender maintains a **congestion window** (`cwnd`, in segments),
discovering the network's capacity by experiment:

1. **Slow start**: begin at the initial window (10 segments on modern
   Linux, ~14 KB) and **double `cwnd` every RTT** (each ACK adds one
   segment) until loss, or until `ssthresh`.
2. **Congestion avoidance**: past `ssthresh`, grow by ~1 segment per RTT.
3. **On loss**, cut: classic Reno halves `cwnd`. This is
   **AIMD** (additive increase, multiplicative decrease), which makes
   competing flows converge to a fair share.

Linux defaults to **Cubic** (growth is a cubic function of time since the
last loss, better on high-BDP links); **BBR** instead *models*
bandwidth and RTT and doesn't treat loss as the main signal, doing better
on lossy or bufferbloated paths. `sysctl net.ipv4.tcp_congestion_control`
shows and sets it.

Why a proxy cares:

- A **new connection starts small.** A fresh upstream connection sends only
  ~14 KB in its first RTT, however fast the link — a large response takes
  several RTTs to ramp up. This is throughput's argument for connection
  reuse, beside latency's ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)).
- **Slow-start after idle.** By default Linux collapses `cwnd` after an idle
  period (`net.ipv4.tcp_slow_start_after_idle=1`), so a pooled connection
  that sat quiet for a few seconds *re-ramps*; high-throughput proxies
  usually disable it.
- **Bufferbloat.** Oversized queues in routers hold seconds of data; loss-based
  algorithms fill them and add huge latency without any loss. Seeing RTT
  climb under load while throughput is flat is the signature.

### Nagle's algorithm, delayed ACKs, and TCP_NODELAY
Two sensible optimizations that interact badly. **Nagle**: if there is
unacknowledged data in flight, hold small writes and combine them until the
ACK arrives (or a full segment accumulates) — fewer tiny packets.
**Delayed ACK**: the receiver waits (up to ~40 ms) hoping to piggyback its
ACK on a reply, or to ACK two segments at once. Combine them: a client
writes a request in two small pieces; the second waits for an ACK for the
first, which the server is delaying because it's waiting for the rest of the
request. Result: a stall of ~40 ms on every request.

`TCP_NODELAY` disables Nagle; proxies and RPC servers set it, since they
already frame their writes. The opposite tool is `TCP_CORK`/`MSG_MORE` —
"hold until I say so" — to assemble headers plus a `sendfile` body into
full packets ([`02-linux/18-zerocopy.md`](../02-linux/18-zerocopy.md)). The better fix is still *one* `write`
(or one vectored write) per message rather than many small ones.

```rust
stream.set_nodelay(true)?;
```

### Gotcha: the sysctl graveyard
The internet is full of "TCP tuning" sysctl lists copied between servers.
Most are obsolete or harmful (`tcp_tw_recycle`, `tcp_sack=0`, a huge
`tcp_rmem` max on a box with thousands of connections — memory per
connection multiplies). Tune only against a measurement: `ss -ti` for
`retrans`/`cwnd`/`rtt`, `nstat -az | grep -i retrans` for system-wide
counters, a before/after load test ([`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)). Kernel
mechanisms (autotuning, queues) are in [`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md).

## Practice

1. Add delay to loopback: `sudo tc qdisc add dev lo root netem delay 50ms`
   (50 ms each way, so ~100 ms RTT). Fetch a 1 MB response from a local
   server with `curl -sS -o /dev/null -w '%{time_total}\n'` and note the
   time; with `ss -ti` during the transfer, watch `cwnd` grow. Remove it with
   `sudo tc qdisc del dev lo root`.
2. Add loss: `sudo tc qdisc add dev lo root netem loss 2%`. Repeat the
   transfer; observe `retrans` in `ss -ti`, and find a fast retransmit in
   `tcpdump -n -S` (the same `seq` sent twice after three duplicate
   ACKs). Compare transfer time with 0% loss.
3. Read `wscale` and `sackOK` out of a SYN in `tcpdump -n`, compute the
   maximum window it allows (65535 << wscale), then compute the BDP of a
   1 Gbit/s, 50 ms path and compare it to the `net.ipv4.tcp_rmem` maximum.
4. Reproduce the Nagle/delayed-ACK stall: write a scratch client that sends
   `"GET /"` and `" HTTP/1.1\r\n\r\n"` as two separate `write`s to a server
   that replies after the complete request, with and without
   `set_nodelay(true)`, and time the round trip; fix it a second way by
   assembling the request into a single write.
5. Create backpressure on purpose: make a client that connects to
   [`labs/00-tcp-server`](../../labs/00-tcp-server) and sends continuously, while the server-side handler
   sleeps before each read; show the window falling to 0 in `tcpdump -n`
   (`win 0`) and the client's `write` blocking, then confirm the server
   stays at constant memory.
6. Compare Cubic vs BBR (`sudo sysctl -w net.ipv4.tcp_congestion_control=bbr`
   if `modprobe tcp_bbr` works on your kernel) on a delayed, lossy `netem`
   loopback; record throughput for each and explain the difference in
   terms of what each uses as its congestion signal.
