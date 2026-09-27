# Async Runtime

Phase 4. Cách tokio biến readiness notification của [`02-linux/07-epoll.md`](../02-linux/07-epoll.md)
và state machine của [`03-rust/05-async.md`](../03-rust/05-async.md) thành một scheduler thực sự
hoạt động — và điều đó có nghĩa gì cho code bạn viết trên nền đó.

## Files

- [`01-tokio.md`](01-tokio.md) — scheduler multi-threaded, work stealing, `spawn`, offload sang blocking-pool
- [`02-waker.md`](02-waker.md) — `Poll::Pending`, waker, `.await` thực sự đăng ký cái gì
- [`03-runtime-config.md`](03-runtime-config.md) — `multi_thread` vs `current_thread`, `LocalSet`/`spawn_local`, định cỡ blocking-pool, `tokio-console`
- [`04-structured-concurrency.md`](04-structured-concurrency.md) — `JoinHandle`/`JoinSet`, cancellation-safety với API thật, `task_local!`
- [`05-runtime-comparisons.md`](05-runtime-comparisons.md) — work-stealing vs thread-per-core (`glommio`/`monoio`), và vì sao [`proxy`](../../proxy) chọn tokio

## Đi tiếp theo đâu

Mọi thứ từ [`05-http-stack/`](../05-http-stack) trở đi chạy trên nền này. Hai failure mode
cần mang theo: block một worker thread làm nghẽn mọi kết nối multiplex
trên nó (xem [`08-observability/04-profiling.md`](../08-observability/04-profiling.md) và `tokio-console` để
tìm ra nó), và một future bị drop là một thao tác bị hủy, thứ mà
[`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) biến thành một bug cụ thể —
[`04-structured-concurrency.md`](04-structured-concurrency.md) là nơi điều đó trở thành một cách sửa dựa
trên `JoinSet` thay vì chỉ là một lời cảnh báo mang tính khái niệm.
[`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md) dựa trực tiếp vào cơ chế
cancellation của [`04-structured-concurrency.md`](04-structured-concurrency.md), và
[`05-runtime-comparisons.md`](05-runtime-comparisons.md) là điểm để quay lại nếu một hot path cụ thể
có vẻ vượt quá mô hình mặc định của tokio.
