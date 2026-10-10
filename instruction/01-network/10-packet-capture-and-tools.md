# Reading the Network: tcpdump, ss, ip, curl, dig, openssl

Every later file's Practice says "capture it" or "check with `ss`". This
file teaches how to *read* those tools' output, so a packet trace becomes
evidence instead of noise. Install notes for the less common tools live in
[`12-testing/06-lab-environment.md`](../12-testing/06-lab-environment.md); the method for using them when something is broken
is [`12-testing/05-debugging.md`](../12-testing/05-debugging.md).

## What to learn

### tcpdump: capture and filter
`tcpdump` records packets seen by an interface. It needs root (or
`CAP_NET_RAW`). Flags worth memorizing:

```text
sudo tcpdump -i lo -n -nn port 8080          # -n: no DNS lookups, -nn: no port names
sudo tcpdump -i any -n host 10.0.0.7 and tcp # BPF filter expression
sudo tcpdump -i lo -n -X -s0 port 8080       # -X: hex+ASCII payload, -s0: whole packet
sudo tcpdump -i lo -n -w cap.pcap port 8080  # save for Wireshark; -r cap.pcap to read
sudo tcpdump -i any -n 'tcp[tcpflags] & (tcp-syn|tcp-fin|tcp-rst) != 0'
```

