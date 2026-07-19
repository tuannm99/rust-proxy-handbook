# RFCs
- RFC 9110 — HTTP Semantics (methods, status codes, headers). Read alongside `01-network/http.md`.
- RFC 9112 — HTTP/1.1 message syntax and routing (the actual wire format). Read alongside `05-http-stack/parser.md` and `05-http-stack/keepalive.md`.
- RFC 7540 / RFC 9113 — HTTP/2 (frames, streams, HPACK; 9113 obsoletes 7540). Read alongside `01-network/http2.md`.
- RFC 9000 — QUIC transport, the substrate under HTTP/3. Read alongside `01-network/http3.md`.
- RFC 9114 — HTTP/3 semantics over QUIC. Read alongside `01-network/http3.md`.
- RFC 7541 — HPACK header compression, referenced by RFC 7540/9113.
- RFC 8446 — TLS 1.3 (handshake, 0-RTT, session resumption). Read alongside `01-network/tls.md`.
- RFC 6455 — The WebSocket Protocol. Read alongside `05-http-stack/websocket.md`.
- RFC 9111 — HTTP Caching (obsoletes the caching parts of 7234). Read alongside `05-http-stack/cache.md`.
- RFC 1035 — DNS message format, still the base for `01-network/dns.md`.
- RFC 6520 / RFC 5077 — TLS session resumption mechanisms referenced from `01-network/tls.md`.

Note: the PROXY protocol used in `01-network/proxy-protocol.md` is **not** an RFC — it's a de facto spec published and maintained by HAProxy (see their `proxy-protocol.txt`).
