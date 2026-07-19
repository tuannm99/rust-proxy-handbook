# PROXY Protocol

How an L7 proxy learns the real client address when it's not the first hop.

## What to learn

### The problem it solves
When your proxy sits behind another load balancer or proxy (a cloud LB, a
CDN edge, another reverse proxy), the TCP connection your proxy sees comes
*from* that intermediary, not the original client — `peer_addr()` on the
accepted socket gives you the LB's IP, not the client's. The PROXY
protocol solves this at the TCP layer, before any HTTP parsing happens, by
having the upstream hop prepend a small header carrying the original
client's address.

### PROXY protocol v1 (text)
A human-readable line sent as the very first bytes of the connection:

```
PROXY TCP4 192.0.2.1 198.51.100.1 56324 443\r\n
```

`PROXY <family> <src-ip> <dst-ip> <src-port> <dst-port>\r\n`. Simple to
parse and debug, but verbose and limited to TCP4/TCP6/UNKNOWN.

### PROXY protocol v2 (binary)
A fixed 12-byte signature followed by a binary header (version/command,
address family/protocol, length, then the address block, optionally
followed by TLVs for extra metadata like the original TLS SNI/ALPN). Used
in production because it's cheaper to parse and unambiguous — no
delimiter-scanning, just fixed offsets and a length-prefixed body.

### Detecting and parsing it before the HTTP layer
The proxy must peek the first bytes of a newly accepted connection *before*
handing it to the HTTP parser: check for the v2 binary signature first
(unambiguous fixed bytes), fall back to checking for the literal `PROXY `
prefix for v1, and otherwise assume no PROXY protocol header is present.
Getting this wrong — e.g. treating the header as part of the HTTP request
body — corrupts every request behind it.

```rust
const V2_SIG: [u8; 12] = [
    0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A,
];

async fn peek_is_proxy_v2(stream: &tokio::net::TcpStream) -> std::io::Result<bool> {
    let mut buf = [0u8; 12];
    let n = stream.peek(&mut buf).await?;
    Ok(n == 12 && buf == V2_SIG)
}
```

### Trust boundary
Only accept a PROXY protocol header from connections you actually trust
(i.e. your known upstream LB's IP range) — anyone who can reach your
listener directly can otherwise *forge* the client address the same way an
unvalidated `X-Forwarded-For` can be forged at the HTTP layer (see
`07-security/ip-filtering.md`). Decide per-listener whether PROXY protocol
is expected at all; don't accept it unconditionally on a public listener.

## Practice

1. Send a raw PROXY v1 line by hand with `nc` in front of a test server and
   confirm the server can parse the original client address out of it.
2. Implement v1 detection/parsing in `proxy`'s
   connection-accept path, exposing the real client IP to the rest of the
   request pipeline (logging, rate limiting, WAF).
3. Add v2 (binary) support and test both formats against the same listener.
4. Add a trusted-source check: only honor a PROXY header if the connecting
   peer's IP is in an allowed list (tie to `07-security/ip-filtering.md`).
5. Explain why a client should never be able to send a PROXY protocol
   header directly and have it trusted.
