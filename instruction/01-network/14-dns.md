# DNS

Names to addresses: the hierarchy, what a lookup actually does on your
machine, the wire format, caching, and why a long-lived proxy must treat DNS
as live data.

## What to learn

### A name is a path through a tree
`www.example.com.` is read right to left: the root (`.`), the top-level
domain (`com`), the domain (`example`), a host label (`www`). The tree is
cut into **zones**, each run by an **authoritative** server set. A parent
zone **delegates** a child with `NS` records ("`example.com` is served by
`ns1.example.net`"), plus **glue** `A` records when the nameserver lives
inside the very zone it serves (otherwise you'd need DNS to find DNS). The
13 root server identities and the TLD servers hold no host records, only
delegations: that is the whole reason the system scales.

### Recursive vs authoritative resolution
A **recursive resolver** (your ISP's, or a public one like 1.1.1.1) walks the
chain on your behalf — ask a root server, get the `com` servers; ask those, get
`example.com`'s servers; ask those, get the answer — and caches every step.
An **authoritative** server is the source of truth for a zone and answers only
for what it owns. Your program talks to a recursive resolver; it almost never
talks to authoritative servers directly. Knowing the chain matters when a DNS
change "isn't propagating": you need to know which layer is stale.

### What happens when your program resolves a name
Your code calls `getaddrinfo` (what `ToSocketAddrs` and `curl` use). On Linux
that consults, in the order set by `/etc/nsswitch.conf`: `/etc/hosts`, then DNS.
For DNS it reads `/etc/resolv.conf`: the `nameserver` addresses (often a local
stub like `127.0.0.53` from `systemd-resolved`, or the cluster DNS in a
container), the `search` domains, and `options ndots:N`.

The **search list** trips people up: a name with fewer than `ndots` dots is
tried with each search suffix *first*. Kubernetes sets `ndots:5`, so resolving
`api.example.com` (2 dots) first tries `api.example.com.default.svc.cluster.local`,
`...svc.cluster.local`, `...cluster.local` — three wasted queries (each an
`NXDOMAIN`) before the real one, for every cold lookup. A trailing dot
(`api.example.com.`) marks a name as absolute and skips the search list. Also
note: glibc's `getaddrinfo` has **no cache of its own** (every call is a
query unless a stub/`nscd` caches it), and musl (Alpine) behaves differently
in details such as search-domain handling — "works on my laptop, fails in the
container" is often this.

### On the wire: one query, one response
A DNS message is a header (16-bit ID, flags, counts), a question (name, type,
class) and answer/authority/additional record sections. Queries go over **UDP
port 53** ([`09-udp.md`](09-udp.md)); the 16-bit ID matches response to query (and, with the
source port, is the only thing guarding against spoofed replies — hence random
source ports). A response whose answer doesn't fit sets the **TC (truncated)**
flag and the client **retries over TCP** — as it also does for large
DNSSEC/EDNS responses. **EDNS0** raises the safe UDP size above the old
512 bytes. The response code tells you what happened:

- `NOERROR` with answers: success. `NOERROR` with **zero** answers (**NODATA**)
  means the name exists but has no record of that type — typical when a host
  has only `A` and you ask for `AAAA`.
- `NXDOMAIN`: the name does not exist.
- `SERVFAIL`: the resolver couldn't get an answer (broken delegation, DNSSEC
  failure, upstream timeout). `REFUSED`: it won't answer you.

### Record types that matter to a proxy
`A`/`AAAA` (IPv4/IPv6 upstream addresses), `CNAME` (alias; a name with a `CNAME`
can't carry other records, which is why you can't `CNAME` a zone apex),
`SRV` (host + port + priority + weight — what a lot of service discovery is
built on), `NS`/`SOA` (delegation and zone metadata), `TXT` (used for ACME
DNS-01 challenges when automating certificate issuance,
[`19-tls.md`](19-tls.md)), `PTR` (reverse lookups: IP to name; logs and mail
use it). A proxy resolving upstream hostnames typically only cares about
`A`/`AAAA` and sometimes `SRV`.

### TTL and caching — positive and negative
TTL tells every downstream cache (OS, resolver, your own proxy) how long a record
may be reused. A proxy resolving upstream hostnames itself needs its own cache with
TTL-respecting expiry — reusing a stale IP after a backend's address changes
silently sends traffic into a void. **Negative answers are cached too**: an
`NXDOMAIN` is remembered for the zone's `SOA` minimum, so a record you *just
created* may keep "not existing" for minutes at resolvers that already looked it
up. Production gotcha: some recursive resolvers or client libraries clamp or
ignore very low TTLs (sub-5s), which breaks fast-failover DNS-based
deployments — don't assume a 1-second TTL gets you 1-second failover. And
nothing already connected re-resolves: a pooled connection to the old IP keeps
using it ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)).

