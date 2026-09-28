# Lộ trình học

Các phase dưới đây ánh xạ 1:1 với các thư mục đánh số. Mỗi phase liệt kê
những gì bạn nên *làm được*, không chỉ đọc thuộc, trước khi sang phase tiếp
theo. Cách làm từng phase — và thực tế mất bao lâu — nằm trong
[`03-study-loop.md`](03-study-loop.md).

0. **Điều kiện tiên quyết** ([`02-prerequisites.md`](02-prerequisites.md)) — pass phần tự kiểm tra
   của nó: viết và giải thích được Rust cơ bản (struct, enum, `Result`,
   trait, closure, lỗi borrow), và mô tả được chuyện gì xảy ra giữa `curl`
   và một response ở tầng DNS/TCP/HTTP. Chỉ bỏ qua phase này nếu phần tự
   kiểm tra đã toàn "có".

1. **Networking** ([`01-network/`](../01-network)) — đọc được một bản capture TCP hoặc TLS
   trong Wireshark và giải thích chuyện gì đang xảy ra; giải thích được vì
   sao HTTP/2 chỉ cần một kết nối TCP trong khi HTTP/1.1 cần tới sáu. Nếu
   các thuật ngữ như "port," "packet," "handshake," hay "certificate" chưa
   có nghĩa chính xác với bạn, bắt đầu từ [`01-network/01-fundamentals.md`](../01-network/01-fundamentals.md)
   — đây là nhóm file duy nhất trong handbook được viết như một primer từ
   con số 0, thay vì giả định một baseline có sẵn.
2. **Linux** ([`02-linux/`](../02-linux)) — giải thích được sự khác nhau giữa
   level-triggered và edge-triggered epoll từ việc chính bạn đã tự dính bug
   EAGAIN ở chế độ edge-triggered trong bài tập raw-epoll ở
   [`02-linux/07-epoll.md`](../02-linux/07-epoll.md). Cùng lưu ý như trên: đọc
   [`02-linux/01-fundamentals.md`](../02-linux/01-fundamentals.md) trước nếu "syscall," "file descriptor,"
   "kernel space," hay "container" chưa chính xác với bạn.
3. **Rust** ([`03-rust/`](../03-rust)) — giải thích được vì sao `Pin` tồn tại mà không
   cần đọc thuộc docs; biết khi nào nên dùng `Arc<Mutex<T>>` so với
   `Arc<RwLock<T>>` so với một atomic.
4. **Async runtime** ([`04-runtime/`](../04-runtime)) — giải thích được `.await` desugar
   thành gì và chuyện gì xảy ra trên executor thread khi một task trả về
   `Poll::Pending`.
5. **HTTP** ([`05-http-stack/`](../05-http-stack)) — tự tay parse một HTTP/1.1 request trong
   [`labs/01-http-parser`](../../labs/01-http-parser) và làm đúng framing Content-Length/chunked.
6. **Reverse Proxy** ([`06-proxy/`](../06-proxy)) — forward request tới một trong N
   upstream, sống sót khi một upstream sập mà không làm rớt request của
   client.
7. **Security** ([`07-security/`](../07-security)) — giải thích được một cuộc tấn công
   request-smuggling đủ rõ để phòng thủ được, chứ không chỉ gọi tên nó.
8. **Observability** ([`08-observability/`](../08-observability)) — trả lời được "p99 latency
   hiện tại là bao nhiêu và nó đến từ upstream nào" bằng chính
   logs/metrics/traces của proxy bạn viết.
9. **Architecture** ([`09-architecture/`](../09-architecture)) — reload config và drain kết nối
   khi shutdown mà không làm rớt request đang xử lý dở.
10. **Production** ([`proxy/README.md`](../../proxy/README.md)) — chạy bài load test và chaos
    trong [`12-testing/`](../12-testing) nhắm vào chính proxy của bạn và sống sót qua nó.

## Bản đồ đọc + code

Mỗi dòng là một crate trong [`labs/`](../../labs), theo thứ tự build: đọc phần tham
chiếu handbook trước, rồi mới implement. Bảng này phản ánh lại README của
từng crate — đây là bản one-page để thấy toàn bộ đường đi [`labs/`](../../labs) →
[`proxy/`](../../proxy) mà không cần mở 18 file.

