# Rust Nginx Handbook

Một handbook có cấu trúc để tự xây dựng một reverse proxy L7 (HTTP)
production-grade bằng Rust, lấy cảm hứng về mặt khái niệm từ nginx, Envoy,
và HAProxy. Handbook được tổ chức như một chuỗi phụ thuộc: mỗi thư mục đánh
số giả định bạn đã nắm các thư mục trước đó.

```
00 introduction  -> vì sao handbook này tồn tại, cách dùng nó
01 network       -> các giao thức trên dây mà proxy phải nói (DNS, HTTP/1/2/3, TCP, TLS)
02 linux         -> các primitive của kernel mà proxy được xây trên đó (epoll, io_uring, zero-copy)
03 rust          -> cơ chế ngôn ngữ mà async Rust cần (ownership, Pin, unsafe, sync)
04 runtime       -> cách tokio biến 02+03 thành một async runtime
05 http-stack    -> parsing/routing/caching trên nền runtime
06 proxy         -> forward request tới upstream: load balancing, health check, retry
07 security      -> auth, rate limiting, WAF, request smuggling, IP filtering
08 observability -> logging, metrics, tracing, profiling
09 architecture  -> nối tất cả lại thành một process: components, config, plugin, shutdown
12 testing       -> load testing, fuzzing, chaos testing cho những gì bạn đã xây
```

([`13-algorithms/`](../13-algorithms) đến [`21-reading-list/`](../21-reading-list) là một lớp deep-dive/phụ lục nằm
ngoài chuỗi phụ thuộc này — xem [`CLAUDE.md`](../../CLAUDE.md).)

Một Cargo workspace đi kèm nằm ở gốc repo ([`labs/`](../../labs), [`proxy/`](../../proxy)) — xem
[`README.md`](../../README.md) ở gốc. [`proxy/`](../../proxy) mới là deliverable thật sự; [`labs/`](../../labs) là 18
bài tập đánh số, độ khó tăng dần, xây dựng dần các kỹ năng mà [`proxy/`](../../proxy) cần.
Mỗi topic trong handbook có mục `## Practice` trỏ tới một crate cụ thể
trong [`labs/`](../../labs) hoặc [`proxy/`](../../proxy).

## Cách dùng handbook này

1. Đi qua các thư mục gần đúng theo thứ tự số — [`06-proxy`](../06-proxy) giả định bạn đã
   qua [`04-runtime`](../04-runtime) và [`05-http-stack`](../05-http-stack), chứ không chỉ "có chút kinh nghiệm
   Rust".
2. Với mỗi file topic, đọc `## What to learn`, rồi làm bài tập liên kết
   trong `## Practice` trước khi đi tiếp. Đừng đọc lướt hết cả 13 thư mục
   rồi mới bắt đầu code — các crate trong [`labs/`](../../labs) và [`proxy/`](../../proxy) mới là nơi
   khái niệm thực sự đọng lại.
3. Dùng [`21-reading-list/`](../21-reading-list) như tài liệu đọc thêm song song, không phải điều
   kiện tiên quyết — không có gì trong `01`-`09` yêu cầu bạn phải đọc sách
   trước.
4. Coi [`12-testing/`](../12-testing) là bài tập chạy *sau khi* dự án đã chạy được, không
   phải trước — bạn cần một proxy đang chạy để load-test hoặc fuzz nó.

## Phạm vi

Trong phạm vi: mọi thứ cần để tự xây một L7 HTTP proxy từ đầu — TCP/TLS
termination, HTTP parsing, routing, load balancing, và các lớp
security/observability mà một deployment thật cần.

Ngoài phạm vi: load balancer chỉ ở L3/L4 (kiểu IPVS), các giao thức không
phải HTTP (gRPC được nhắc qua ở chỗ nó ảnh hưởng đến xử lý HTTP/2 nhưng
không phải một topic riêng), và các vấn đề cloud/infra (Kubernetes Ingress
controller, service mesh) ngoài các pointer kiểu "chỗ này sẽ cắm vào đâu"
trong [`09-architecture/`](../09-architecture).