Always `-n` (otherwise tcpdump blocks doing reverse DNS and distorts what
you're measuring), always a filter on a busy host. `-i lo` for local
tests, `-i any` to see all interfaces (but then you lose the link-layer
header). The filter language is **BPF**, the same machinery eBPF grew out
of ([`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md)). Capture happens in the kernel before
your application sees data and after it has sent, so it shows what is
*actually on the wire* — including packets your code never knew about.

### Reading one TCP line
```text
12:00:01.000100 IP 127.0.0.1.52000 > 127.0.0.1.8080: Flags [S], seq 1000, win 65495, options [mss 65495,sackOK,TS val 1 ecr 0,nop,wscale 7], length 0
12:00:01.000120 IP 127.0.0.1.8080 > 127.0.0.1.52000: Flags [S.], seq 2000, ack 1001, win 65483, ..., length 0
12:00:01.000130 IP 127.0.0.1.52000 > 127.0.0.1.8080: Flags [.], ack 2001, win 512, length 0
12:00:01.000300 IP 127.0.0.1.52000 > 127.0.0.1.8080: Flags [P.], seq 1:79, ack 1, win 512, length 78
```

Decode: `src.port > dst.port`; **Flags** `S` = SYN, `.` = ACK only,
`S.` = SYN+ACK, `P.` = PSH+ACK (data), `F.` = FIN+ACK, `R` = RST.
`seq 1:79` = bytes 1 through 78 of the stream (tcpdump prints
**relative** sequence numbers after the handshake; `-S` shows absolute).
`ack` = next byte the sender expects. `length` = payload bytes. `win` =
advertised receive window. `options` show MSS, SACK, window scaling,
timestamps — all negotiated in the SYN ([`13-tcp-reliability.md`](13-tcp-reliability.md)). Given
this vocabulary you can read a handshake, a request, a retransmission
(same `seq` appearing twice) and a reset at a glance.

### Wireshark and tshark
Wireshark opens a `.pcap`, decodes every layer into a tree, and can
**follow a TCP stream** (reassembling both directions into the
application bytes). `tshark` is the same engine on the command line:
`tshark -r cap.pcap -Y 'http.request' -T fields -e http.host`. Capture on
the server with tcpdump, analyze on your laptop in Wireshark — the usual
workflow. TLS payload is opaque unless you give Wireshark the session
keys: set `SSLKEYLOGFILE=keys.log` for a client that supports it (curl and
browsers do; rustls has a `KeyLogFile` helper), then point Wireshark's TLS
preferences at that file ([`19-tls.md`](19-tls.md)).

### ss: the kernel's socket table
`ss` (replaces `netstat`) prints sockets straight from the kernel.

```text
ss -tlnp            # TCP, Listening, Numeric, Process: who listens where
ss -tn state established '( dport = :8080 or sport = :8080 )'
ss -tn state time-wait | wc -l
ss -tni             # -i: per-socket internals (rtt, cwnd, retrans, mss)
ss -s               # summary counts by state
```

For a listening socket, `Recv-Q` is the current **accept-queue length** and
`Send-Q` is its maximum (the backlog) — the number to watch when
connections are being dropped ([`11-socket.md`](11-socket.md)). For an established socket,
`Recv-Q` is bytes the kernel has received that your process has *not read
yet* and `Send-Q` is bytes sent but not yet acknowledged. A persistently
growing `Recv-Q` means your app is slow to read (backpressure
[`12-tcp.md`](12-tcp.md)); a growing `Send-Q` means the peer or network is slow.
`ss -tni` exposes `rtt`, `cwnd`, `retrans` and `bytes_acked`, the TCP
state the kernel keeps for you.

### ip and nc: addresses, routes, and a hand-made client
`ip addr`, `ip route`, `ip neigh`, `ip -s link` (counters) cover
interfaces, routing and ARP ([`07-link-layer.md`](07-link-layer.md), [`08-ip-and-icmp.md`](08-ip-and-icmp.md)).
`ip route get <dst>` asks the kernel which route and source address it
would pick. `nc` (netcat) opens a raw TCP or UDP connection and lets you
type bytes — `printf 'GET / HTTP/1.1\r\nHost: x\r\n\r\n' | nc 127.0.0.1 8080`
sends a hand-built request; `nc -l 9999` listens; `nc -z host 1-1024`
scans ports. `nc` is how you prove a bug is in the protocol bytes rather
than in curl's helpfulness.

### curl -v and --resolve
`curl -v` prints the whole exchange: DNS result, TCP connect, TLS
handshake (version, cipher, certificate), request headers (`>`), response
headers (`<`). Useful variants:

```text
curl -sS -o /dev/null -w 'dns=%{time_namelookup} tcp=%{time_connect} tls=%{time_appconnect} ttfb=%{time_starttransfer} total=%{time_total}\n' https://example.com/
curl --resolve example.com:443:127.0.0.1 https://example.com/   # bypass DNS, keep SNI/Host
curl --http1.1 / --http2 / --http3 URL      # pick the protocol
curl --path-as-is 'http://h/a/../b'         # don't normalize the path
curl -H 'Transfer-Encoding: chunked' -d @file URL
```

The `-w` timing line gives the latency breakdown from
[`04-latency-throughput.md`](04-latency-throughput.md) in one command: each field is cumulative, so
`tcp - dns` is the TCP handshake cost, `tls - tcp` the TLS handshake,
`ttfb - tls` the server's think time.

### dig and openssl s_client
```text
dig example.com A +noall +answer            # records and TTL
dig @1.1.1.1 example.com AAAA               # ask a specific resolver
dig +trace example.com                      # walk root -> TLD -> authoritative
openssl s_client -connect host:443 -servername host -alpn h2 </dev/null
openssl s_client -connect host:443 -showcerts   # print the chain
```

`dig` talks DNS directly, bypassing the OS resolver config, so differences
between `dig` and your program point at `/etc/resolv.conf`, `nsswitch`,
or caching ([`14-dns.md`](14-dns.md)). `openssl s_client` performs a TLS client
handshake and prints the negotiated version, cipher, ALPN choice and
certificate chain; `-servername` sets SNI — leave it off against a
virtual-hosted server and you get the wrong certificate (the SNI
mismatch bug, [`19-tls.md`](19-tls.md)).

### Gotcha: where you capture decides what you see
Capture on the **client** side and you see packets as sent (including
ones later dropped); capture on the **server** and you see only what
arrived. Loopback traffic skips the NIC entirely — no real checksums,
no MTU limit, no loss — so a local test cannot reproduce a PMTUD or
offload problem. With TCP segmentation offload, tcpdump may show
"packets" larger than the MTU, because the NIC does the cutting after the
capture point. And a capture on a proxy shows two separate
conversations (client-side and upstream-side) that you must correlate by
timestamps and ports yourself — which is exactly what a trace ID
([`08-observability/`](../08-observability)) automates.

## Practice

1. Start [`labs/00-tcp-server`](../../labs/00-tcp-server) and run `sudo tcpdump -i lo -n port <p>`;
   in another terminal `printf 'hello\n' | nc 127.0.0.1 <p>`. Annotate every
   line of the trace: which is SYN, SYN-ACK, ACK, the data segment, its ACK,
   FIN, and the final ACK.
2. Save that trace with `-w`, open it in Wireshark (or `tshark -r`), and
   use "Follow TCP stream" to see the application bytes. Then run
   `tcpdump -r cap.pcap -n -X` and locate the same bytes in the hex dump.
3. While a long `nc` session is open, run `ss -tni dst 127.0.0.1` and
   identify its state, `rtt`, `cwnd`, and queue sizes; stop reading on the
   server side (a `sleep` in the handler) while the client keeps sending and
   watch `Recv-Q`/`Send-Q` grow.
4. Run the `curl -w` timing command from above against a remote HTTPS
   site and against a local plain-HTTP server; explain every field and
   which of them is zero for the local case, and why.
5. Use `dig +trace example.com` to walk the delegation chain, then compare
   `dig example.com` with `getent hosts example.com` (what most programs
   use) and explain any difference in output or latency.
6. Run `openssl s_client -connect example.com:443 -servername example.com
   -alpn h2 </dev/null`, then again without `-servername` against a
   multi-tenant host, and compare the certificates returned. Finally set
   `SSLKEYLOGFILE=$PWD/keys.log`, run `curl https://example.com/` under
   `tcpdump -w`, and read the decrypted HTTP in Wireshark.
