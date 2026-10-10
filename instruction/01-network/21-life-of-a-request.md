# The Life of a Request: Every Layer, End to End

One `curl https://app.example.com/api` followed from keystroke to response,
through a reverse proxy, naming the file in this directory that explains each
step — then turned around into a symptom-to-layer table you can debug with.
This is the file to re-read after finishing the rest of `01-network/`: if any
step below is hazy, the link says where to go back.

## What to learn

### The whole trip, in order
```text
 1  parse URL            scheme=https host=app.example.com port=443 path=/api
 2  DNS                  app.example.com -> 203.0.113.10           [14-dns]
 3  route + ARP          dst not on my subnet -> frame to gateway   [07, 08]
 4  NAT (home router)    src 192.168.1.5:51000 -> 198.51.100.7:40123 [02]
 5  TCP handshake        SYN / SYN-ACK / ACK                        [12]
 6  TLS handshake        ClientHello(SNI, ALPN) ... Finished        [19]
 7  HTTP request         GET /api, Host: app.example.com            [15, 16]
 8  proxy: accept->parse->route->upstream->forward                  [05-http-stack, 06-proxy]
 9  response bytes       slow start, ACKs, window                   [13]
10  reuse or close       keep-alive / FIN                           [12]
```

### 1-2: Names before packets
Before a single packet leaves, the client parses the URL (scheme picks the
default port: 443; the fragment is never sent) and resolves the host
([`14-dns.md`](14-dns.md)): `getaddrinfo` consults `/etc/hosts` and the local resolver
cache, which asks a recursive resolver, which — on a miss — walks root, TLD,
authoritative. Cost: 0 ms (cached), ~1 RTT to the resolver, or several RTTs for
a cold walk. If the site is behind a CDN, the answer is an **anycast** or
geo-chosen address near the client ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)). The client now has an
IP, and a TTL telling it how long that answer is good.

### 3-4: Getting a frame out, and the NAT on the way
The client compares the destination IP to its subnet ([`02-addressing.md`](02-addressing.md)): not
local, so it uses the **default route** and ARPs for the **gateway's MAC**
([`07-link-layer.md`](07-link-layer.md)). The frame is addressed to the gateway, the IP packet to the
server. The home router **rewrites the source** `192.168.1.5:51000` to its public
address and a chosen port, recording the mapping in a table; return traffic is
matched against that table. From here the packet crosses perhaps 10–15 routers,
each decrementing TTL, doing a longest-prefix lookup, and rebuilding the Ethernet
header for the next link ([`08-ip-and-icmp.md`](08-ip-and-icmp.md)). Nobody on the path knows the
path; each hop makes a local decision. The server's side may also sit behind a
cloud load balancer doing destination NAT or terminating the connection itself
([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)).

### 5: TCP — one RTT
The client calls `connect()`; the kernel sends `SYN` with an initial sequence
number and options (MSS, window scale, SACK). The SYN arrives at the proxy's
host, lands in a listening socket's SYN queue; the kernel replies `SYN+ACK`; the
client's `ACK` completes the handshake and the connection moves to the
**accept queue**, where the proxy's `accept()` will pick it up
([`11-socket.md`](11-socket.md), [`12-tcp.md`](12-tcp.md)). Client's `connect()` returned after **1 RTT**; the
server's side is complete one half-RTT later. The new connection has a small
congestion window (~10 segments) ([`13-tcp-reliability.md`](13-tcp-reliability.md)).

### 6: TLS — one more RTT (TLS 1.3)
The client sends `ClientHello` (supported ciphers, a key share, **SNI** =
`app.example.com`, **ALPN** = `h2, http/1.1`). The proxy picks the certificate
matching the SNI, answers with its key share, certificate and `Finished`; both
sides derive the same session keys ([`06-crypto-basics.md`](06-crypto-basics.md), [`19-tls.md`](19-tls.md)). The
client verifies the certificate chain, hostname and validity; the ALPN result
decides HTTP/2 vs HTTP/1.1. Total so far: DNS + 1 RTT (TCP) + 1 RTT (TLS 1.3)
before the first byte of HTTP — which is what resumption, 0-RTT, keep-alive and
QUIC ([`18-http3.md`](18-http3.md)) all attack.

### 7: The request, as bytes
```text
GET /api HTTP/1.1\r\nHost: app.example.com\r\nAccept: */*\r\n\r\n
```
Encrypted inside TLS records, split by TCP into segments ≤ MSS, carried in IP
packets, in Ethernet frames. In HTTP/2 the same request is a HEADERS frame on a
stream, HPACK-compressed ([`17-http2.md`](17-http2.md)). Segments may arrive split or merged; the
proxy's read loop must reassemble until the header terminator
([`16-http1-wire-format.md`](16-http1-wire-format.md), [`03-byte-streams.md`](03-byte-streams.md)).

