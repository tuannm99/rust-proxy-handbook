# Learning Roadmap

The phases below map 1:1 to the numbered directories. Each phase lists what
you should be able to *do*, not just recite, before moving to the next one.

1. **Networking** (`01-network/`) — read a TCP or TLS packet capture in
   Wireshark and explain what's happening; explain why HTTP/2 needs one TCP
   connection where HTTP/1.1 needed six. If terms like "port," "packet,"
   "handshake," or "certificate" don't already have a precise meaning,
   start with `01-network/01-fundamentals.md` — it's the one group of
   files in this handbook written as a from-scratch primer rather than
   assuming a baseline.
2. **Linux** (`02-linux/`) — explain the difference between level-triggered
   and edge-triggered epoll from having hit the edge-triggered EAGAIN bug
   yourself in the raw-epoll exercise in `02-linux/06-epoll.md`. Same note
   as above: `02-linux/01-fundamentals.md` first if "syscall," "file
   descriptor," "kernel space," or "container" aren't already precise to
   you.
3. **Rust** (`03-rust/`) — explain why `Pin` exists without reciting the
   docs; know when to reach for `Arc<Mutex<T>>` vs `Arc<RwLock<T>>` vs an
   atomic.
4. **Async runtime** (`04-runtime/`) — explain what `.await` desugars to and
   what happens on the executor thread when a task returns `Poll::Pending`.
5. **HTTP** (`05-http-stack/`) — hand-parse an HTTP/1.1 request in
   `labs/01-http-parser` and get the Content-Length/chunked framing right.
6. **Reverse Proxy** (`06-proxy/`) — forward a request to one of N upstreams,
   survive one upstream going down without dropping client requests.
7. **Security** (`07-security/`) — explain a request-smuggling attack well
   enough to defend against it, not just name it.
8. **Observability** (`08-observability/`) — answer "what's our p99 latency
   right now and which upstream is it coming from" using your own proxy's
   logs/metrics/traces.
9. **Architecture** (`09-architecture/`) — reload config and drain
   connections on shutdown without dropping in-flight requests.
10. **Production** (`proxy/00-README.md`) — run `12-testing/`'s load
    test and chaos exercises against your own proxy and survive them.

## Read + code map

One row per `labs/` crate, in build order: read its handbook references
first, then implement it. This table mirrors each crate's own README —
it's the one-page version so the whole `labs/` → `proxy/` path is visible
without opening 18 files.

| Lab | Read first | Then code |
| --- | --- | --- |
| `labs/00-tcp-server` | `01-network/01-fundamentals.md`, `02-linux/01-fundamentals.md` (if needed), `01-network/07-socket.md`, `01-network/08-tcp.md`, `03-rust/01-ownership.md`, `03-rust/05-async.md`, `04-runtime/01-tokio.md`, `02-linux/06-epoll.md` | TCP echo server |
| `labs/01-http-parser` | `05-http-stack/01-parser.md`, `07-security/05-request-smuggling.md` | hand-written HTTP/1.1 parser, no hyper |
| `labs/02-http-server` | `05-http-stack/01-parser.md`, `05-http-stack/04-keepalive.md`, `05-http-stack/02-hop-by-hop-headers.md`, `01-network/10-http.md`, `01-network/11-http2.md` | hyper/hyper-util plain HTTP server |
| `labs/03-router` | `05-http-stack/03-router.md` | method+path routing |
| `labs/04-static-server` | `05-http-stack/05-static.md`, `02-linux/10-zerocopy.md` | streaming static files |
| `labs/05-reverse-proxy` | `06-proxy/01-upstream.md`, `06-proxy/03-healthcheck.md`, `06-proxy/04-outlier-detection.md`, `06-proxy/05-retry.md`, `06-proxy/06-circuit-breaker.md`, `06-proxy/07-service-discovery.md`, `01-network/10-http.md` | hyper client forwarding to an upstream pool |
| `labs/06-load-balancer` | `06-proxy/02-load-balancer.md`, `13-algorithms/smooth-wrr.md`, `13-algorithms/rendezvous-hash.md`, `13-algorithms/maglev.md` | RR/least-conn/consistent-hash/smooth-WRR/Maglev |
| `labs/07-tls` | `01-network/13-tls.md` | tokio-rustls termination, ALPN |
| `labs/08-http2` | `01-network/11-http2.md` | multiplexing/flow-control specifics |
| `labs/09-http3` | `01-network/12-http3.md`, `19-reading-source/quinn/` | QUIC via `quinn` |
| `labs/10-cache` | `05-http-stack/07-cache.md`, `05-http-stack/08-cache-stampede.md`, `13-algorithms/lru.md`, `13-algorithms/lfu.md`, `13-algorithms/arc.md`, `13-algorithms/tinylfu.md` | HTTP response caching + eviction policy |
| `labs/11-rate-limit` | `07-security/07-ratelimit.md`, `07-security/11-load-shedding.md`, `13-algorithms/token-bucket.md`, `13-algorithms/sliding-window.md`, `13-algorithms/leaky-bucket.md` | token bucket / sliding window / leaky bucket |
| `labs/12-waf` | `07-security/06-waf.md`, `07-security/04-normalization.md`, `13-algorithms/aho-corasick.md` | rule-based filtering |
| `labs/13-hot-reload` | `09-architecture/03-config.md` | config reload without dropping connections |
| `labs/14-plugin` | `09-architecture/02-plugin.md` | request/response middleware |
| `labs/15-prometheus` | `08-observability/02-metrics.md` | metrics export |
| `labs/16-opentelemetry` | `08-observability/03-tracing.md` | distributed tracing export |
| `labs/17-ebpf` | `16-kernel/09-ebpf.md`, `16-kernel/10-xdp.md`, `07-security/09-ddos.md`, `07-security/10-slowloris.md` | XDP/eBPF packet filtering |
| `proxy/` | `01-network/13-tls.md`, `01-network/14-proxy-protocol.md`, `07-security/*.md`, `08-observability/*.md`, `09-architecture/*.md` (see `proxy/00-README.md`) | the final L7 proxy, combining every lab above |

