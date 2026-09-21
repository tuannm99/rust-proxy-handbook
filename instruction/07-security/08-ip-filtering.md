# IP Filtering

## What to learn
### Allow/deny lists and CIDR matching
Filter by source IP against a list of CIDR blocks (e.g. `10.0.0.0/8` for
internal, or a vendor's published egress ranges). Match by computing
whether the IP's bits, masked by the CIDR prefix length, equal the
network's bits — don't reinvent this by hand, use `ipnet` or std's
`Ipv4Addr`/`Ipv6Addr` bit operations, and always support both v4 and v6
(a v4-only allowlist is trivially bypassed by an attacker with v6
connectivity if the listener accepts both).

```rust
fn ip_in_cidr(ip: std::net::Ipv4Addr, network: std::net::Ipv4Addr, prefix_len: u8) -> bool {
    let mask = u32::MAX.checked_shl(32 - prefix_len as u32).unwrap_or(0);
    u32::from(ip) & mask == u32::from(network) & mask
}
```
Note the `checked_shl` — shifting a `u32` by 32 is undefined-behavior
territory in C and a panic in debug Rust, and `/0` is a real CIDR people
write. That single edge case is why "just use `ipnet`" is the right call
in production code.

### The IPv4-mapped IPv6 trap
A dual-stack listener (bound to `::` with `IPV6_V6ONLY` off, which is the
common default) reports an IPv4 client's address as an IPv4-*mapped* IPv6
address: `::ffff:10.0.0.1`, not `10.0.0.1`. Compare that against your
`10.0.0.0/8` rule as a v6 address and it does not match — silently.

Which direction that breaks depends on the list, and both are bad: an
*allowlist* stops matching and locks out every legitimate IPv4 client; a
*denylist* stops matching and lets every banned IPv4 client straight
through. The second is a security hole that no test with a v6 client will
ever reveal.

```rust
// normalize once, at the boundary, before any rule is consulted
fn canonical(ip: std::net::IpAddr) -> std::net::IpAddr {
    match ip {
        std::net::IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => std::net::IpAddr::V4(v4),
            None => std::net::IpAddr::V6(v6),
        },
        v4 => v4,
    }
}
```
Gotcha: use `to_ipv4_mapped()`, not the older `to_ipv4()`. The latter also
converts IPv4-*compatible* addresses (`::1.2.3.4`, a deprecated format)
and, notoriously, maps `::1` to `0.0.0.1` — so a loopback connection can
come out the other side looking like some arbitrary public address.

### Matching against large lists
A linear scan over CIDRs is fine for the dozen entries in a hand-written
config, and wrong for a threat-intel feed with 100k blocks evaluated on
every request. The right structure is a prefix trie over address bits —
which is exactly `13-algorithms/radix-tree.md`'s structure with a fixed
32- or 128-bit key — giving O(prefix length) lookup independent of list
size. The `ip_network_table` crate implements this; the kernel's own FIB
does the same thing for routing.

Gotcha: rebuild the trie on config reload and swap it atomically
(`06-proxy/07-service-discovery.md`'s `ArcSwap` pattern) rather than mutating
it under a lock — request-path reads should never block on a list update.

### Trusting forwarded-for headers only from known proxies
`X-Forwarded-For` (or the PROXY protocol header, see
`01-network/08-proxy-protocol.md`) is client-supplied data unless you strip
and re-set it yourself at a trust boundary. If your proxy blindly trusts
whatever `X-Forwarded-For` value arrives, any client can claim to be
`127.0.0.1` or an allowlisted internal IP and bypass IP filtering entirely.

The correct model: only trust forwarded-address headers when the
*immediate TCP peer* is itself a known, trusted upstream load balancer
(check the actual socket peer address against a trusted-proxies list);
otherwise, ignore the header and use the real peer address, or strip the
header entirely before it reaches your filtering logic.

### Parsing X-Forwarded-For correctly
The header is a comma-separated list that each hop *appends* to, so it
reads `client, proxy1, proxy2` — and every entry to the left of your
trusted hops was written by someone you don't trust. An attacker sends
`X-Forwarded-For: 127.0.0.1` and your CDN appends the real address,
producing `127.0.0.1, 203.0.113.9`. Taking the **leftmost** entry — the
obvious reading of "the original client" — hands the attacker their
chosen value.

The correct algorithm is **rightmost-untrusted**: walk the list from the
right, skipping entries that are in your trusted-proxy set; the first
entry that isn't trusted is the real client. Everything left of it is
attacker-controlled and must be discarded, not logged as fact.

Gotcha: also handle the malformed cases, because they are how the parser
gets bypassed — an entry that isn't a valid IP, an IPv6 address with a
port in brackets (`[2001:db8::1]:443`), whitespace variants, and a
50-entry list designed to make you allocate. Reject rather than guess, and
cap the number of entries you'll parse.

Gotcha: RFC 7239's `Forwarded: for=...;proto=...;by=...` header is the
standardized version of the same thing, with its own quoting rules. If you
accept both, make sure they can't disagree — an attacker supplying one and
the CDN supplying the other is another parser differential
(`05-request-smuggling.md`).

### IP spoofing caveats
Source IP spoofing on TCP is hard in practice (the 3-way handshake means a
spoofed-source SYN can't complete a real connection without seeing the
SYN-ACK), which is why IP allowlisting is meaningfully strong for
TCP-based protocols — but it's the *forwarded-header* trust boundary above
that's the actually-exploitable gap in most real incidents, not raw IP
spoofing.

### What IP filtering fundamentally cannot do
Worth being clear-eyed about, because IP blocking is often reached for as
a general-purpose defense and is weak as one:
- **Addresses are shared.** Carrier-grade NAT puts thousands of mobile
  users behind one address; blocking it blocks a city. University and
  corporate networks are the same.
- **Addresses are cheap.** A cloud account rents a new one for cents, and
  residential-proxy services rent millions of real consumer IPs
  specifically to defeat this control.
- **Addresses are reassigned.** A block you added today lands on an
  unrelated customer next month, and nobody remembers why the rule exists.

So: allowlists are strong (a short, known set of peers, e.g. admin
endpoints or partner integrations), denylists are weak and temporary.
Treat a denylist entry as a rate-limiting or incident-response tool with a
TTL, not as a permanent security control, and pair it with
`07-security/07-ratelimit.md` which degrades far more gracefully against
shared addresses.

### Dynamic bans, and bounding them
The useful form of a denylist is generated, not written: a client that
trips the rate limiter or WAF repeatedly gets a temporary ban, which
avoids re-running expensive checks on traffic you've already judged.

Two constraints make this safe. Every entry needs a **TTL** (minutes to
hours), both because addresses are shared and because a permanent
autoban list is an eventual self-inflicted outage. And the table must be
**bounded** — it is keyed by attacker-controlled data, so an unbounded map
is the memory-exhaustion vector described in
`13-algorithms/count-min-sketch.md`. Cap it and evict (LRU, or oldest-TTL
first) rather than growing.

### Where to enforce: proxy or kernel
By the time your proxy evaluates a rule, it has completed a TCP handshake
and usually a TLS handshake — the expensive parts — for a peer you're
about to reject. That's fine for policy decisions on normal traffic and
useless against a flood, where the cost per rejected connection is exactly
what the attacker is spending your budget on.

Volumetric blocking belongs lower: `nftables`/`ipset` in the kernel, or
XDP at the driver (`16-kernel/10-xdp.md`), where a packet is dropped before
the stack allocates a socket. The proxy's role is to *decide* (it has the
application context) and push the decision down, which is exactly the
feedback loop `07-security/09-ddos.md` and `labs/17-ebpf` build.

## Practice
Build these in order.

1. In `proxy`, implement CIDR allow/deny against the real TCP peer address
   using `ipnet`, for both v4 and v6. **Done when** a `/0` rule doesn't
   panic and per-family rules match correctly.
2. Bind a dual-stack listener and connect over IPv4. **Done when** you've
   seen the peer address arrive as `::ffff:...` and confirmed your v4
   rules silently fail to match — then add `canonical()` normalization and
   confirm they match.
3. Add `trusted_proxies` and the rightmost-untrusted XFF algorithm.
   **Done when** a request from an untrusted peer carrying
   `X-Forwarded-For: 127.0.0.1` is filtered on its real address, and a
   request through a trusted proxy with `1.2.3.4, <trusted>` resolves to
   `1.2.3.4`.
4. Fuzz the XFF parser with malformed input (non-IPs, bracketed v6 with
   ports, 10k entries, embedded whitespace). **Done when** none of it
   panics, allocates unboundedly, or produces a trusted verdict — see
   `12-testing/02-fuzzing.md`.
5. Swap linear CIDR matching for a prefix trie and load a 100k-entry list.
   **Done when** per-request match latency is flat between a 10-entry and
   a 100k-entry list, and a config reload swaps the table without blocking
   request-path reads.
6. Add TTL'd dynamic bans driven by rate-limit violations, with a bounded
   table. **Done when** a banned client is rejected before the rate
   limiter runs, the ban expires on schedule, and filling the table with
   1M synthetic addresses plateaus in memory instead of growing.
7. (Stretch) Wire PROXY protocol v2 parsing (`01-network/08-proxy-protocol.md`)
   as a structured alternative to XFF. **Done when** the real client IP is
   recovered from the binary header and a connection *without* the
   expected header on a PROXY-protocol listener is rejected rather than
   parsed as HTTP.
