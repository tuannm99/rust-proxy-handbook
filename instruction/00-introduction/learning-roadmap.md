# Learning Roadmap

The phases below map 1:1 to the numbered directories. Each phase lists what
you should be able to *do*, not just recite, before moving to the next one.

1. **Networking** (`01-network/`) — read a TCP or TLS packet capture in
   Wireshark and explain what's happening; explain why HTTP/2 needs one TCP
   connection where HTTP/1.1 needed six.
2. **Linux** (`02-linux/`) — explain the difference between level-triggered
   and edge-triggered epoll from having hit the edge-triggered EAGAIN bug
   yourself in `labs/epoll-echo`.
3. **Rust** (`03-rust/`) — explain why `Pin` exists without reciting the
   docs; know when to reach for `Arc<Mutex<T>>` vs `Arc<RwLock<T>>` vs an
   atomic.
4. **Async runtime** (`04-runtime/`) — explain what `.await` desugars to and
   what happens on the executor thread when a task returns `Poll::Pending`.
5. **HTTP** (`05-http-stack/`) — hand-parse an HTTP/1.1 request in
   `labs/http-parser-raw` and get the Content-Length/chunked framing right.
6. **Reverse Proxy** (`06-proxy/`) — forward a request to one of N upstreams,
   survive one upstream going down without dropping client requests.
7. **Security** (`07-security/`) — explain a request-smuggling attack well
   enough to defend against it, not just name it.
8. **Observability** (`08-observability/`) — answer "what's our p99 latency
   right now and which upstream is it coming from" using your own proxy's
   logs/metrics/traces.
9. **Architecture** (`09-architecture/`) — reload config and drain
   connections on shutdown without dropping in-flight requests.
10. **Production** (`10-projects/project-04.md`) — run `12-testing/`'s load
    test and chaos exercises against your own proxy and survive them.

## What to learn

Each phase's *specific* subtopics live in that phase's own directory
(`01-network/dns.md`, `01-network/http.md`, ... — see `CLAUDE.md` for the
full file list) rather than being duplicated here. This file is the
one-page map; the topic files are the actual curriculum.

## Practice

- Work through `10-projects/project-01.md` through `project-04.md` in
  order — each is gated on the phases listed in its own
  "Handbook prerequisites" section.
- After `project-03` or `project-04` is running, do the exercises in
  `12-testing/` against it.
