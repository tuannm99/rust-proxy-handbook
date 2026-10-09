# Recall and Review: Making the Networking Stick

Reading a file once produces the feeling of understanding, not the memory of
it — that is why you learn something on Monday and it is gone by Wednesday.
Memory is built by **retrieval**: closing the page and pulling the answer out
of your own head, then checking. This file is the retrieval kit for
`01-network/`: a skeleton to hang details on, questions per file, drawings to
reproduce from memory, and predict-then-run experiments. It teaches nothing
new; it makes you use what the other files taught.

## What to learn

### How to use this file (the method)
1. **Cover, answer, check.** Read a question, answer *out loud or in writing*
   without looking, then open the linked section and compare. A wrong answer
   found now is worth more than a right answer you only recognized.
2. **Never re-read first.** If you can't answer, don't skim the file again —
   first try to *derive* it from the layer map below ("what problem is this layer
   solving? what would break without it?"), then check. Derivation is what
   survives.
3. **Space it.** Review a file's questions about 1 day after studying it, then 3
   days, 7 days, 21 days, then monthly. Each review should take minutes. Missed a
   question twice? Put it in your own log (see [`00-introduction/03-study-loop.md`](../00-introduction/03-study-loop.md)) and
   make a flashcard of it.
4. **Teach it.** Explain "life of a request" ([`21-life-of-a-request.md`](21-life-of-a-request.md)) to a rubber duck or a
   friend with no notes. Where you stall is the gap.
5. **Run it.** Every concept below has a command that shows it. A fact you have
   *seen in a `tcpdump`* is retained far longer than one you read.

### The skeleton: twelve facts to hold in your head
Everything else in this directory hangs off these. If you can say all twelve and
explain each in a sentence, you own the structure.

1. **Layers** wrap data in nested headers: frame (next hop) > packet (right host) >
   segment (right process) > message (right meaning). [`01-fundamentals.md`](01-fundamentals.md)
2. Layers 1–3 are **hop-by-hop**; 4+ are **end-to-end** — until a proxy terminates the
   connection and becomes a new end. [`01-fundamentals.md`](01-fundamentals.md)
3. A connection is identified by a **4-tuple**; a **socket** is an endpoint (IP + port), not a
   port. [`02-addressing.md`](02-addressing.md)
4. **NAT** rewrites addresses/ports and remembers the mapping; it only works if replies return
   through it. [`02-addressing.md`](02-addressing.md), [`08-ip-and-icmp.md`](08-ip-and-icmp.md)
5. **TCP is a byte stream**: no message boundaries — your protocol needs framing.
   [`03-byte-streams.md`](03-byte-streams.md), [`12-tcp.md`](12-tcp.md)
6. A new TCP connection costs **1 RTT** (handshake), TLS 1.3 another **1 RTT**, and a new
   connection starts with a small window — hence **connection reuse**. [`12-tcp.md`](12-tcp.md),
   [`13-tcp-reliability.md`](13-tcp-reliability.md), [`19-tls.md`](19-tls.md)
7. **Flow control** protects the receiver, **congestion control** protects the network; send
   at most `min(rwnd, cwnd)` in flight. [`13-tcp-reliability.md`](13-tcp-reliability.md)
8. **Close is FIN (polite) or RST (abort)**; the closer holds `TIME_WAIT`; a pile of
   `CLOSE_WAIT` is *your* bug. [`12-tcp.md`](12-tcp.md)
9. **DNS** is a cached tree walk; negative answers are cached too; long-lived processes must
   re-resolve. [`14-dns.md`](14-dns.md)
10. **HTTP** is stateless text/binary framing around methods, status codes and headers; some
    headers are **hop-by-hop**. [`15-http.md`](15-http.md), [`16-http1-wire-format.md`](16-http1-wire-format.md)
11. **TLS** = asymmetric to agree on a key, symmetric to encrypt, certificates to prove identity;
    **SNI** and **ALPN** let a proxy pick the cert and protocol. [`06-crypto-basics.md`](06-crypto-basics.md), [`19-tls.md`](19-tls.md)
12. A **proxy is two connections** with separate state and failure modes; you only ever see your
    hop. [`21-life-of-a-request.md`](21-life-of-a-request.md)

### Question bank, by file
Answer each without notes, then check the arrow. Starred (*) questions are the
ones people most often get wrong.

**01 fundamentals** ([`01-fundamentals.md`](01-fundamentals.md))
- Name the four TCP/IP layers, the unit and the address used at each.
- What does a router do to the Ethernet header of a packet it forwards, and why?
- Why can a proxy add retries and TLS that a router cannot? *

**02 addressing** ([`02-addressing.md`](02-addressing.md))
- What exactly identifies a TCP connection? Is "socket" the same as "port"? *
- How many addresses are in a `/24`? How does a host decide a destination is on its own subnet?
- What does NAT rewrite on an outgoing packet, and what breaks if replies take another path?

**03 byte streams** ([`03-byte-streams.md`](03-byte-streams.md))
- A sender does two `write`s of 100 bytes. List every sequence of reads the receiver may see. *
- What do you gain and lose choosing UDP over TCP?

**04 latency/throughput** ([`04-latency-throughput.md`](04-latency-throughput.md))
- Which of latency, bandwidth, throughput, RTT decides the cost of a handshake?
- Compute the bandwidth-delay product of 1 Gbit/s at 40 ms RTT. What if the window is smaller? *

**05 proxy taxonomy** ([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md))
- Forward vs reverse proxy: who is configured to know the proxy exists?
- What can an L7 proxy do that an L4 cannot, and what does it cost?

**06 crypto basics** ([`06-crypto-basics.md`](06-crypto-basics.md))
- Why does TLS use both asymmetric and symmetric crypto?
- What does a certificate bind together, and what does a client verify? *
- HMAC vs digital signature: when is each appropriate?

**07 link layer** ([`07-link-layer.md`](07-link-layer.md))
- A host sends to an IP outside its subnet. What is the destination MAC? Why? *
- How does a switch learn where a MAC lives, and what does it do with an unknown one?
- Where does the MSS of 1460 come from?

**08 IP and ICMP** ([`08-ip-and-icmp.md`](08-ip-and-icmp.md))
- Two routes match a destination. Which does the router pick? What is the default route?
- With DF set, a packet is too big for a link. What happens, and what symptom appears if ICMP is blocked? *
- How does `traceroute` work using TTL?

**09 UDP** ([`09-udp.md`](09-udp.md))
- What does UDP add on top of IP? What happens to a datagram larger than your receive buffer?
- Why do amplification attacks work on UDP, and how does QUIC limit them?

**10 tools** ([`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md))
- Decode: `Flags [P.], seq 1:79, ack 1, length 78`. What happened?
- What do `Recv-Q`/`Send-Q` mean on a *listening* socket vs an *established* one? *
- Which `curl -w` fields isolate TCP-connect and TLS-handshake time?

**11 sockets** ([`11-socket.md`](11-socket.md))
- List the server and client syscalls in order. Which returns a new fd?
- What happens when the listen backlog fills, and where do you see it? *
- `SO_REUSEADDR` vs `SO_REUSEPORT`; `shutdown(SHUT_WR)` vs `close`.

**12 TCP** ([`12-tcp.md`](12-tcp.md))
- A segment has `seq=1000` and 500 payload bytes. What `ack` does the peer send?
- A growing pile of `CLOSE_WAIT` vs a growing pile of `TIME_WAIT`: whose bug, and the fix? *
- Why does closing a socket that has unread data send an RST, and what is lost?
- Why is the default TCP keepalive useless behind a NAT?

**13 TCP reliability** ([`13-tcp-reliability.md`](13-tcp-reliability.md))
- RTO vs fast retransmit: what triggers each? What is a duplicate ACK?
- Flow-control window vs congestion window: what does each protect, and which limits throughput now?
- Why is a fresh connection slow, and what is slow-start-after-idle? *
- Explain the Nagle + delayed-ACK stall step by step.

**14 DNS** ([`14-dns.md`](14-dns.md))
- In what order does `getaddrinfo` look things up? Why does `ndots:5` add queries?
- `NXDOMAIN` vs NODATA. How long is a negative answer cached? *
- What does *not* re-resolve when DNS changes? How do you defend against DNS rebinding?

**15 HTTP** ([`15-http.md`](15-http.md))
- Name the four request-target forms. Why is `Host` required?
- `302` vs `307` vs `308`; what does `304` mean?
- Which headers are hop-by-hop? Why can't `Set-Cookie` be comma-joined? Why must `X-Forwarded-For` be treated as forgeable? *

**16 HTTP/1.1 wire format** ([`16-http1-wire-format.md`](16-http1-wire-format.md))
- A request has both `Content-Length` and `Transfer-Encoding: chunked`. What must you do, and which attack does that stop? *
- How does a chunked body end? How do you know where a request's body ends?

**17 HTTP/2** ([`17-http2.md`](17-http2.md))
- What HTTP/1.1 problem does multiplexing solve, and which problem remains because of TCP?
- Why is HPACK state a danger for a proxy? What is Rapid Reset?

**18 HTTP/3** ([`18-http3.md`](18-http3.md))
- Why does QUIC run over UDP, and how does it avoid head-of-line blocking?
- What is the Connection ID for? How does a client learn a server speaks HTTP/3?

**19 TLS** ([`19-tls.md`](19-tls.md))
- What in the ClientHello lets a proxy choose the certificate and the protocol?
- How many RTTs does a TLS 1.3 handshake take, and what removes them for repeat visits?

**20 PROXY protocol** ([`20-proxy-protocol.md`](20-proxy-protocol.md))
- What problem does it solve that `X-Forwarded-For` can't? What is the trust caveat? *

**21 life of a request** ([`21-life-of-a-request.md`](21-life-of-a-request.md))
- Narrate steps 1–10 with RTT counts. Which connections exist, and who owns each?
- "Connect hangs then times out" vs "connection refused": what is each telling you? *

### Draw it from memory
Redraw each on blank paper, then compare with the file. If you can't, you don't
own it yet.
1. The four-layer stack with units, addresses, devices ([`01-fundamentals.md`](01-fundamentals.md)).
2. A frame with all nested headers, labelled with who reads what ([`07-link-layer.md`](07-link-layer.md)).
3. The TCP state machine, with the client path, server path and close paths ([`12-tcp.md`](12-tcp.md)).
4. The TCP handshake and a data exchange with real seq/ack numbers ([`12-tcp.md`](12-tcp.md)).
5. cwnd over time through slow start, a loss, and recovery ([`13-tcp-reliability.md`](13-tcp-reliability.md)).
6. The DNS resolution chain from your program to the authoritative server ([`14-dns.md`](14-dns.md)).
7. The TLS 1.3 handshake as a timeline ([`19-tls.md`](19-tls.md)).
8. A client, a reverse proxy and an upstream with *both* connections and every timeout ([`21-life-of-a-request.md`](21-life-of-a-request.md)).

### Predict, then run
For each: write your prediction first, run it, and note any surprise in your log.
1. `ping -M do -s 1473 <host>` — what error, and why?
2. `iptables ... -j DROP` vs `-j REJECT --reject-with tcp-reset` on a port, then `curl` it — what does each show in `curl -v` and `tcpdump`?
3. Close a socket with unread data — does the peer see FIN or RST?
4. `tc qdisc add dev lo root netem delay 50ms`, then `curl -w` — which timing fields grow, and by how much?
5. `dig` a nonexistent name twice — does the second answer come faster, and why?
6. Open 100 short connections without keep-alive — how many `TIME_WAIT`, on which side?

### Gotcha: recognition is not recall
The most dangerous feeling is "oh yes, I remember that" while reading the
answer. That is *recognition*, and it is cheap and unreliable. If you did not
produce the answer before looking, count it as missed.

## Practice
1. Today, close this file and write down the twelve skeleton facts from memory;
   score yourself against the list, then repeat after 1, 3 and 7 days and record
   the score each time in your learning log.
2. Pick the five starred questions you missed and turn each into a flashcard
   (question on one side, a one-line answer and a link on the other).
3. Teach [`21-life-of-a-request.md`](21-life-of-a-request.md) aloud with no notes while drawing items 1, 3 and 8
   above, and list every place you hesitated.
4. Run three of the "Predict, then run" experiments against
   [`labs/00-tcp-server`](../../labs/00-tcp-server) or [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) and write what each taught you
   that reading hadn't.
