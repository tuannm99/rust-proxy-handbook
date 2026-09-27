# RFC
- RFC 9110 — HTTP Semantics (method, status code, header). Đọc song song
  với `01-network/10-http.md`.
- RFC 9112 — HTTP/1.1 message syntax and routing (định dạng thực tế trên
  dây). Đọc song song với `05-http-stack/01-parser.md` và
  `05-http-stack/04-keepalive.md`.
- RFC 7540 / RFC 9113 — HTTP/2 (frame, stream, HPACK; 9113 thay thế
  7540). Đọc song song với `01-network/11-http2.md`.
- RFC 9000 — QUIC transport, nền tảng bên dưới HTTP/3. Đọc song song với
  `01-network/12-http3.md`.
- RFC 9114 — HTTP/3 semantics trên QUIC. Đọc song song với
  `01-network/12-http3.md`.
- RFC 7541 — nén header HPACK, được tham chiếu bởi RFC 7540/9113.
- RFC 8446 — TLS 1.3 (handshake, 0-RTT, session resumption). Đọc song
  song với `01-network/13-tls.md`.
- RFC 6455 — Giao thức WebSocket. Đọc song song với
  `05-http-stack/09-websocket.md`.
- RFC 9111 — HTTP Caching (thay thế phần caching của 7234). Đọc song song
  với `05-http-stack/07-cache.md`.
- RFC 1035 — định dạng message DNS, vẫn là nền tảng cho
  `01-network/09-dns.md`.
- RFC 6520 / RFC 5077 — các cơ chế session resumption của TLS được tham
  chiếu từ `01-network/13-tls.md`.

Lưu ý: giao thức PROXY dùng trong `01-network/14-proxy-protocol.md`
**không phải** một RFC — đó là một spec de facto do HAProxy công bố và
duy trì (xem `proxy-protocol.txt` của họ).
