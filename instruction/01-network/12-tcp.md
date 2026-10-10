# TCP: Connections, State, and Teardown

What TCP promises, how a connection is born and dies, and the state machine
that governs it. The mechanisms that make the promises true (retransmission,
windows, congestion control) are in [`13-tcp-reliability.md`](13-tcp-reliability.md); the kernel
implementation is in [`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md).

## What to learn

### What TCP promises — and what it doesn't
TCP gives two processes a **reliable, ordered, bidirectional byte stream**
on top of IP's best-effort packets ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)): every byte sent
arrives exactly once, in order, or the connection reports failure. It also
adapts its speed to the receiver (flow control) and to the network
(congestion control).

What it does **not** give you: message boundaries (two `write`s may arrive as
one `read`, [`03-byte-streams.md`](03-byte-streams.md)); any bound on *time* (a lost packet
stalls the whole stream until retransmitted); delivery confirmation to the
*application* (an ACK means the peer's kernel has the bytes, not that the
peer's code has processed them); or security ([`19-tls.md`](19-tls.md)). A successful
`write()` means only "the kernel accepted these bytes into the send buffer."

### The segment header
Every TCP segment carries (20 bytes plus options):

```text
source port | destination port
sequence number         (32 bit: position of the first payload byte)
acknowledgment number   (32 bit: next byte I expect from you)
data offset | flags: SYN ACK FIN RST PSH URG | window size
checksum | urgent pointer
options: MSS, window scale, SACK-permitted, timestamps ...
```

Sequence numbers count **bytes**, not segments. A segment with
`seq=1000` and 500 payload bytes covers bytes 1000..1499; the receiver
acknowledges it with `ack=1500` — "I have everything before 1500, send
1500 next." ACKs are **cumulative**: `ack=1500` confirms everything below
1500 at once. `SYN` and `FIN` each consume one sequence number even though
they carry no data. The **4-tuple** (src IP, src port, dst IP, dst port)
identifies a connection; that is how your proxy's one listening port serves
thousands of clients.

### The three-way handshake, with numbers
```text
client                                   server
  SYN   seq=1000                  ->       (SYN_RECV; entry in SYN queue)
        <-  SYN+ACK seq=5000 ack=1001
  ACK   seq=1001 ack=5001         ->       (ESTABLISHED; moved to accept queue)
```

Each side picks a random **initial sequence number** (ISN) — random so that
stray packets of an old connection, and blind forgers, can't easily land
inside the window. The SYN/SYN-ACK also exchange **options**: the maximum
segment size, whether SACK is supported, the window-scale factor and
timestamps ([`13-tcp-reliability.md`](13-tcp-reliability.md)). They can only be negotiated here.

Server-side, two queues exist ([`11-socket.md`](11-socket.md)): half-open connections
waiting for the final ACK (the **SYN queue**) and completed ones waiting
for your `accept()` (the **accept queue**). A flood of SYNs that never
complete fills the first; a slow accept loop fills the second. **SYN
cookies** let the kernel survive a SYN flood by encoding the state in the
ISN instead of storing it ([`07-security/09-ddos.md`](../07-security/09-ddos.md)).

One round trip passes before the client can send data (more with TLS,
[`19-tls.md`](19-tls.md)): the client's connect completes after SYN-ACK arrives, i.e. 1 RTT
([`04-latency-throughput.md`](04-latency-throughput.md)). That is the cost connection pooling avoids
([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)).

### Teardown: FIN is polite, RST is not
Each direction closes independently. `shutdown(SHUT_WR)` (or `close`) sends
`FIN`: "I have no more to send." The peer ACKs it; the peer may keep
sending (a **half-close**) until it sends its own `FIN`, then the first
side ACKs that. A clean close is four segments, though the middle two are
often combined.

`RST` is an abort: "this connection does not exist / is invalid, forget
it." The receiver drops all buffered data and its next `read`/`write` fails
with `ECONNRESET` (or `EPIPE`). RST is sent when a segment arrives for a
port with no listener (`ECONNREFUSED` for the connecting side), for a
connection that no longer exists, when the app `close()`s a socket that
still has **unread received data** in its buffer, or when it sets
`SO_LINGER` to zero. That third case is a classic proxy bug: close a
client socket before reading its whole request and the client sees a
reset, *losing the response you already sent*. An orderly close drains
first, or `shutdown(SHUT_WR)`s then reads to EOF ([`11-socket.md`](11-socket.md)).

### The state machine
```text
            LISTEN                      CLOSED
              | recv SYN, send SYN+ACK    | connect: send SYN
           SYN_RECV <---------------- SYN_SENT
              | recv ACK                  | recv SYN+ACK, send ACK
              +----------> ESTABLISHED <--+
      active close | send FIN        | recv FIN, send ACK  (passive close)
         FIN_WAIT_1                CLOSE_WAIT
           | recv ACK                 | app calls close(): send FIN
         FIN_WAIT_2                 LAST_ACK
           | recv FIN, send ACK       | recv ACK
         TIME_WAIT --2*MSL-->  CLOSED
```

Read `ss -tan` through this diagram. The states that matter in production:

- **`ESTABLISHED`**: normal.
- **`CLOSE_WAIT`**: the *peer* has closed and sent `FIN`; the kernel is
  waiting for **your code** to notice (a `read` returning 0) and `close()`.
  A growing pile of `CLOSE_WAIT` is **always an application bug** — a
  socket or file-descriptor leak — never a network problem, and it ends
  in `EMFILE` ([`02-linux/20-limits-and-proc.md`](../02-linux/20-limits-and-proc.md)).
- **`FIN_WAIT_2`**: you closed, the peer ACKed but hasn't closed yet.
  Linux times these out (`tcp_fin_timeout`, 60s) so a silent peer can't
  hold it forever.
- **`TIME_WAIT`**: see below.
- **`SYN_RECV`/`SYN_SENT`** in bulk: a SYN flood, or a dialed peer
  that's not answering.

### TIME_WAIT and ephemeral ports
The side that closes first (**active closer**) ends in `TIME_WAIT` for
2×MSL (60s on Linux) so that late duplicate segments from the old
connection can't be mistaken for a new one reusing the same 4-tuple, and
so a lost final ACK can be re-sent. A proxy that dials a fresh upstream
connection per request and closes it first accumulates thousands of
`TIME_WAIT` sockets; each holds a source port, and you have only ~28,000
(`net.ipv4.ip_local_port_range`) per destination IP:port. Hit that and
`connect()` fails with `EADDRNOTAVAIL`. The fix is connection reuse
(pooling, [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)); `tcp_tw_reuse` (client side only)
helps at the margin; `SO_REUSEADDR` ([`11-socket.md`](11-socket.md)) lets a *listening* server
rebind through `TIME_WAIT`. Never "fix" it with `tcp_tw_recycle` — it was
removed because it breaks clients behind NAT.

### Keepalive: detecting a peer that vanished
A TCP connection with no traffic stays `ESTABLISHED` forever, even if the
peer crashed or a NAT/firewall forgot the mapping — no packet, no failure.
TCP keepalive (`SO_KEEPALIVE` with `TCP_KEEPIDLE`/`TCP_KEEPINTVL`/
`TCP_KEEPCNT`) sends probes after idleness and declares the peer dead if
unanswered. Defaults are 2 hours, far longer than most NAT idle timeouts
(minutes), so a proxy must set them. This is distinct from HTTP keep-alive
([`05-http-stack/05-keepalive.md`](../05-http-stack/05-keepalive.md)), which is about *reusing* a connection.

```rust
// socket2 with the `all` feature enabled
use socket2::{SockRef, TcpKeepalive};
let ka = TcpKeepalive::new()
    .with_time(std::time::Duration::from_secs(30))
    .with_interval(std::time::Duration::from_secs(10));
SockRef::from(&stream).set_tcp_keepalive(&ka)?;
```

### Short reads, short writes, and EOF
Because TCP is a stream, `read` may return fewer bytes than the peer wrote
in one call, and `write` may accept fewer than you offered when the send
buffer is full. A `read` returning `0` means the peer sent `FIN` (EOF) — a
*normal* end, not an error; `ECONNRESET` is the abnormal one. Hand-written
code must loop until all bytes are written (`write_all`) and must treat
"read 0" as the cue to finish and `close`. The wire-format consequence is
that a parser has to cope with a message split at any byte boundary
([`16-http1-wire-format.md`](16-http1-wire-format.md)).

## Practice

1. Capture a full connection lifecycle with `tcpdump -i lo -n -S port <p>`
   (`-S` = absolute sequence numbers) while hitting
   [`labs/00-tcp-server`](../../labs/00-tcp-server) with `nc`. On paper, verify the arithmetic:
   `ack` = the peer's `seq` + payload length (+1 for SYN/FIN).
2. Make a CLOSE_WAIT leak on purpose: in a scratch tokio server, `read` a
   connection until it returns 0 but then `std::mem::forget` the stream
   instead of dropping it. Connect with `nc`, press Ctrl-D (which sends FIN),
   and show the server-side socket stuck in `ss -tan state close-wait`;
   then remove the `forget` and watch it disappear.
3. Run `ss -tn state time-wait | wc -l` while hammering
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) with short-lived upstream connections (no
   keep-alive), then again with reuse enabled — compare counts and explain
   which side held the `TIME_WAIT`.
4. Provoke an RST: from a client, send a request, then have the server
   `close()` without reading it (a scratch server with a small sleep), and
   show `ECONNRESET` on the client and the `R` flag in `tcpdump`.
5. Configure TCP keepalive on the upstream client connections in
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) and verify (kill an upstream process without
   closing its socket, e.g. `sudo iptables -A INPUT -p tcp --dport <p> -j
   DROP` on the upstream side) that your proxy eventually detects the dead
   peer; read the probe schedule off `tcpdump`.
6. Shrink `net.ipv4.ip_local_port_range` to a few hundred ports, open and
   close connections in a loop without reuse, and capture the
   `EADDRNOTAVAIL` that results; restore the setting afterward.
