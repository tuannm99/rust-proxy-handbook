# The IP Layer: Packets, Routing, Fragmentation, ICMP

What an IP packet is, how routers decide where it goes next, what happens
when it is too big, and how the network reports problems. Builds on
[`02-addressing.md`](02-addressing.md) (addresses, CIDR, NAT) and [`07-link-layer.md`](07-link-layer.md) (frames, ARP).

## What to learn

### The IPv4 header: what a router actually reads
An IPv4 header is 20 bytes (more with options). The fields you need to
know:

```text
 version | header len | DSCP/ECN | total length
 identification | flags (DF, MF) | fragment offset
 TTL | protocol (6=TCP, 17=UDP, 1=ICMP) | header checksum
 source IP
 destination IP
```

- **Total length** is the packet's size including the header, up to 65535.
- **TTL** (time to live) is decremented by every router; at 0 the packet
  is dropped and the router reports it. This stops routing loops from
  circulating packets forever — and it is the trick `traceroute` uses.
- **Protocol** tells the receiving kernel which transport to hand the
  payload to: TCP, UDP, ICMP.
- A router **never looks past the IP header** for plain forwarding, which
  is why an L3 device cannot make decisions on HTTP paths or even ports
  (a firewall or NAT looking at ports is doing extra, stateful work).

### Routing: longest-prefix match, hop by hop
Every host and router holds a **routing table**: a list of
`prefix -> next hop, interface`. To forward a packet it finds the entries
whose prefix contains the destination and picks the **most specific**
(longest prefix) one. `0.0.0.0/0` is the **default route**: "anything I
don't know goes to the gateway."

```text
$ ip route
default via 10.0.0.1 dev eth0
10.0.0.0/24 dev eth0 proto kernel scope link src 10.0.0.5
172.17.0.0/16 dev docker0 scope link
```

Destination `10.0.0.9` matches `/24` (direct, ARP for it); `8.8.8.8` only
matches the default (ARP for `10.0.0.1`, hand it the packet). Routing is
**per-packet and per-hop**: no router knows the whole path, each makes a
local decision, and the forward path and return path may differ
(**asymmetric routing**). Asymmetry is invisible to TCP but breaks
stateful firewalls and NAT that see only one direction. A Linux box
forwards packets between interfaces only if `net.ipv4.ip_forward=1` —
which is what makes it a router (or a container host's NAT gateway).
Routing tables are filled by hand, by DHCP, or by protocols (OSPF, BGP) —
BGP is how the whole internet agrees on which networks live where, and
**anycast** (the same IP announced from many places; routing picks the
nearest) is how CDNs and public DNS resolvers like `1.1.1.1` work.

### MTU, fragmentation, and Path MTU Discovery
A link's MTU limits packet size ([`07-link-layer.md`](07-link-layer.md)). When a router must
forward a packet larger than the next link's MTU, IPv4 offers two
outcomes depending on the **DF** (Don't Fragment) bit:

- DF clear: the router **fragments** the packet; the destination
  reassembles. Fragmentation is slow, fragile (lose one fragment, lose
  the whole packet) and a classic attack surface.
- DF set: the router **drops** it and sends back an ICMP "Fragmentation
  Needed" message carrying the next-hop MTU.

TCP sets DF and uses this feedback — **Path MTU Discovery (PMTUD)** — to
discover the smallest MTU on the path and shrink its segment size.
IPv6 has no router fragmentation at all; only the sender may fragment.

**Gotcha — the PMTUD black hole.** If a firewall drops *all* ICMP
(a common misguided hardening), those "too big" messages never arrive.
Small packets (the handshake, short requests) work; the first full-size
segment of a large response vanishes and the connection **hangs** — the
classic "I can connect, small pages load, big ones stall" symptom,
especially over VPNs and tunnels with a reduced MTU. The fixes are
allowing ICMP type 3 code 4, or **MSS clamping** (a router rewrites the
MSS option in SYNs so both ends send small enough segments).

### ICMP: the network's error and diagnostic channel
**ICMP** rides inside IP (protocol 1) and carries control messages, not
application data. The ones to recognize:

- **Echo request/reply** — `ping`.
- **Destination unreachable** — with codes: network/host unreachable,
  port unreachable (a UDP packet hit a port with no listener — this is
  how UDP "connection refused" is observed), fragmentation needed.
- **Time exceeded** — TTL reached 0. `traceroute` sends probes with TTL 1,
  2, 3, ... and records which router replies "time exceeded" at each step,
  mapping the path.
- **Redirect** — a router telling you of a better first hop (commonly
  ignored for security).

ICMP is not optional plumbing: blocking it wholesale breaks PMTUD and
makes failures silent instead of fast.

### IPv6 in one page
128-bit addresses (`2001:db8::1`), a fixed 40-byte header with no
checksum and no router fragmentation, `::1` as loopback, `fe80::/10`
**link-local** addresses on every interface, `/64` as the standard subnet
size, and Neighbor Discovery instead of ARP. For a proxy the practical
consequences: listen on `[::]` (a dual-stack socket also accepts IPv4
unless `IPV6_V6ONLY` is set), parse `[addr]:port` in `Host` headers and
config, store client IPs as `IpAddr` (never `u32`), and expect upstream
DNS answers to contain both `A` and `AAAA` ([`14-dns.md`](14-dns.md)). NAT is rarely
needed because addresses are plentiful, so the real client IP is more
often visible end to end.

### Gotcha: TTL and hop count as a debugging and security tool
Senders start TTL at a known value (commonly 64 on Linux, 128 on
Windows), so the TTL on a received packet tells you roughly how many hops
it crossed. Some protocols exploit that: a service that should only be
spoken to by on-link peers can require TTL 255 (the "GTSM" trick), since
any packet that crossed a router has a lower value. And because a packet
lives a bounded number of hops, a routing loop shows up as `traceroute`
repeating the same two routers — a loop that would otherwise just look
like "timeouts".

## Practice

1. Run `ip route` and `ip route get 8.8.8.8` and `ip route get 10.0.0.5`
   (adapt to your subnet); explain for each which routing entry won and
   why, naming the prefix length that made it most specific.
2. Run `ping -c3 -M do -s 1472 <host>` (DF set, 1472 + 28 bytes of
   headers = 1500), then `-s 1473` and read the "Message too long"
   error; this shows your path MTU with your own eyes. Repeat against a
   host across a VPN if you have one.
3. Run `traceroute -n 8.8.8.8` (or `mtr -n -c 10 8.8.8.8`) and explain
   each line in terms of TTL and ICMP time-exceeded. Capture with
   `sudo tcpdump -n -i any icmp` while it runs and find the probes and the
   replies.
4. Run `sysctl net.ipv4.ip_forward` and explain what changes if you set it
   to 1; find which Docker/Kubernetes component sets it on a container
   host and why.
5. Connect to a UDP port with nothing listening
   (`echo hi | nc -u -w1 127.0.0.1 9999`) with `sudo tcpdump -n -i lo icmp`
   running, and find the ICMP port-unreachable reply.
6. Run `ip -6 addr` and `curl -6 -v http://[::1]:8080/` against
   [`labs/00-tcp-server`](../../labs/00-tcp-server) after making it listen on `[::]`; confirm whether
   an IPv4 client (`curl -4 http://127.0.0.1:8080/`) also connects, and
   explain from `IPV6_V6ONLY` why or why not.
