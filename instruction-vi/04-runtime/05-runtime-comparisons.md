# Runtime Landscape: Work-Stealing vs. Thread-per-Core

## What to learn

### Mô hình của tokio, nhìn lại như một lựa chọn thiết kế
Runtime multi-threaded của tokio là work-stealing: bất kỳ task nào cũng
có thể chạy trên bất kỳ worker thread nào, và worker rảnh ăn cắp từ worker
bận ([`04-runtime/01-tokio.md`](01-tokio.md)). Điều này tối ưu cho cân bằng tải giữa các
core với ít tinh chỉnh thủ công nhất — bạn không phải nghĩ về việc kết nối
nào rơi vào core nào. Cái giá: một task di chuyển giữa các core nghĩa là
dữ liệu của nó (một cache line của `Arc<Mutex<_>>`, một cấp phát cục bộ
NUMA) giờ có thể bị đụng từ một core khác với nơi nó được cấp phát —
overhead thật, dù thường nhỏ ([`17-performance/03-numa.md`](../17-performance/03-numa.md)).

### Giải pháp thay thế thread-per-core: `glommio`, `monoio`
Một runtime thread-per-core ("shard-per-core") pin một thread cho mỗi
core, cho nó event loop riêng và thường là một instance `io_uring` riêng
([`02-linux/08-io_uring.md`](../02-linux/08-io_uring.md)), và không bao giờ di chuyển một task khỏi
core nó bắt đầu. Cấu trúc dữ liệu có thể là `Rc<RefCell<_>>` thay vì
`Arc<Mutex<_>>` bên trong một shard, vì không có gì khác từng đụng vào
memory của shard đó — không atomic, không cache-line nảy qua lại giữa
các core. Cái giá chuyển sang chỗ khác: mất cân bằng tải giữa các shard
phải được giải quyết ở tầng kiến trúc (`SO_REUSEPORT` cộng với việc phân
phối kết nối của kernel, [`16-kernel/05-rss.md`](../16-kernel/05-rss.md)/[`16-kernel/06-rps.md`](../16-kernel/06-rps.md)) thay vì để
một scheduler tự động ăn cắp việc.

```rust
// Hình dạng khái niệm, không phải tokio: mỗi shard sở hữu độc quyền kết nối của nó.
// glommio::LocalExecutorBuilder::new(Placement::Fixed(core_id)).spawn(|| async move {
//     // event loop, instance io_uring, và trạng thái Rc<RefCell<_>> của shard này đều sống ở đây
// });
```

### Vì sao nginx và Envoy đã chọn cách này, và vì sao tokio không bắt buộc phải vậy
Mô hình worker-process-per-core của nginx và mô hình thread-per-core của
Envoy là phiên bản C++/systems-level của cùng ý tưởng shard-per-core, có
trước io_uring — chúng phân phối listening socket qua `SO_REUSEPORT` và
theo thiết kế không bao giờ chia sẻ trạng thái kết nối giữa các worker.
Mô hình work-stealing của tokio tồn tại một phần *vì* hệ thống ownership
và `Send`/`Sync` của Rust làm cho việc chia sẻ an toàn giữa các thread
(`Arc<Mutex<_>>`) rẻ để viết đúng, thứ C++ không cho không — lưới an toàn
mà thread-per-core né tránh trong C++ là thứ Rust đã có sẵn, điều này làm
suy yếu (mà không loại bỏ) lợi thế truyền thống của thread-per-core.

### Khi nào sự khác biệt thực sự lộ ra
Với hầu hết workload proxy HTTP/1.1 hay HTTP/2 bị nghẽn bởi TLS handshake,
header parsing, hay round-trip time tới upstream, mô hình work-stealing
của tokio không phải bottleneck — overhead di chuyển giữa các core nhỏ so
với các chi phí đó. Kiến trúc thread-per-core xứng đáng với độ phức tạp
của nó ở workload connection-churn rất cao, request rất ngắn (hàng triệu
op nhỏ mỗi giây, nghĩ tới một cache hay KV proxy) nơi chính overhead mỗi
request, không phải chờ I/O, chiếm ưu thế — chính xác nơi các lựa chọn
cấu trúc dữ liệu kiểu [`13-algorithms/`](../13-algorithms) và các mối quan tâm cache/NUMA của
[`17-performance/`](../17-performance) bắt đầu quan trọng hơn việc chọn mô hình async nào.

### [`proxy`](../../proxy) đứng ở đâu
[`proxy`](../../proxy) được xây trên mô hình work-stealing của tokio một cách có chủ đích
([`04-runtime/00-README.md`](00-README.md)) — không phải vì thread-per-core sai, mà vì
proxy HTTP L7 không rõ ràng cần nó, và hệ sinh thái của tokio (hyper,
rustls, h2, quinn) là thứ phần còn lại của handbook này giả định. Hiểu
giải pháp thay thế là thứ giúp bạn nhận ra sau này, liệu một hot path cụ
thể bên trong [`proxy`](../../proxy) (một lần tra cứu cache rất nóng,
[`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)) có được lợi từ việc tách ra thành component
shard-per-core riêng của nó thay vì luôn giả định mặc định của tokio là
công cụ đúng.

## Practice
1. Đọc README của `glommio` hay `monoio` và xác định cụ thể những API nào
   của tokio (đặc biệt các API dựa trên `Arc<Mutex<_>>`) không có tương
   đương trực tiếp trong mô hình lập trình của chúng, và ghi lại vì sao.
2. Benchmark [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) dưới connection churn cao (nhiều kết
   nối ngắn hạn) và dùng `perf top` hay `tokio-console` để tìm bottleneck
   thực sự nằm ở đâu — xác nhận đó là overhead scheduler hay thứ khác
   (chi phí TLS handshake, chi phí accept-loop).
3. Giải thích, bằng lời của bạn, vì sao `SO_REUSEPORT`
   ([`01-network/07-socket.md`](../01-network/07-socket.md)) là nền tảng cho một thiết kế thread-per-core
   theo cách nó không phải với mô hình work-stealing của tokio.
4. Đọc [`16-kernel/05-rss.md`](../16-kernel/05-rss.md) và [`16-kernel/06-rps.md`](../16-kernel/06-rps.md), và nối việc điều hướng packet
   ở mức NIC với vì sao một proxy thread-per-core quan tâm packet của một
   kết nối rơi vào core nào, trong khi một proxy work-stealing thì hầu
   như không.
5. Viết một đoạn văn bảo vệ lựa chọn tokio thay vì một runtime
   thread-per-core của [`proxy`](../../proxy), cụ thể theo những gì [`proxy`](../../proxy) thực sự làm
   (terminate và proxy HTTP L7) thay vì một lập luận chung chung "tokio
   phổ biến hơn."
