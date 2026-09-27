# Reverse Proxy

Phase 6 — phần lõi của việc `proxy/` thực sự là gì. Mọi thứ ở đây xoay
quanh phía *upstream*: request nào đi tới backend nào, chuyện gì xảy ra khi
nó fail, và tập hợp backend thay đổi ra sao ngay dưới chân bạn.

## Files

- `01-upstream.md` — model một upstream, connection reuse và sizing pool, ba loại timeout
- `02-load-balancer.md` — round robin, least connections, P2C, peak EWMA, consistent hashing
- `03-healthcheck.md` — active probing, mỗi probe depth chứng minh điều gì, deep-check gây correlated failure
- `04-outlier-detection.md` — passive detection từ traffic thật, flap damping, slow start
- `05-retry.md` — idempotency, retry budget, backoff và jitter, hedged request
- `06-circuit-breaker.md` — trip theo failure rate, half-open gating, scope theo từng upstream
- `07-service-discovery.md` — membership dựa trên DNS và watch, không bao giờ chấp nhận kết quả rỗng, draining

## Thứ tự đọc

Đọc `01-upstream.md` trước — nó định nghĩa type mà mọi thứ khác thao tác
trên đó. Rồi tới `02-load-balancer.md` (chọn một upstream), sau đó là cặp
xử lý failure theo thứ tự nào cũng được: `03-healthcheck.md` +
`04-outlier-detection.md` (quyết định một host bị hỏng) và `05-retry.md` +
`06-circuit-breaker.md` (phản ứng với một request thất bại).
`07-service-discovery.md` đọc cuối, vì nó thay đổi cái pool mà mọi phần
trước đó giả định là cố định.

Cùng nhau, các file này hỗ trợ `labs/05-reverse-proxy` và
`labs/06-load-balancer`. Các thuật toán đứng sau balancer — smooth WRR,
Maglev, rendezvous hashing — có bài deep dive riêng trong `13-algorithms/`.