| Lab | Đọc trước | Rồi code |
| --- | --- | --- |
| [`labs/00-tcp-server`](../../labs/00-tcp-server) | [`01-network/01-fundamentals.md`](../01-network/01-fundamentals.md), [`02-linux/01-fundamentals.md`](../02-linux/01-fundamentals.md) (nếu cần), [`01-network/07-socket.md`](../01-network/07-socket.md), [`01-network/08-tcp.md`](../01-network/08-tcp.md), [`03-rust/01-ownership.md`](../03-rust/01-ownership.md), [`03-rust/05-async.md`](../03-rust/05-async.md), [`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md), [`02-linux/07-epoll.md`](../02-linux/07-epoll.md) | TCP echo server |
| [`labs/01-http-parser`](../../labs/01-http-parser) | [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md), [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md) | tự viết HTTP/1.1 parser, không dùng hyper |
| [`labs/02-http-server`](../../labs/02-http-server) | [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md), [`05-http-stack/04-keepalive.md`](../05-http-stack/04-keepalive.md), [`05-http-stack/02-hop-by-hop-headers.md`](../05-http-stack/02-hop-by-hop-headers.md), [`01-network/10-http.md`](../01-network/10-http.md), [`01-network/11-http2.md`](../01-network/11-http2.md) | HTTP server thuần bằng hyper/hyper-util |
| [`labs/03-router`](../../labs/03-router) | [`05-http-stack/03-router.md`](../05-http-stack/03-router.md) | routing theo method+path |
| [`labs/04-static-server`](../../labs/04-static-server) | [`05-http-stack/05-static.md`](../05-http-stack/05-static.md), [`02-linux/11-zerocopy.md`](../02-linux/11-zerocopy.md) | streaming static file |
| [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) | [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md), [`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md), [`06-proxy/04-outlier-detection.md`](../06-proxy/04-outlier-detection.md), [`06-proxy/05-retry.md`](../06-proxy/05-retry.md), [`06-proxy/06-circuit-breaker.md`](../06-proxy/06-circuit-breaker.md), [`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md), [`01-network/10-http.md`](../01-network/10-http.md) | hyper client forward tới một upstream pool |
| [`labs/06-load-balancer`](../../labs/06-load-balancer) | [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md), [`13-algorithms/smooth-wrr.md`](../13-algorithms/smooth-wrr.md), [`13-algorithms/rendezvous-hash.md`](../13-algorithms/rendezvous-hash.md), [`13-algorithms/maglev.md`](../13-algorithms/maglev.md) | RR/least-conn/consistent-hash/smooth-WRR/Maglev |
| [`labs/07-tls`](../../labs/07-tls) | [`01-network/13-tls.md`](../01-network/13-tls.md) | tokio-rustls termination, ALPN |
| [`labs/08-http2`](../../labs/08-http2) | [`01-network/11-http2.md`](../01-network/11-http2.md) | chi tiết multiplexing/flow-control |
| [`labs/09-http3`](../../labs/09-http3) | [`01-network/12-http3.md`](../01-network/12-http3.md), [`19-reading-source/quinn/`](../19-reading-source/quinn) | QUIC qua `quinn` |
| [`labs/10-cache`](../../labs/10-cache) | [`05-http-stack/07-cache.md`](../05-http-stack/07-cache.md), [`05-http-stack/08-cache-stampede.md`](../05-http-stack/08-cache-stampede.md), [`13-algorithms/lru.md`](../13-algorithms/lru.md), [`13-algorithms/lfu.md`](../13-algorithms/lfu.md), [`13-algorithms/arc.md`](../13-algorithms/arc.md), [`13-algorithms/tinylfu.md`](../13-algorithms/tinylfu.md) | cache HTTP response + chính sách eviction |
| [`labs/11-rate-limit`](../../labs/11-rate-limit) | [`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md), [`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md), [`13-algorithms/token-bucket.md`](../13-algorithms/token-bucket.md), [`13-algorithms/sliding-window.md`](../13-algorithms/sliding-window.md), [`13-algorithms/leaky-bucket.md`](../13-algorithms/leaky-bucket.md) | token bucket / sliding window / leaky bucket |
| [`labs/12-waf`](../../labs/12-waf) | [`07-security/06-waf.md`](../07-security/06-waf.md), [`07-security/04-normalization.md`](../07-security/04-normalization.md), [`13-algorithms/aho-corasick.md`](../13-algorithms/aho-corasick.md) | filtering theo rule |
| [`labs/13-hot-reload`](../../labs/13-hot-reload) | [`09-architecture/03-config.md`](../09-architecture/03-config.md) | reload config mà không rớt kết nối |
| [`labs/14-plugin`](../../labs/14-plugin) | [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md) | middleware cho request/response |
| [`labs/15-prometheus`](../../labs/15-prometheus) | [`08-observability/02-metrics.md`](../08-observability/02-metrics.md) | export metrics |
| [`labs/16-opentelemetry`](../../labs/16-opentelemetry) | [`08-observability/03-tracing.md`](../08-observability/03-tracing.md) | export distributed tracing |
| [`labs/17-ebpf`](../../labs/17-ebpf) | [`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md), [`16-kernel/10-xdp.md`](../16-kernel/10-xdp.md), [`07-security/09-ddos.md`](../07-security/09-ddos.md), [`07-security/10-slowloris.md`](../07-security/10-slowloris.md) | packet filtering bằng XDP/eBPF |
| [`proxy/`](../../proxy) | [`01-network/13-tls.md`](../01-network/13-tls.md), [`01-network/14-proxy-protocol.md`](../01-network/14-proxy-protocol.md), `07-security/*.md`, `08-observability/*.md`, `09-architecture/*.md` (xem [`proxy/README.md`](../../proxy/README.md)) | proxy L7 cuối cùng, gộp mọi lab ở trên |

