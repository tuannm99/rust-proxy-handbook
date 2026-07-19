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

### Trusting forwarded-for headers only from known proxies
`X-Forwarded-For` (or the PROXY protocol header, see
`01-network/proxy-protocol.md`) is client-supplied data unless you strip
and re-set it yourself at a trust boundary. If your proxy blindly trusts
whatever `X-Forwarded-For` value arrives, any client can claim to be
`127.0.0.1` or an allowlisted internal IP and bypass IP filtering entirely.

The correct model: only trust forwarded-address headers when the
*immediate TCP peer* is itself a known, trusted upstream load balancer
(check the actual socket peer address against a trusted-proxies list);
otherwise, ignore the header and use the real peer address, or strip the
header entirely before it reaches your filtering logic.

### IP spoofing caveats
Source IP spoofing on TCP is hard in practice (the 3-way handshake means a
spoofed-source SYN can't complete a real connection without seeing the
SYN-ACK), which is why IP allowlisting is meaningfully strong for
TCP-based protocols — but it's the *forwarded-header* trust boundary above
that's the actually-exploitable gap in most real incidents, not raw IP
spoofing.

## Practice
1. In `proxy`, implement CIDR-based allow/deny
   checking against the real TCP peer address using `ipnet` (or the raw
   bit-math above) for both IPv4 and IPv6.
2. Add a `trusted_proxies` list; only read `X-Forwarded-For` when the
   direct peer is in that list, otherwise use the raw peer IP.
3. Write a test simulating a client sending a spoofed
   `X-Forwarded-For: 127.0.0.1` from an untrusted peer and confirm it's
   ignored.
4. (Stretch) Wire real PROXY protocol v2 parsing (see
   `01-network/proxy-protocol.md`) as an alternative, more structured
   source of the "real client IP" than `X-Forwarded-For`.
