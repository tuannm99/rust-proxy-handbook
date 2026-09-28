# HTTP Stack

Phase 5. Mọi thứ proxy làm với một HTTP message trước khi quyết định gửi nó
đi đâu — parse nó, route nó, và các hành vi riêng theo từng protocol khiến
một proxy khác với một HTTP server thông thường.

## Files

- [`01-parser.md`](01-parser.md) — tự viết một HTTP/1.1 parser: incremental parsing, framing, các giới hạn
- [`02-hyper.md`](02-hyper.md) — bản đồ hyper 1.x: các crate, connection future, service và vì sao `Err` không phải là 500, streaming body và backpressure, timeout, client có pool
- [`03-hop-by-hop-headers.md`](03-hop-by-hop-headers.md) — những header nào không bao giờ được forward, và vì sao forward các header framing tạo ra bug smuggling
- [`04-router.md`](04-router.md) — match method và path, precedence, và path normalization như một security boundary
- [`05-keepalive.md`](05-keepalive.md) — persistent connection, vòng đời pooling, giới hạn tuổi thọ connection, cuộc đua close/request
- [`06-static.md`](06-static.md) — streaming file, range request, conditional request, path traversal
- [`07-compression.md`](07-compression.md) — negotiation gzip/brotli/zstd, streaming compression, BREACH
- [`08-cache.md`](08-cache.md) — semantics caching của proxy: directive, key, `Vary`, poisoning
- [`09-cache-stampede.md`](09-cache-stampede.md) — single-flight coalescing, stale-while-revalidate, TTL jitter
- [`10-websocket.md`](10-websocket.md) — upgrade handshake, kiểm tra Origin, framing, backpressure
- [`11-grpc.md`](11-grpc.md) — trailer, các dạng streaming, load balancing theo từng RPC, lỗi kiểu gRPC
- [`12-vhost-routing.md`](12-vhost-routing.md) — routing theo Host/SNI, chứng chỉ multi-tenant, cách ly tenant

## Thứ tự đọc

[`01-parser.md`](01-parser.md) trước tiên (nó là nền tảng cho [`labs/01-http-parser`](../../labs/01-http-parser), mà các
phần sau giả định bạn đã làm), rồi [`02-hyper.md`](02-hyper.md) trước [`labs/02-http-server`](../../labs/02-http-server) (mọi lab
sau đó đều xây trên hyper), rồi tới [`03-hop-by-hop-headers.md`](03-hop-by-hop-headers.md) và
[`04-router.md`](04-router.md) — hai file đó định nghĩa proxy được phép forward cái gì và
quyết định gửi đi đâu như thế nào. Các file còn lại độc lập với nhau và ánh
xạ tới lab riêng của chúng: [`06-static.md`](06-static.md) → [`labs/04-static-server`](../../labs/04-static-server),
[`08-cache.md`](08-cache.md) + [`09-cache-stampede.md`](09-cache-stampede.md) → [`labs/10-cache`](../../labs/10-cache), v.v.

Các thuật toán eviction nằm ở [`13-algorithms/`](../13-algorithms); kỷ luật normalization mà
[`04-router.md`](04-router.md) và [`07-security/06-waf.md`](../07-security/06-waf.md) cùng chia sẻ nằm ở
[`07-security/04-normalization.md`](../07-security/04-normalization.md).