Các đường dẫn ở trên là tương đối so với `instruction/` (hay
`instruction-vi/` trong bản này) trừ khi có tiền tố [`labs/`](../../labs) hoặc [`proxy/`](../../proxy).
Nếu README của một crate lệch khỏi bảng này, README của crate đó là đúng —
cập nhật bảng này theo nó, không phải ngược lại.

Mỗi thư mục đánh số cũng có một [`00-README.md`](00-README.md) liệt kê các file kèm mô tả
một dòng và thứ tự đọc gợi ý — bắt đầu từ đó khi bước vào một phase, thay
vì đoán mò từ tên file.

## Lớp deep-dive (13-20)

[`13-algorithms/`](../13-algorithms) đến [`20-reference/`](../20-reference) không phải một "phase 11+" để đi qua
theo thứ tự — chúng là nền tảng được kéo vào *từ* các phase ở trên khi cần
(ví dụ implement Maglev ở phase 6 sẽ dẫn bạn tới [`13-algorithms/maglev.md`](../13-algorithms/maglev.md)).
Ngoại lệ duy nhất là [`19-reading-source/`](../19-reading-source), đáng để quay lại sau phase 10:
đọc source của `pingora` sau khi đã tự xây proxy của mình sẽ có ý nghĩa hơn
nhiều so với đọc nó khi chưa biết gì.

### Nếu bạn vẫn muốn đọc lớp deep-dive theo thứ tự thẳng

Thứ tự "kéo vào khi cần" ở trên là mặc định, nhưng nếu bạn muốn đọc hết
[`13-algorithms/`](../13-algorithms) và [`14-memory/`](../14-memory) một lượt (phần lớn khá ngắn, và đã xem qua
một lần sẽ giúp các phần "khi cần" sau này dễ tiếp thu hơn), đây là một thứ
tự nội bộ hợp lý — mỗi dòng chỉ phụ thuộc vào các dòng phía trên nó:

