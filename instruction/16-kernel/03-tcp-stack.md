# The Kernel TCP Stack

`01-network/02-tcp.md` covers TCP from the application's side (the
handshake, byte-stream framing). This file covers what the kernel is
doing underneath — the state machine and buffers a proxy's connection
count and traffic pattern actually stress.

## What to learn

### The state machine, and the two backlogs that matter for a listener
A listening socket has two separate queues, and confusing them is a
common source of "why are we dropping connections under load" bugs:
- **SYN backlog** (half-open connections, in `SYN_RCVD`, waiting for the
  final ACK): sized by `net.ipv4.tcp_max_syn_backlog`. This is what a
  SYN flood exhausts (`07-security/09-ddos.md`).
- **Accept backlog** (fully established connections waiting for your
  process to call `accept()`): sized by the smaller of `listen()`'s
  `backlog` argument and `net.core.somaxconn`. A proxy whose accept loop
  falls behind (blocked doing something else) fills *this* queue instead,
  and new connections are refused (or silently dropped, depending on
  `tcp_abort_on_overflow`) even though the SYN backlog is fine.

### Buffer autotuning and its memory cost at scale
`net.ipv4.tcp_rmem`/`tcp_wmem` let the kernel grow each socket's send/
receive buffer up to a max as throughput demands it — good for a single
high-throughput connection, but multiply that max by connection count: a
proxy holding 100,000 idle-ish connections at even a modest 64 KB buffer
each is 6.4 GB of kernel memory that never shows up in your process's own
RSS accounting (`02-linux/03-memory.md`). Watch `/proc/net/sockstat` and
`ss -m`, not just your process's memory metrics, when diagnosing memory
under high connection counts.

### TIME_WAIT: the cost of being the one who closes first
The side that sends the first `FIN` (active close) ends up holding the
connection's 4-tuple in `TIME_WAIT` for `2 * MSL` (typically ~60s on
Linux) after close, to guard against stray late packets from being
misattributed to a new connection reusing the same tuple. A proxy that
opens and closes short-lived upstream connections per-request accumulates
`TIME_WAIT` entries fast enough to exhaust ephemeral ports or the
connection-tracking table. Mitigations, in order of how much they
actually fix the root cause rather than paper over it: reuse upstream
connections (`06-proxy/01-upstream.md`'s pooling, which avoids opening and
closing at all), let the *upstream* be the one to close first when a
choice exists, and only reach for `SO_REUSEADDR`/tuning
`net.ipv4.tcp_tw_reuse` as a last resort for the client-facing side.

### Gotcha: RST vs FIN, and what your application actually sees
An abrupt `RST` (a connection reset, from a peer crash, a firewall, or an
explicit `close()` on data still in the receive buffer) surfaces to a
Rust async read/write as an `io::Error` with `ErrorKind::ConnectionReset`,
distinct from a clean EOF from an orderly `FIN`-based close. Code that
treats every connection-ending error the same way loses the ability to
distinguish "the client hung up normally" from "something is actively
resetting our connections," which matters for both debugging and
detecting certain classes of scanning/attack traffic.

## Practice
1. Watch `ss -tan state time-wait | wc -l` while driving load at
   `labs/05-reverse-proxy` opening a fresh upstream connection per
   request; then add connection pooling (`06-proxy/01-upstream.md`) and
   compare the count.
2. Deliberately make an accept loop slow (sleep before each `accept()`)
   and overflow the accept backlog with a burst of connections; observe
   the difference between that and overflowing the SYN backlog with a
   half-open-connection flood (e.g. via a raw SYN, in a lab environment
   you control).
3. Watch `/proc/net/sockstat` and `ss -m` while holding 10,000+ idle
   connections open against `labs/00-tcp-server`; compare the kernel-side
   buffer memory against what your process's own RSS reports.
4. Trigger both a clean close and a `RST` against a connection your code
   holds open, and confirm you can distinguish the two `io::Error` kinds
   in your handling code.
