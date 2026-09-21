# XDP

eXpress Data Path: run an eBPF program (`16-kernel/09-ebpf.md`) in the NIC
driver, before the kernel builds any per-packet state. The cheapest place
to drop a packet on Linux.

## What to learn

### Where XDP sits, and why that makes it fast
A normal packet's journey allocates an `sk_buff` (a few hundred bytes of
metadata), walks the netfilter hooks, traverses the TCP/IP stack, and
eventually reaches a socket. XDP runs *before* the `sk_buff` allocation, on
the raw DMA'd frame in the driver's receive path.

Dropping there costs a bounds check and a return code. Dropping at
iptables costs the `sk_buff` allocation plus the netfilter traversal;
dropping in your proxy costs all of that plus a wakeup, a syscall, and
a TCP handshake. The measured gap is roughly an order of magnitude per
step — XDP sustains tens of millions of packets per second per core, where
iptables tops out in the low millions.

That is the entire value proposition for `07-security/09-ddos.md`: under a
volumetric flood, the cost of *rejecting* traffic is what determines
whether you survive.

### The five return codes
An XDP program's verdict is its return value:

- `XDP_DROP` — discard immediately. The one that matters for DDoS.
- `XDP_PASS` — continue into the normal network stack.
- `XDP_TX` — bounce the packet back out the same interface (after
  modification). This is how an XDP load balancer replies or redirects.
- `XDP_REDIRECT` — send to another interface or into an AF_XDP socket.
- `XDP_ABORTED` — drop and raise a tracepoint; means a bug, not a policy.

Gotcha: `XDP_ABORTED` is what an unhandled error path returns, and it drops
the packet silently as far as your application is concerned. Traffic
vanishing with no log is the classic XDP debugging experience — watch the
`xdp:xdp_exception` tracepoint before assuming the network is at fault.

### Three attach modes, with very different performance
- **Native** — the driver implements XDP hooks directly. The real thing.
  Requires driver support (ixgbe, mlx5, i40e, virtio-net, veth; notably
  *not* every NIC).
- **Offloaded** — runs on the NIC hardware itself. Fastest, near-zero host
  CPU, supported by very few cards (Netronome).
- **Generic (SKB mode)** — a fallback that runs after `sk_buff` allocation,
  so it gives up the entire performance advantage. Works everywhere.

Gotcha: generic mode is what you silently get when the driver does not
support native XDP, and it "works" — your program loads, your tests pass,
and the performance benefit is absent. Always check which mode you actually
attached in, especially in a VM or container where the interface is often
veth or a virtualized NIC.

### Writing the parser: everything is a bounds check
You get a pointer to the frame start and one to its end, and the verifier
demands proof that every read lies between them. Parsing to the IP header
means checking room for the Ethernet header, then the IP header, each
before touching it:

```
if (data + sizeof(ethhdr) > data_end) return XDP_PASS;
// only now may you read eth->h_proto
if (data + sizeof(ethhdr) + sizeof(iphdr) > data_end) return XDP_PASS;
// only now may you read ip->saddr
```

Returning `XDP_PASS` on a too-short packet rather than `XDP_DROP` is the
right default: let the kernel's stack handle malformed input, since your
job is policy, not validation.

Gotcha: VLAN tags, IPv6 extension headers, and IP options all shift the
offsets. A parser assuming a fixed 14-byte Ethernet + 20-byte IPv4 prefix
misreads any tagged or optioned packet — and an attacker who notices can
add a VLAN tag to bypass your filter entirely. Parse the chain, or
explicitly `XDP_PASS` anything that does not match the exact shape you
handle.

### What XDP can and cannot do for a proxy
It **can**: drop by source IP against an `LPM_TRIE` of CIDRs, rate-limit
per source with per-CPU counters, drop malformed or unexpected-protocol
packets, and enforce a SYN rate — all before the kernel spends anything.

It **cannot**: see TCP streams (it sees individual packets, not reassembled
data), inspect HTTP (a request may span packets, and TLS means it is
encrypted anyway), or make decisions that need application state. Anything
requiring the request is the proxy's job, by definition.

The correct architecture is layered: XDP drops what is provably hostile at
line rate, and everything else passes through to the proxy's own
accept-rate limiting and HTTP-layer defenses (`07-security/07-ratelimit.md`,
`07-security/06-waf.md`). Blocklists come *from* the proxy — which sees the
requests — and are pushed *into* XDP maps from user space, which is the
feedback loop that makes both layers useful.

Gotcha: a stale or overly broad blocklist entry is now dropping traffic at
a layer with no logging and no application visibility. Always give
XDP-installed blocks a TTL that user space refreshes, so a bug expires
instead of persisting until someone notices.

## Practice
1. In `labs/17-ebpf`, attach an XDP program that counts and passes all
   packets. Confirm with `bpftool net show` which attach mode you got — if
   it is generic, find out why.
2. Add Ethernet + IPv4 parsing with correct bounds checks; drop packets
   from one hardcoded source IP and verify with `ping` from that host.
3. Send a VLAN-tagged packet at your parser and confirm it is *not*
   misparsed — then handle or explicitly pass it.
4. Replace the hardcoded IP with an `LPM_TRIE` map populated from user
   space; add and remove CIDRs at runtime with the program still attached.
5. Benchmark drop rate for the same blocklist implemented three ways: XDP,
   iptables, and in `proxy`'s accept loop. Use a packet generator and
   compare both throughput and host CPU.
6. Close the loop: have `proxy` detect an abusive source via
   `07-security/07-ratelimit.md` and push it into the XDP map with a TTL;
   confirm the traffic stops reaching user space, and that the entry
   expires on its own.
