# Network

Phase 1 của lộ trình học. Các giao thức mà một proxy phải nói, từ socket
cho tới HTTP/3 — đọc trước `02-linux/` và `05-http-stack/`, hai phần giả
định bạn đã biết một connection và một request thực sự là gì.

## Files

**Fundamentals** (bắt đầu từ đây nếu "port," "packet," "handshake," "NAT,"
hay "certificate" chưa có nghĩa chính xác với bạn — nhóm file duy nhất
trong thư mục này được viết như một primer từ con số 0 thay vì giả định
sẵn một baseline; mọi file bên dưới đều giả định bạn đã nắm phần này):
- `01-fundamentals.md` — mô hình client-server, và một index tới năm file bên dưới
- `02-addressing.md` — IP address, port, CIDR notation, NAT (SNAT/DNAT/CGNAT), kiến thức routing cơ bản
- `03-byte-streams.md` — ảo giác byte-stream, packet/segment/datagram, TCP vs UDP, handshake như một pattern
- `04-latency-throughput.md` — latency, bandwidth, throughput, RTT, bandwidth-delay product
- `05-proxy-taxonomy.md` — forward vs reverse proxy, L4 vs L7, NAT gateway, API gateway, CDN, sidecar — `proxy/` nằm ở đâu trong bức tranh này
- `06-crypto-basics.md` — mã hóa symmetric/asymmetric, hashing, HMAC, digital signature, certificate/PKI — điều kiện tiên quyết mà `13-tls.md` và các file identity trong `07-security/` giả định bạn đã biết

**Protocols:**
- `07-socket.md` — bind/listen/accept, socket option, `SO_REUSEADDR`
- `08-tcp.md` — handshake, byte-stream framing, short read và short write
- `09-dns.md` — resolution, TTL, resolver caching, và vì sao một process sống lâu phải re-resolve
- `10-http.md` — ngữ nghĩa HTTP/1.1: method, status code, header nào proxy phải rewrite
- `11-http2.md` — vòng đời stream, trạng thái HPACK, flow control, Rapid Reset
- `12-http3.md` — QUIC, demux trên UDP, QPACK, chi phí congestion, Alt-Svc
- `13-tls.md` — handshake, SNI, ALPN, session resumption
- `14-proxy-protocol.md` — giữ lại IP client thật khi đứng sau một load balancer khác

## Bước tiếp theo

Nhóm fundamentals trước nếu bạn cần — mọi phần còn lại đều giả định bạn đã
qua nó. `07-socket.md` + `08-tcp.md` là nền cho `labs/00-tcp-server`;
`10-http.md` là nền cho `labs/02-http-server`; `13-tls.md` là nền cho
`labs/07-tls`; `11-http2.md` và `12-http3.md` là nền cho `labs/08-http2` và
`labs/09-http3`. Cơ chế phía kernel bên dưới nằm ở
`16-kernel/03-tcp-stack.md`.
