# Learning Roadmap

The phases below map 1:1 to the numbered directories. Each phase lists what
you should be able to *do*, not just recite, before moving to the next one.

1. **Networking** (`01-network/`) — read a TCP or TLS packet capture in
   Wireshark and explain what's happening; explain why HTTP/2 needs one TCP
   connection where HTTP/1.1 needed six.
2. **Linux** (`02-linux/`) — explain the difference between level-triggered
   and edge-triggered epoll from having hit the edge-triggered EAGAIN bug
   yourself in the raw-epoll exercise in `02-linux/epoll.md`.
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
10. **Production** (`proxy/README.md`) — run `12-testing/`'s load
    test and chaos exercises against your own proxy and survive them.

## Read + code map

One row per `labs/` crate, in build order: read its handbook references
first, then implement it. This table mirrors each crate's own README —
it's the one-page version so the whole `labs/` → `proxy/` path is visible
without opening 18 files.

| Lab | Read first | Then code |
| --- | --- | --- |
| `labs/00-tcp-server` | `01-network/socket.md`, `01-network/tcp.md`, `03-rust/ownership.md`, `03-rust/async.md`, `04-runtime/tokio.md`, `02-linux/epoll.md` | TCP echo server |
| `labs/01-http-parser` | `05-http-stack/parser.md`, `07-security/request-smuggling.md` | hand-written HTTP/1.1 parser, no hyper |
| `labs/02-http-server` | `05-http-stack/parser.md`, `05-http-stack/keepalive.md`, `01-network/http.md`, `01-network/http2.md` | hyper/hyper-util plain HTTP server |
| `labs/03-router` | `05-http-stack/router.md` | method+path routing |
| `labs/04-static-server` | `05-http-stack/static.md`, `02-linux/zerocopy.md` | streaming static files |
| `labs/05-reverse-proxy` | `06-proxy/upstream.md`, `06-proxy/healthcheck.md`, `06-proxy/retry.md`, `06-proxy/service-discovery.md`, `01-network/http.md` | hyper client forwarding to an upstream pool |
| `labs/06-load-balancer` | `06-proxy/load-balancer.md`, `13-algorithms/smooth-wrr.md`, `13-algorithms/rendezvous-hash.md`, `13-algorithms/maglev.md` | RR/least-conn/consistent-hash/smooth-WRR/Maglev |
| `labs/07-tls` | `01-network/tls.md` | tokio-rustls termination, ALPN |
| `labs/08-http2` | `01-network/http2.md` | multiplexing/flow-control specifics |
| `labs/09-http3` | `01-network/http3.md`, `19-reading-source/quinn/` | QUIC via `quinn` |
| `labs/10-cache` | `05-http-stack/cache.md`, `13-algorithms/lru.md`, `13-algorithms/lfu.md`, `13-algorithms/arc.md`, `13-algorithms/tinylfu.md` | HTTP response caching + eviction policy |
| `labs/11-rate-limit` | `07-security/ratelimit.md`, `13-algorithms/token-bucket.md`, `13-algorithms/sliding-window.md`, `13-algorithms/leaky-bucket.md` | token bucket / sliding window / leaky bucket |
| `labs/12-waf` | `07-security/waf.md`, `13-algorithms/aho-corasick.md` | rule-based filtering |
| `labs/13-hot-reload` | `09-architecture/config.md` | config reload without dropping connections |
| `labs/14-plugin` | `09-architecture/plugin.md` | request/response middleware |
| `labs/15-prometheus` | `08-observability/metrics.md` | metrics export |
| `labs/16-opentelemetry` | `08-observability/tracing.md` | distributed tracing export |
| `labs/17-ebpf` | `16-kernel/ebpf.md`, `16-kernel/xdp.md`, `07-security/ddos.md` | XDP/eBPF packet filtering |
| `proxy/` | `01-network/tls.md`, `01-network/proxy-protocol.md`, `07-security/*.md`, `08-observability/*.md`, `09-architecture/*.md` (see `proxy/README.md`) | the final L7 proxy, combining every lab above |

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

## What to learn

Each phase's *specific* subtopics live in that phase's own directory
(`01-network/dns.md`, `01-network/http.md`, ... — see `CLAUDE.md` for the
full file list) rather than being duplicated here. This file is the
one-page map; the topic files are the actual curriculum.

## Practice

- Work through `labs/00-tcp-server` through `labs/17-ebpf` in order, then
  `proxy/` — each crate's README points back at the handbook topics it needs.
- Once `labs/05-reverse-proxy` or `proxy/` is running, do the exercises in
  `12-testing/` against it.