### 8: Inside the proxy
The proxy's `accept()` returns; tokio spawns a task for the connection
([`04-runtime/`](../04-runtime)). It terminates TLS, parses the request
([`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md)), normalizes it and applies security checks
([`07-security/`](../07-security)), picks a route by `Host` + path
([`05-http-stack/04-router.md`](../05-http-stack/04-router.md)), chooses an upstream
([`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)), checks out a **pooled** upstream connection — or
pays another DNS + TCP (+ TLS) round trip to open one
([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)) — strips hop-by-hop headers, adds
`X-Forwarded-For` ([`15-http.md`](15-http.md)), forwards the request, and **streams** the
response back, with backpressure all the way ([`13-tcp-reliability.md`](13-tcp-reliability.md)). Failures
here become `502`/`503`/`504` ([`15-http.md`](15-http.md)).

### 9-10: The response, and what happens next
The response travels the same chain in reverse: upstream -> proxy (own TCP
connection, own window) -> client. Large responses ramp `cwnd`; every ACK both
confirms data and advertises window. When the exchange is done, the connection
stays open for reuse (HTTP keep-alive, [`05-http-stack/05-keepalive.md`](../05-http-stack/05-keepalive.md)) or
closes with `FIN`, the closer entering `TIME_WAIT` ([`12-tcp.md`](12-tcp.md)). Note there were
**two** independent connections the whole time — client-proxy and proxy-upstream
— with separate DNS, TCP state, TLS sessions, windows and failure modes; that is
what "L7 proxy" means ([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)).

### Symptom -> layer: using the model to debug
| What you observe | Most likely layer / cause | First tool |
|---|---|---|
| `Could not resolve host` / `NXDOMAIN` | DNS: name wrong, record missing, search-domain/`ndots` | `dig`, `getent hosts` |
| connect hangs, then timeout | packets dropped: firewall, wrong route, down host, PMTUD black hole | `tcpdump` (SYN, no reply), `traceroute` |
| `Connection refused` (instant) | host reachable, nothing listening (RST) | `ss -tlnp` on server |
| connect OK, then `Connection reset` | app closed with unread data, or middlebox RST | `tcpdump` for `R` flag |
| TLS error / wrong cert | SNI missing, expired/untrusted cert, ALPN mismatch | `openssl s_client -servername` |
| small responses fine, large ones hang | MTU / PMTUD (blocked ICMP) | `ping -M do -s`, MSS clamp |
| ~40 ms added to every small request | Nagle + delayed ACK | `TCP_NODELAY` |
| fast at first, slow bulk transfers | cwnd ramp, loss, bufferbloat | `ss -ti`, `netem` repro |
| `502` | upstream refused/invalid/closed early | proxy logs, `ss` on upstream |
| `504` | upstream accepted but too slow | latency breakdown, upstream metrics |
| fd exhaustion, many `CLOSE_WAIT` | app doesn't close sockets | `ss -tan state close-wait` |
| many `TIME_WAIT`, `EADDRNOTAVAIL` | short upstream connections, no reuse | pool, `ip_local_port_range` |

### Gotcha: you only see your hop
Each participant sees only its own segment: the client sees one connection to
the proxy, the upstream sees one from the proxy, and neither sees the other.
The IP in the backend's `accept()` is the proxy's unless you propagate the
client's ([`15-http.md`](15-http.md), [`20-proxy-protocol.md`](20-proxy-protocol.md)). Timeouts compose
too: a client timeout shorter than the proxy's upstream timeout leaves the proxy
working for a client that left. Tracing ([`08-observability/03-tracing.md`](../08-observability/03-tracing.md)) exists to
stitch the segments back into one story.

## Practice

1. Pick a URL and, using only the tools from [`10-packet-capture-and-tools.md`](10-packet-capture-and-tools.md), produce a
   timeline with a number for every step above: DNS time, TCP connect, TLS,
   time to first byte, total (`curl -w`), and confirm the counts against
   `tcpdump`'s timestamps for SYN / SYN-ACK / ClientHello.
2. With [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) running in front of a local upstream, capture
   both sides at once (`sudo tcpdump -i lo -n 'port <proxy> or port
   <upstream>'`) for one request and identify the client-proxy and
   proxy-upstream connections; show on paper which packets belong to which.
3. Run the same request twice through the proxy with upstream keep-alive on and
   off, capture each, and count the extra SYN/SYN-ACK/ACK in the "off" case;
   state the added latency as a number of RTTs.
4. For every row of the symptom table, reproduce one on your machine
   (a bad hostname, a closed port, a `DROP` rule with `iptables`, an
   `iptables ... -j REJECT --reject-with tcp-reset`, a wrong SNI, a
   `netem` loss/delay) and record what `curl -v` and `ss`/`tcpdump` show, so
   each symptom becomes a recognizable fingerprint.
5. Write, in your own words and without looking back, the sequence of events
   for a first-ever request to a new HTTPS site through a reverse proxy
   (steps 1–10), then diff it against this file and note what you missed.
