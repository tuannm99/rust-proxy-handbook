# Linux Networking: Interfaces, netfilter, conntrack, NAT, and Transparent Proxying

How the Linux kernel routes, filters and rewrites packets *around* your process — the
machinery behind `docker -p`, Kubernetes Services, `iptables -j REJECT` in your tests, and
"my proxy sees the wrong source IP." Ties the network files ([`01-network/07-link-layer.md`](../01-network/07-link-layer.md),
[`01-network/08-ip-and-icmp.md`](../01-network/08-ip-and-icmp.md)) to the OS ones ([`13-containers.md`](13-containers.md)).

## What to learn

### The packet's path through the kernel
A packet entering a Linux host goes through a fixed sequence, with checkpoints where rules can act:

```text
NIC -> [PREROUTING] -> routing decision -+-> [INPUT] -> local process (your proxy's socket)
                                         |
                                         +-> [FORWARD] -> [POSTROUTING] -> NIC (as a router)

your proxy writes -> routing -> [OUTPUT] -> [POSTROUTING] -> NIC
```

The five checkpoints are **netfilter hooks**. A **routing decision** (which interface, which next hop —
[`01-network/08-ip-and-icmp.md`](../01-network/08-ip-and-icmp.md)) sits after PREROUTING: traffic for one of the host's own addresses goes up to INPUT and a
socket; traffic for someone else is forwarded (only if `net.ipv4.ip_forward=1`) or dropped. `PREROUTING` is where
destination NAT happens (before routing, so the new destination is routed); `POSTROUTING` is where source NAT happens
(after routing, as the packet leaves). Everything below is rules attached to these hooks.

### iptables and nftables: rules in chains
Rules are organized into **tables** (`filter`: accept/drop; `nat`: rewrite addresses; `mangle`: alter headers/marks;
`raw`: bypass connection tracking) containing **chains** tied to hooks, each an ordered list of
`match -> target` rules; the first matching terminal target (`ACCEPT`, `DROP`, `REJECT`, `DNAT`, ...) wins, otherwise the chain's
default **policy** applies. **nftables** is the modern replacement (one syntax, atomic rule-set replacement, better performance);
`iptables` commands on current distros are often a compatibility layer over it.

