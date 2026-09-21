# DNS

Recursive vs authoritative, TTL, caching.

## What to learn

### Recursive vs authoritative resolution
A recursive resolver (e.g. your OS stub resolver, or a public resolver like
1.1.1.1) walks the chain root -> TLD -> authoritative nameserver on your
behalf and caches the result. An authoritative server is the source of
truth for a zone and answers only for records it owns. A proxy usually
talks to a recursive resolver, not authoritative servers directly — but
knowing the chain matters when a DNS change "isn't propagating" and you
need to know which layer is stale.

### Record types that matter to a proxy
`A`/`AAAA` (IPv4/IPv6 upstream addresses), `CNAME` (aliasing, can't coexist
with other records at the same name), `SRV` (host+port+priority+weight —
this is what a lot of service discovery is built on), `TXT` (used for
ACME DNS-01 challenges when automating cert issuance). A proxy resolving
upstream hostnames typically only cares about `A`/`AAAA` and sometimes `SRV`.

### TTL and caching
TTL tells every downstream cache (OS, resolver, your own proxy) how long a
record may be reused. A proxy resolving upstream hostnames itself needs its
own cache with TTL-respecting expiry — reusing a stale IP after a backend's
address changes silently sends traffic into a void. Production gotcha: some
recursive resolvers or client libraries clamp or ignore very low TTLs
(sub-5s), which breaks fast-failover DNS-based deployments — don't assume a
1-second TTL actually gets you 1-second failover in practice.

### Resolution in Rust: blocking vs async
The libc `getaddrinfo` used by `std::net::ToSocketAddrs` is *blocking* and
does its own OS-level caching/config parsing (`/etc/resolv.conf`,
`/etc/hosts`) — calling it directly inside an async task stalls the
executor thread. Options: run it via `tokio::task::spawn_blocking`, or use
a pure-async resolver crate like `hickory-resolver` that speaks DNS
directly over UDP/TCP and gives you TTL-aware caching and control over which
nameservers you query.

```rust
// blocking resolution done off the async executor thread
let addrs = tokio::task::spawn_blocking(|| {
    "backend.internal:8080".to_socket_addrs()
})
.await??;
```

### DNS as a service-discovery mechanism
Many systems (Kubernetes headless Services, Consul DNS interface) expose
their service registry *as* DNS — an `A` record that returns multiple IPs,
or changes over time as pods/instances come and go. A proxy that re-resolves
periodically and swaps its upstream set atomically gets basic dynamic
service discovery almost for free; see `06-proxy/07-service-discovery.md` for
how that swap needs to happen without dropping in-flight requests.

## Practice

1. Use `dig +trace example.com` to watch the root -> TLD -> authoritative
   chain manually, then compare against one `dig example.com` (which used a
   recursive resolver's cache).
2. Write a small standalone Rust program using `hickory-resolver` (async)
   that resolves a hostname to multiple `A` records and prints their TTLs.
3. In `labs/05-reverse-proxy`, resolve upstream hostnames instead
   of hardcoding IPs, and re-resolve on a timer respecting the record's TTL.
4. Simulate a backend IP change: point a hostname at IP A, start your
   proxy, then change DNS to IP B. Measure how long your proxy takes to
   notice, and whether any requests failed during the switch.
5. Read `06-proxy/07-service-discovery.md` and note which parts of it your
   DNS-based approach in step 3 already satisfies vs still needs.