Every numbered directory also has a `00-README.md` index listing its files
with one-line descriptions and a suggested reading order — start there
when you enter a phase, rather than guessing from filenames.

Paths above are relative to `instruction/` unless prefixed `labs/` or
`proxy/`. If a crate's own README ever drifts from this table, the
crate's README wins — update this table to match, not the other way
around.

## Deep-dive layer (13-20)

`13-algorithms/` through `20-reference/` are not a phase 11+ to work
through in order — they're foundations pulled in *from* the phases above
as you need them (e.g. implementing Maglev in phase 6 sends you to
`13-algorithms/maglev.md`). The one exception is `19-reading-source/`,
which is worth returning to after phase 10: reading `pingora`'s source
once you've built your own proxy will make far more sense than reading it
cold.

### If you want to read the deep-dive layer straight through anyway

The pull-it-in-when-needed order above is the default, but if you'd
rather work through `13-algorithms/` and `14-memory/` end to end once
(most of it is short, and having seen it once makes the "as needed"
callouts land faster later), this is a reasonable internal order — each
row only depends on rows above it:

| # | File | Depends on |
| --- | --- | --- |
| 1 | `13-algorithms/hashmap.md` | — |
| 2 | `13-algorithms/dfa.md` | — |
| 3 | `13-algorithms/fsm.md` | `dfa.md` |
| 4 | `13-algorithms/trie.md` | — |
| 5 | `13-algorithms/radix-tree.md` | `trie.md` |
| 6 | `13-algorithms/ring-buffer.md` | — |
| 7 | `13-algorithms/heap.md` | — |
| 8 | `13-algorithms/priority-queue.md` | `ring-buffer.md`, `heap.md` |
| 9 | `13-algorithms/skiplist.md` | — |
| 10 | `13-algorithms/slab.md` | — |
| 11 | `13-algorithms/lru.md` | `slab.md` |
| 12 | `13-algorithms/lfu.md` | — |
| 13 | `13-algorithms/arc.md` | `lru.md`, `lfu.md` |
| 14 | `13-algorithms/count-min-sketch.md` | — |
| 15 | `13-algorithms/bloom-filter.md` | — |
| 16 | `13-algorithms/hyperloglog.md` | — |
| 17 | `13-algorithms/tinylfu.md` | `count-min-sketch.md`, `bloom-filter.md`, `lru.md` |
| 18 | `13-algorithms/token-bucket.md` | — |
| 19 | `13-algorithms/sliding-window.md` | — |
| 20 | `13-algorithms/leaky-bucket.md` | `token-bucket.md` |
| 21 | `13-algorithms/consistent-hash.md` | — |
| 22 | `13-algorithms/rendezvous-hash.md` | `consistent-hash.md` |
| 23 | `13-algorithms/maglev.md` | `consistent-hash.md` |
| 24 | `13-algorithms/smooth-wrr.md` | — |
| 25 | `13-algorithms/regex-engine.md` | `dfa.md` |
| 26 | `13-algorithms/aho-corasick.md` | `regex-engine.md` |
| 27 | `14-memory/01-allocator.md` | — |
| 28 | `14-memory/02-arena.md` | — |
| 29 | `14-memory/03-object-pool.md` | — |
| 30 | `14-memory/04-buffer-pool.md` | `object-pool.md` |
| 31 | `14-memory/05-slab-allocator.md` | `13-algorithms/slab.md`, `allocator.md` |
| 32 | `14-memory/06-fragmentation.md` | `allocator.md`, `arena.md`, `object-pool.md` |

`15-parser/`, `16-kernel/`, `17-performance/`, and `18-distributed/` each
state their own internal order and reading trigger in their own
`00-README.md` — read those four `00-README.md`s for the same kind of ordering
once you get there; it isn't repeated here to avoid the two copies
drifting apart.

## What to learn

Each phase's *specific* subtopics live in that phase's own directory
(`01-network/09-dns.md`, `01-network/10-http.md`, ... — see `CLAUDE.md` for the
full file list) rather than being duplicated here. This file is the
one-page map; the topic files are the actual curriculum.

## Practice

- Work through `labs/00-tcp-server` through `labs/17-ebpf` in order, then
  `proxy/` — each crate's README points back at the handbook topics it needs.
- Once `labs/05-reverse-proxy` or `proxy/` is running, do the exercises in
  `12-testing/` against it.
