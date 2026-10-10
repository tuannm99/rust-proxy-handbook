# The Link Layer: Ethernet, MAC, Switches, ARP

How a packet gets from one machine to the machine next to it. Everything
above this layer ([`08-ip-and-icmp.md`](08-ip-and-icmp.md), [`12-tcp.md`](12-tcp.md)) assumes the "next hop" problem is
solved; this file is where it gets solved.

## What to learn

### Layers are encapsulation, nothing more
A "layer" is a header glued in front of the layer above's data. Your proxy
calls `write(fd, b"GET / HTTP/1.1...")`. The kernel wraps those bytes in a
TCP header (ports, sequence numbers), wraps that in an IP header (source
and destination IP), wraps that in an Ethernet header (source and
destination MAC), and hands the result to the network card.

```text
| Ethernet hdr 14B | IP hdr 20B | TCP hdr 20B | HTTP bytes ... | FCS 4B |
|<--------------------------- one frame on the wire ----------------->|
```

Each layer only reads *its own* header and treats everything after it as
opaque payload. A switch reads only the Ethernet header. A router reads
up to the IP header. Your L4 load balancer reads up to the TCP header;
your L7 proxy ([`05-proxy-taxonomy.md`](05-proxy-taxonomy.md)) reads all the way into HTTP. "L4 vs L7"
is literally "how deep into this stack of headers does the box look."
The full layer table (OSI numbers vs the TCP/IP model) is in [`01-fundamentals.md`](01-fundamentals.md);
this file is layer 2.

### MAC addresses: identity on one local network
A **MAC address** is a 48-bit identifier burned into (or assigned to) a
network interface, written `aa:bb:cc:dd:ee:ff`. It only means something
on the **local segment** — the set of machines that can reach each other
without crossing a router. IP addresses are global-ish and routable; MAC
addresses are flat and local. When your packet crosses a router, the
Ethernet header is *thrown away and rebuilt* for the next segment: the
IP header stays the same (apart from TTL), the MACs change at every hop.
`ff:ff:ff:ff:ff:ff` is the **broadcast** address: every interface on the
segment accepts the frame.

### Switches: learn, then forward
A **switch** connects machines on one segment. It keeps a table mapping
MAC address -> physical port, built by watching the *source* MAC of
frames arriving on each port. For a frame whose destination MAC it has
learned, it forwards out that one port; for an unknown destination or
broadcast, it **floods** out every port except the one it came in on. A
**hub** (obsolete) floods everything. A switch is invisible to IP: it
doesn't change addresses and has no IP address of its own to route with.
The set of machines that receive each other's broadcasts is a **broadcast
domain**; keeping it small is why networks are subdivided.

### ARP: the bridge between IP and MAC
Your machine wants to send to `10.0.0.7`, on its own subnet
([`02-addressing.md`](02-addressing.md)). The Ethernet header needs a MAC, not an IP. So it
broadcasts **ARP**: "who has 10.0.0.7? tell 10.0.0.5." The owner replies
with its MAC; the answer is cached in the **neighbor table** (`ip neigh`)
for a few minutes. If the destination is *not* on your subnet, you ARP
for the **default gateway's** MAC instead and send the frame to the
gateway — with the IP destination still the far-away host. That one
detail (frame addressed to the gateway, packet addressed to the final
host) is what routing is.
IPv6 replaces ARP with Neighbor Discovery (ICMPv6), same idea.

```text
$ ip neigh
10.0.0.1 dev eth0 lladdr 52:54:00:12:35:02 REACHABLE
10.0.0.7 dev eth0 lladdr 08:00:27:aa:bb:cc STALE
```

### MTU: the biggest frame a link carries
Ethernet carries at most **1500 bytes of IP packet** per frame (the
**MTU**). Subtract 20 (IP) and 20 (TCP) and you get the **MSS** — 1460
bytes of TCP payload per segment — which is why a 100 KB response is
~70 segments, not one. Tunnels (VPN, VXLAN, GRE) add their own header and
therefore *lower* the usable MTU of the path; the loopback interface has
a huge MTU (65536), which is one reason local benchmarks look better than
real networks. What happens when a packet is bigger than a link's MTU is
in [`08-ip-and-icmp.md`](08-ip-and-icmp.md).

### VLANs, bridges, and virtual interfaces
A **VLAN** adds a 4-byte tag so one physical switch acts as several
isolated broadcast domains. On a Linux host, the same ideas exist in
software: a `veth` pair is a virtual cable between two network
namespaces, a **bridge** (`docker0`, `cni0`) is a software switch that
joins them, and every container's `eth0` is the end of such a cable
([`02-linux/13-containers.md`](../02-linux/13-containers.md)). When a Kubernetes pod talks to a
neighbor pod on the same node, the "network" is a Linux bridge doing
MAC learning exactly as above — no wire involved.

### Gotcha: the wire is not reliable, and nothing at this layer fixes it
Ethernet has a checksum (FCS) that makes a NIC *drop* a corrupted frame,
but there is no acknowledgement and no retransmission. A switch with a
full output queue drops frames silently. Every guarantee you rely on
later — ordered, complete, no duplicates — is rebuilt on top by TCP
([`13-tcp-reliability.md`](13-tcp-reliability.md)). This is also why `ip -s link` counters
(`dropped`, `overrun`, `errors`) are the first place to look when a
loaded proxy host loses packets before your code ever sees them.

## Practice

1. Run `ip -br link` and `ip -br addr` and identify your machine's
   interfaces, their MACs, and their IPs. Find `lo` and note its MTU
   with `ip link show lo`.
2. Run `ip neigh`, then `ping -c1 <your-gateway-ip>` (get it from
   `ip route | grep default`), then `ip neigh` again — watch the gateway
   entry appear or refresh.
3. Capture ARP live: in one terminal `sudo tcpdump -n -e -i <iface> arp`
   (`-e` prints MAC addresses), in another `sudo ip neigh flush all`
   followed by a `ping` to a LAN host. Identify the request (broadcast)
   and the reply (unicast) and explain who sent each.
4. With `sudo tcpdump -n -e -i <iface> icmp` running, `ping` a host
   *outside* your subnet and compare the destination MAC in the frame
   against the MAC from `ip neigh` for your default gateway. Explain why
   it is the gateway's MAC and not the remote host's.
5. Run `ip -s link show <iface>` before and after a large transfer and
   read the `RX`/`TX` packet and `errors`/`dropped` counters.
6. Create two network namespaces joined by a `veth` pair
   (`ip netns add`, `ip link add ... type veth peer name ...`, `ip link set
   ... netns ...`), assign addresses, ping across, and inspect the
   neighbor table inside each namespace — the same machinery container
   networking uses. Then run [`labs/00-tcp-server`](../../labs/00-tcp-server) in one namespace
   and connect to it from the other with `nc`.