### Resolution in Rust: blocking vs async
The libc `getaddrinfo` used by `std::net::ToSocketAddrs` is *blocking* and does
its own OS-level config parsing (`/etc/resolv.conf`, `/etc/hosts`) — calling it
directly inside an async task stalls the executor thread. Options: run it via
`tokio::task::spawn_blocking` (`tokio::net::lookup_host` does exactly this),
or use a pure-async resolver crate like `hickory-resolver` that speaks DNS
directly over UDP/TCP and gives you TTL-aware caching and control over which
nameservers you query.

```rust
// blocking resolution done off the async executor thread
let addrs = tokio::task::spawn_blocking(|| {
    "backend.internal:8080".to_socket_addrs()
})
.await??;
```

A resolver answer can contain many addresses of both families. Connecting well
means trying them sensibly — **Happy Eyeballs**: start IPv6, and if it hasn't
connected within ~250 ms, race IPv4 — so a broken `AAAA` path doesn't cost every
request a long timeout. A bare `TcpStream::connect(name)` tries addresses
sequentially, each with the full connect timeout.

### DNS as a service-discovery and load-distribution mechanism
Many systems (Kubernetes headless Services, Consul DNS interface) expose their
service registry *as* DNS — an `A` record that returns multiple IPs, or changes
over time as pods/instances come and go. A proxy that re-resolves periodically
and swaps its upstream set atomically gets basic dynamic service discovery
almost for free; see [`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md) for how that swap needs
to happen without dropping in-flight requests. **DNS round-robin** (several `A`
records, order rotated) spreads clients crudely: no health awareness, cached
answers pin a client for the TTL, and a few big resolvers can send most
traffic to one address. **GeoDNS** returns different answers by client location.
Both are coarse tools compared to a real load balancer
([`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)).

### Security: DNS is unauthenticated by default
Plain DNS is cleartext and unsigned: anyone on the path can read it or forge
answers (cache poisoning). **DNSSEC** signs records so a validating resolver can
detect forgery (not widely enforced end-to-end); **DoT/DoH** encrypt the
client-to-resolver hop. For a proxy the sharper danger is **DNS rebinding and
SSRF**: if you resolve a user-influenced hostname, an attacker can point it at
`127.0.0.1` or `169.254.169.254`, or make it resolve to a public address when
you *validate* and a private one when you *connect*. Defend by validating the
**resolved IP**, and connecting to that exact IP, not re-resolving
([`07-security/04-normalization.md`](../07-security/04-normalization.md)).

## Practice

1. Use `dig +trace example.com` to watch the root -> TLD -> authoritative chain,
   then compare against `dig example.com` (a recursive resolver's cached
   answer; watch the TTL count down across two runs) and `dig example.com AAAA`.
   Find an `NXDOMAIN` and a NODATA response (`dig nonexistent.example.com`,
   `dig www.example.com TXT`) and read the `status:` and `ANSWER:` fields.
2. Run `cat /etc/resolv.conf /etc/nsswitch.conf`, then
   `strace -f -e trace=openat,connect,sendto getent hosts example.com` and
   identify: which files are read, which nameserver is contacted, and the query
   packets. Capture the same lookup with `sudo tcpdump -n -i any port 53`
   and read the query ID, type and response code.
3. Compare `getent hosts name` with `dig name` for `localhost`, a name in
   `/etc/hosts`, and a short name that depends on a search domain; explain every
   difference. Count the queries a single lookup makes with `ndots:5` by
   adding it to a test `resolv.conf`.
4. Write a small standalone Rust program using `hickory-resolver` (async) that
   resolves a hostname to multiple `A`/`AAAA` records, prints their TTLs, and
   shows what changes on a second call within the TTL.
5. In [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), resolve upstream hostnames instead of
   hardcoding IPs, and re-resolve on a timer respecting the record's TTL;
   simulate a backend IP change (hostname -> IP A via `/etc/hosts` or a local
   `dnsmasq`, then IP B) and measure how long your proxy takes to notice and
   whether any requests failed during the switch.
6. Read [`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md) and note which parts of it your
   DNS-based approach in step 5 already satisfies and which it still needs.