| # | File | Phụ thuộc vào |
| --- | --- | --- |
| 1 | [`13-algorithms/hashmap.md`](../13-algorithms/hashmap.md) | — |
| 2 | [`13-algorithms/dfa.md`](../13-algorithms/dfa.md) | — |
| 3 | [`13-algorithms/fsm.md`](../13-algorithms/fsm.md) | `dfa.md` |
| 4 | [`13-algorithms/trie.md`](../13-algorithms/trie.md) | — |
| 5 | [`13-algorithms/radix-tree.md`](../13-algorithms/radix-tree.md) | `trie.md` |
| 6 | [`13-algorithms/ring-buffer.md`](../13-algorithms/ring-buffer.md) | — |
| 7 | [`13-algorithms/heap.md`](../13-algorithms/heap.md) | — |
| 8 | [`13-algorithms/priority-queue.md`](../13-algorithms/priority-queue.md) | `ring-buffer.md`, `heap.md` |
| 9 | [`13-algorithms/skiplist.md`](../13-algorithms/skiplist.md) | — |
| 10 | [`13-algorithms/slab.md`](../13-algorithms/slab.md) | — |
| 11 | [`13-algorithms/lru.md`](../13-algorithms/lru.md) | `slab.md` |
| 12 | [`13-algorithms/lfu.md`](../13-algorithms/lfu.md) | — |
| 13 | [`13-algorithms/arc.md`](../13-algorithms/arc.md) | `lru.md`, `lfu.md` |
| 14 | [`13-algorithms/count-min-sketch.md`](../13-algorithms/count-min-sketch.md) | — |
| 15 | [`13-algorithms/bloom-filter.md`](../13-algorithms/bloom-filter.md) | — |
| 16 | [`13-algorithms/hyperloglog.md`](../13-algorithms/hyperloglog.md) | — |
| 17 | [`13-algorithms/tinylfu.md`](../13-algorithms/tinylfu.md) | `count-min-sketch.md`, `bloom-filter.md`, `lru.md` |
| 18 | [`13-algorithms/token-bucket.md`](../13-algorithms/token-bucket.md) | — |
| 19 | [`13-algorithms/sliding-window.md`](../13-algorithms/sliding-window.md) | — |
| 20 | [`13-algorithms/leaky-bucket.md`](../13-algorithms/leaky-bucket.md) | `token-bucket.md` |
| 21 | [`13-algorithms/consistent-hash.md`](../13-algorithms/consistent-hash.md) | — |
| 22 | [`13-algorithms/rendezvous-hash.md`](../13-algorithms/rendezvous-hash.md) | `consistent-hash.md` |
| 23 | [`13-algorithms/maglev.md`](../13-algorithms/maglev.md) | `consistent-hash.md` |
| 24 | [`13-algorithms/smooth-wrr.md`](../13-algorithms/smooth-wrr.md) | — |
| 25 | [`13-algorithms/regex-engine.md`](../13-algorithms/regex-engine.md) | `dfa.md` |
| 26 | [`13-algorithms/aho-corasick.md`](../13-algorithms/aho-corasick.md) | `regex-engine.md` |
| 27 | [`14-memory/01-allocator.md`](../14-memory/01-allocator.md) | — |
| 28 | [`14-memory/02-arena.md`](../14-memory/02-arena.md) | — |
| 29 | [`14-memory/03-object-pool.md`](../14-memory/03-object-pool.md) | — |
| 30 | [`14-memory/04-buffer-pool.md`](../14-memory/04-buffer-pool.md) | `object-pool.md` |
| 31 | [`14-memory/05-slab-allocator.md`](../14-memory/05-slab-allocator.md) | [`13-algorithms/slab.md`](../13-algorithms/slab.md), `allocator.md` |
| 32 | [`14-memory/06-fragmentation.md`](../14-memory/06-fragmentation.md) | `allocator.md`, `arena.md`, `object-pool.md` |

[`15-parser/`](../15-parser), [`16-kernel/`](../16-kernel), [`17-performance/`](../17-performance), và [`18-distributed/`](../18-distributed) mỗi
thư mục đều tự nêu thứ tự nội bộ và thời điểm nên đọc trong [`00-README.md`](00-README.md)
của chính nó — đọc bốn file [`00-README.md`](00-README.md) đó để có cùng kiểu sắp xếp khi
bạn tới đó; không lặp lại ở đây để tránh hai bản lệch nhau.

## What to learn

Các subtopic *cụ thể* của từng phase nằm trong chính thư mục của phase đó
([`01-network/09-dns.md`](../01-network/09-dns.md), [`01-network/10-http.md`](../01-network/10-http.md), ... — xem [`CLAUDE.md`](../../CLAUDE.md) để
có danh sách file đầy đủ) thay vì lặp lại ở đây. File này là bản đồ
one-page; các file topic mới là curriculum thật sự.

## Practice

- Đi qua [`labs/00-tcp-server`](../../labs/00-tcp-server) đến [`labs/17-ebpf`](../../labs/17-ebpf) theo thứ tự, rồi tới
  [`proxy/`](../../proxy) — README của mỗi crate trỏ ngược lại các topic handbook mà nó
  cần.
- Khi [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) hoặc [`proxy/`](../../proxy) đã chạy được, làm các bài tập
  trong [`12-testing/`](../12-testing) nhắm vào nó.
