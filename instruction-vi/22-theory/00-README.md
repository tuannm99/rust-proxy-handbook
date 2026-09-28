# CS Theory

Một lớp tham khảo, giống [`13-algorithms/`](../13-algorithms): lý thuyết CS kinh điển đằng
sau những gì [`01-network/`](../01-network), [`02-linux/`](../02-linux), và [`03-rust/`](../03-rust) dạy ở mức thực
dụng, "đủ để xây một proxy". Không có thứ tự đọc — được kéo vào từ các
thư mục đó khi cần, không đọc từ đầu tới cuối. Không có gì ở đây là bắt
buộc để hoàn thành [`proxy/`](../../proxy); nó tồn tại cho người đọc muốn có nền tảng học
thuật đằng sau phần xử lý thực dụng — lý thuyết deadlock đằng sau một bug
lock-ordering thật, hay phần toán đằng sau một tham số congestion-control
bạn đã tune bằng cảm tính.

## Trạng thái: đã viết

## Files

- [`01-deadlock.md`](01-deadlock.md) — bốn điều kiện Coffman, resource allocation graph, dining philosophers, prevention/avoidance/detection
- [`02-sync-classics.md`](02-sync-classics.md) — producer-consumer, readers-writers, semaphore vs mutex một cách hình thức, bài toán barrier
- [`03-page-replacement.md`](03-page-replacement.md) — FIFO, LRU, clock/second-chance, optimal (Bélády's), thrashing và mô hình working-set
- [`04-cpu-scheduling.md`](04-cpu-scheduling.md) — FCFS, SJF/SRTF, round-robin, multi-level feedback queue, và các metric dùng để đánh giá chúng
- [`05-congestion-control-math.md`](05-congestion-control-math.md) — tăng trưởng theo cấp số của slow start, AIMD, công thức throughput, vì sao Cubic/BBR tồn tại
- [`06-queueing-theory.md`](06-queueing-theory.md) — Little's Law, M/M/1, vì sao "80% CPU" không phải là "20% headroom", trực giác Pollaczek-Khinchine
- [`07-crypto-math.md`](07-crypto-math.md) — cấu trúc round của AES, Diffie-Hellman, RSA, vì sao hash là một chiều
- [`08-amdahls-law.md`](08-amdahls-law.md) — trần speedup do phần tuần tự quyết định, cách đóng khung lại của Gustafson, vì sao tokio nhắm throughput thay vì parallelism mỗi request
- [`09-cap-flp.md`](09-cap-flp.md) — CAP theorem chọn hai trong ba dưới partition, kết quả bất khả thi FLP, vì sao mọi hệ thống consensus thật dựa vào timeout

## Nối lại với đâu

- [`01-deadlock.md`](01-deadlock.md) và [`02-sync-classics.md`](02-sync-classics.md) → [`03-rust/04-sync.md`](../03-rust/04-sync.md), [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)
- [`03-page-replacement.md`](03-page-replacement.md) → [`02-linux/09-memory.md`](../02-linux/09-memory.md)
- [`04-cpu-scheduling.md`](04-cpu-scheduling.md) → [`16-kernel/07-scheduler.md`](../16-kernel/07-scheduler.md), [`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md)
- [`05-congestion-control-math.md`](05-congestion-control-math.md) → [`01-network/08-tcp.md`](../01-network/08-tcp.md)
- [`06-queueing-theory.md`](06-queueing-theory.md) → [`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md)
- [`07-crypto-math.md`](07-crypto-math.md) → [`01-network/06-crypto-basics.md`](../01-network/06-crypto-basics.md), [`01-network/14-tls.md`](../01-network/14-tls.md)
- [`08-amdahls-law.md`](08-amdahls-law.md) → [`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md), [`04-runtime/05-runtime-comparisons.md`](../04-runtime/05-runtime-comparisons.md), [`17-performance/`](../17-performance)
- [`09-cap-flp.md`](09-cap-flp.md) → [`18-distributed/`](../18-distributed) (cả bốn file)

Mỗi file thực dụng ở trên chỉ trỏ về đây cho người đọc muốn có lý thuyết;
không file nào giả định bạn đã đọc thư mục này trước.