```text
iptables -L -n -v                     # filter table, with counters
iptables -t nat -L -n -v              # NAT table
iptables -A INPUT -p tcp --dport 8080 -j DROP
nft list ruleset
```
**`DROP` vs `REJECT`** matters when you test: `DROP` silently discards, so the client sees a **hang then a timeout**;
`REJECT` sends back an immediate error (TCP RST via `--reject-with tcp-reset`, or ICMP port-unreachable), so the client sees
`Connection refused`/reset instantly. They exercise different code paths in your proxy (connect timeout vs refused —
[`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md), [`01-network/21-life-of-a-request.md`](../01-network/21-life-of-a-request.md)). Rules are not persistent across reboot unless saved, and
**order matters**: `-I` inserts at the top, `-A` appends at the bottom after perhaps a catch-all.

### Connection tracking: the kernel remembers flows
**conntrack** gives netfilter *state*: it tracks each flow (the 4-tuple plus state — `NEW`, `ESTABLISHED`, `RELATED`,
`INVALID`) in a table, including for UDP and ICMP via timeouts. Stateful rules rely on it
(`-m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT`: "allow replies to connections I allowed out"), and NAT relies on it to
remember mappings and un-translate the reply packets. `conntrack -L` lists entries; `conntrack -C` counts them;
`cat /proc/sys/net/netfilter/nf_conntrack_max` is the table capacity.

**Gotcha: conntrack-table-full.** A busy NAT box, a Kubernetes node, or any host with conntrack loaded and a proxy making
many short connections can fill the table; the kernel then **drops new connections** and logs `nf_conntrack: table full, dropping
packet` (`dmesg`). Symptoms are intermittent connection timeouts with healthy services and CPU. Remedies: raise
`nf_conntrack_max` (mind memory), lower timeouts (an idle `ESTABLISHED` TCP entry lasts 5 days by default!), reuse
connections ([`01-network/12-tcp.md`](../01-network/12-tcp.md)), or exempt traffic from tracking (`-t raw ... -j NOTRACK`). Compare
`conntrack -C` to `nf_conntrack_max` in monitoring ([`08-observability/02-metrics.md`](../08-observability/02-metrics.md)).

### NAT in practice: SNAT, MASQUERADE, DNAT
The concepts are in [`01-network/02-addressing.md`](../01-network/02-addressing.md); here is how they are made:

- **SNAT/MASQUERADE** (`POSTROUTING`): rewrite the *source* so replies come back to the gateway.
  `iptables -t nat -A POSTROUTING -s 172.17.0.0/16 -j MASQUERADE` is how containers reach the internet (the source becomes the host's IP).
- **DNAT** (`PREROUTING` for external traffic, `OUTPUT` for local): rewrite the *destination*.
  `iptables -t nat -A PREROUTING -p tcp --dport 8080 -j DNAT --to-destination 172.17.0.2:80` is `docker run -p 8080:80`.
  The packet reaches your container with its **original source IP intact** — but only if replies traverse the same NAT box; the
  "asymmetric routing broke my NAT" bug is the reply bypassing it ([`01-network/08-ip-and-icmp.md`](../01-network/08-ip-and-icmp.md)).
- **Kubernetes** `kube-proxy` implements Services as DNAT rules (iptables) or IPVS virtual servers: a Service's virtual IP is not any
  interface's address — it is a rewrite rule picking a pod IP per *connection*. This is a L4 load balancer inside the kernel. A client
  with a long-lived connection (HTTP/2, gRPC) therefore pins to one pod for its lifetime, the reason gRPC needs L7 balancing
  ([`05-http-stack/11-grpc.md`](../05-http-stack/11-grpc.md), [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)).

NAT hides the client: your proxy behind a SNAT'ing load balancer sees the balancer's IP, which is why the PROXY protocol and
`X-Forwarded-For` exist ([`01-network/20-proxy-protocol.md`](../01-network/20-proxy-protocol.md), [`01-network/15-http.md`](../01-network/15-http.md)).

### Transparent proxying: intercepting traffic not addressed to you
A service mesh sidecar or a forward proxy often must intercept connections that were addressed to *someone else*, without the client
knowing. Rules redirect them to a local port (`-j REDIRECT --to-ports 15001`, or `TPROXY` for UDP and to preserve addresses), and the
proxy recovers where the client was **really** going with `getsockopt(SO_ORIGINAL_DST)` on the accepted socket (after `REDIRECT`) or from
`getsockname` (under `TPROXY` with `IP_TRANSPARENT`). That original-destination lookup is how Istio/Envoy sidecars and `redsocks`-style tools
work ([`01-network/05-proxy-taxonomy.md`](../01-network/05-proxy-taxonomy.md)). It is also why a proxy's own outgoing traffic must be exempted from the redirect rule
(match on UID or a socket `mark`), or it loops into itself.

### Network namespaces, veth, bridges
Each **network namespace** ([`13-containers.md`](13-containers.md)) has its own interfaces, routes, iptables rules and conntrack table — a container's
`eth0` is one end of a **veth pair** whose other end is on a host **bridge** (software switch, [`01-network/07-link-layer.md`](../01-network/07-link-layer.md)). `ip netns add`, `ip link
add ... type veth`, `ip netns exec <ns> <cmd>` let you build isolated mini-networks by hand — the best way to test a proxy against
failures (drop, delay, loss) without touching your real network. Remember a listening socket belongs to *a namespace*: a proxy in the host
namespace listening on `127.0.0.1` is unreachable from a container's `127.0.0.1`.

### Traffic control: queues and netem
`tc` configures **queueing disciplines (qdiscs)** on an interface's output. `tc qdisc add dev eth0 root netem delay 100ms loss 1%` injects
latency and loss; `tbf`/`htb` shape bandwidth; `fq_codel` fights bufferbloat ([`01-network/13-tcp-reliability.md`](../01-network/13-tcp-reliability.md)). This is the fault-injection
tool of [`12-testing/03-chaos.md`](../12-testing/03-chaos.md). `netem` applies on **egress**; to affect traffic a host *receives* you apply it on the peer, or on an `ifb`
device.

### The sysctls a network service meets
`net.core.somaxconn` (the cap on any `listen` backlog; your `listen(1024)` is silently clamped, [`01-network/11-socket.md`](../01-network/11-socket.md)),
`net.ipv4.tcp_max_syn_backlog`, `net.ipv4.ip_local_port_range`, `net.ipv4.ip_forward`, `net.ipv4.tcp_tw_reuse`, `net.netfilter.nf_conntrack_max`,
`net.ipv4.conf.*.rp_filter` (drops packets whose source isn't reachable back via the arrival interface — a source of mysterious drops
with asymmetric routing). `sysctl -a | grep <name>` reads them; `sysctl -w` changes them until reboot; `/etc/sysctl.d/` persists. In a container
many are per-namespace; some need privileges.

## Practice

Build these in order.

1. Build two namespaces and a bridge by hand (`ip netns add`, `ip link add br0 type bridge`, two `veth` pairs,
   addresses, `ip link set ... up`) and ping between them. **Done when** the ping succeeds and you can point at the
   bridge's MAC table (`bridge fdb show`) and each namespace's routes and explain every entry.
2. In one namespace run [`labs/00-tcp-server`](../../labs/00-tcp-server); `curl` it from the other. Then `ip netns exec <server-ns> iptables -A INPUT -p tcp
   --dport <p> -j DROP`, and later replace it with `-j REJECT --reject-with tcp-reset`. **Done when** you have captured
   both with `tcpdump` ([`01-network/10-packet-capture-and-tools.md`](../01-network/10-packet-capture-and-tools.md)) and shown DROP = SYN with no reply (client hangs, then times
   out) vs REJECT = immediate RST (`Connection refused`).
3. Make a namespace a NAT router: enable `ip_forward`, add a MASQUERADE rule, and have a client in a private namespace reach a
   server on the host. **Done when** the server logs the *rewritten* source IP and `conntrack -L` shows the matching entry.
4. Add a DNAT rule publishing your proxy ([`labs/05-reverse-proxy`](../../labs/05-reverse-proxy)) on a different external port into a namespace. **Done when** the proxy
   logs the original client IP, and after you break reply symmetry on purpose (reply path bypasses the NAT box) you can show the
   resulting hang in `tcpdump`.
5. Add `tc qdisc ... netem delay 100ms loss 1%` on a veth and run a request loop through your proxy. **Done when** you can
   show p50/p99 latency and retry counts before and after, and tie them to the proxy's timeout/retry settings.
6. Fill conntrack deliberately in a namespace (a connection loop with a tiny `nf_conntrack_max`). **Done when** `dmesg` shows
   `table full, dropping packet` and you have recorded what the client observed.
7. Use `iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports <port>` with a scratch program that reads
   `SO_ORIGINAL_DST` via `getsockopt` (`libc`/`nix`). **Done when** it prints the real destination of a redirected `curl`,
   and you have exempted the proxy's own UID so it does not redirect to itself.
