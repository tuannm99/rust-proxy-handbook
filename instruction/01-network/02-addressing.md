# Addressing: IP, Ports, CIDR, NAT

Part of the from-scratch fundamentals series — see `01-network/01-fundamentals.md`
for the full index. This file covers how a specific process on a specific
machine gets identified well enough that a packet can find it.

## What to learn

### IP addresses identify a host
An **IP address** identifies a host — a specific machine (more precisely,
a specific network interface) on the network. It answers "which
computer." Routers along the path use it to decide which direction to
forward a packet, hop by hop, without needing to know anything about what
the packet contains.

### Ports identify a process on that host
A **port** is a 16-bit number (0-65535) that identifies a specific
listening program *on* that host. It answers "which program on that
computer." One machine can run a web server on port 443 and an SSH daemon
on port 22 at the same time — the IP address gets a packet to the right
*machine*, the port gets it to the right *program*.

Ports below 1024 are "well-known" (80, 443, 22, 53 — HTTP, HTTPS, SSH,
DNS) and conventionally require elevated privilege to bind on Unix, which
is why a proxy often either runs as root briefly to bind port 443 and
then drops privileges, or binds a high port and relies on something else
(a load balancer, `iptables`, a capability grant) to get traffic to it.

Ports above roughly 32768 (the exact range is configurable, see
`/proc/sys/net/ipv4/ip_local_port_range`) are **ephemeral**: the OS
assigns one automatically to the *client* side of an outgoing connection,
picking from that pool and releasing it when the connection closes.
Gotcha, and a real production one: a proxy that opens many short-lived
outbound connections to the same upstream can exhaust its own ephemeral
port pool (roughly 28,000 available by default) faster than `TIME_WAIT`
releases them — this is the practical reason `06-proxy/01-upstream.md`
and `01-network/08-tcp.md` push so hard toward connection reuse rather
than dial-per-request.

### A socket, precisely
A **socket** is the combination: an IP address plus a port, representing
one endpoint of a connection. A single TCP connection is actually
identified by *four* values together (the "4-tuple"): source IP, source
port, destination IP, destination port. That's why one server process
listening on one port can serve thousands of simultaneous clients — each
client's 4-tuple is different even though the server's IP and port are
fixed. `01-network/07-socket.md` covers the actual API that creates one.

### CIDR notation: describing a block of addresses
An IPv4 address is 32 bits, written as four decimal octets
(`192.0.2.1`). **CIDR notation** (`10.0.0.0/8`) describes a *block*: the
number after the slash is how many leading bits are fixed (the "network"
part), leaving the rest free (the "host" part).

```
10.0.0.0/8   -> first 8 bits fixed  -> 2^24 addresses (all of 10.x.x.x)
10.0.0.0/24  -> first 24 bits fixed -> 2^8 addresses  (10.0.0.0-10.0.0.255)
10.0.0.5/32  -> all 32 bits fixed   -> exactly one address
```

IPv6 addresses are 128 bits, written as eight groups of hex digits
(`2001:db8::1`, with `::` collapsing one run of zero groups), and use the
same slash notation (`2001:db8::/64`). You'll need this fluently for
`07-security/08-ip-filtering.md` (allow/deny lists) and
`07-security/07-ratelimit.md` (why keying a rate limiter on a full IPv6
address hands an attacker 2^64 free identities inside their own `/64` —
covered in depth there).

### NAT: the address you see isn't always the address that was sent
**Network Address Translation (NAT)** rewrites the source and/or
destination address of a packet as it passes through a device, so that
many real addresses can share one, or a private address can reach the
public internet. Two shapes matter here:

- **SNAT (source NAT)**, the common home-router/CGNAT case: a device
  rewrites many internal clients' source addresses to one public IP as
  their traffic leaves, and rewrites the reply back on the way in,
  tracking which internal client owns which outbound connection. From a
  server's point of view, thousands of different home users behind one
  ISP's CGNAT can all arrive looking like the *same* source IP.
- **DNAT (destination NAT)**, what a load balancer or a Kubernetes
  Service does: an incoming packet addressed to one public/virtual IP
  gets its destination rewritten to whichever real backend should handle
  it, before your proxy ever sees it.

Why this matters for a proxy specifically: by the time a connection
reaches your listening socket, `peer_addr()` may already be several NAT
hops removed from the actual client — this is *exactly* the problem
`01-network/14-proxy-protocol.md` and the `X-Forwarded-For` discussion in
`07-security/08-ip-filtering.md` exist to solve, and it's why "just trust
the socket's peer address" is naive the moment there's any load balancer,
NAT gateway, or CDN in front of you.

### Routing, briefly
A packet whose destination isn't on the local network gets sent to a
**gateway** (the "default route") — a router that knows (or knows who to
ask) how to get the packet one hop closer to its destination. Your
machine doesn't need a full map of the internet; it only needs to know
its default gateway, and every router along the path makes the same
local, one-hop decision. `ip route` (Linux) or `route -n` shows your
machine's routing table; `traceroute`/`mtr` shows the hop-by-hop path a
packet actually takes to reach somewhere. This is background for
understanding *why* latency accumulates hop by hop (`04-latency-throughput.md`)
rather than something you'll implement — routing is exclusively the
kernel/router's job, never an application's.

## Practice
1. Run `ip addr` (or `ifconfig`) and identify your machine's own IP
   address(es); run `ip route` and identify your default gateway.
2. Run `traceroute example.com` (or `mtr` for a live view) and count the
   hops; compare that hop count against the RTT you measured in
   `04-latency-throughput.md`'s exercises.
3. Compute, by hand, how many addresses `10.0.0.0/8` and `2001:db8::/64`
   each cover, then verify with a CIDR calculator.
4. If you're behind NAT (almost everyone is, at home), visit a
   "what's my IP" site and compare the address it reports against your
   machine's own address from step 1 — they'll differ; that gap is your
   router's SNAT in action.
5. In `labs/00-tcp-server`, connect two different clients simultaneously
   and log each connection's full 4-tuple (`local_addr()` +
   `peer_addr()`) — confirm they differ only in source port if both
   clients are on the same machine.
