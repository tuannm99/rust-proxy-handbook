# Testing

Phase 10 trong thực tế, dù được đánh số 12. Mọi thứ ở đây được chạy *nhắm
vào* [`proxy/`](../../proxy) khi nó đã tồn tại — đây là các bài tập cho bạn biết chín
phase trước đó có thực sự hoạt động hay không.

## Files

- [`01-load-testing.md`](01-load-testing.md) — tạo tải thực tế, cái gì cần đo, mô hình open vs closed
- [`02-fuzzing.md`](02-fuzzing.md) — fuzz parser và bất kỳ input nào lộ ra phía kẻ tấn công
- [`03-chaos.md`](03-chaos.md) — tiêm lỗi upstream, latency, và xuống cấp một phần
- [`04-ci-tooling.md`](04-ci-tooling.md) — clippy, miri, sanitizer, và cái gì nên chặn merge
- [`05-debugging.md`](05-debugging.md) — `tracing`, `curl -v`/`nc`/`ss`/`tcpdump`, `strace`, `tokio-console`, debugger, flamegraph, heap profiler — bộ công cụ thu thập bằng chứng. Khác với các file còn lại, hữu ích ngay từ [`labs/00-tcp-server`](../../labs/00-tcp-server), không chỉ khi [`proxy/`](../../proxy) đã tồn tại
- [`06-lab-environment.md`](06-lab-environment.md) — cài đặt và sử dụng các công cụ bên ngoài mà phần kiểm tra `Done when` của các lab dùng: `oha`/`wrk`/`vegeta`/`h2load`, tải lệch kiểu Zipf, setup `criterion`, đo RSS và thread, Prometheus + `promtool`

## Đi tới đâu

Gần như mọi mục `## Practice` trong handbook này kết thúc bằng một bước
cần một trong số này: một load test để chứng minh một thay đổi có ích
([`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)), một fuzz target cho một input parser
([`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md), [`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md)), một lần
tiêm chaos để kích hoạt một alert ([`08-observability/06-alerting.md`](../08-observability/06-alerting.md)).

Thứ tự quan trọng cần nhớ: load-test *trước khi* tối ưu ([`17-performance/`](../17-performance)
nói rõ profiling chỉ đến sau khi một phép đo chỉ ra một hướng cụ thể), và
fuzz bất cứ thứ gì parse byte do kẻ tấn công kiểm soát trước khi ship nó.
