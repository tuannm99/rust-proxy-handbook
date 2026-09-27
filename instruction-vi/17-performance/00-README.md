# Performance

Tuning hiệu năng ở mức hardware — cách CPU và memory subsystem thực sự
hoạt động, khác với các kỹ thuật zero-copy ở mức syscall trong
[`02-linux/11-zerocopy.md`](../02-linux/11-zerocopy.md) (thư mục này cross-reference tới đó thay vì lặp
lại).

## Trạng thái: đã viết, nhưng hãy đọc sau khi profile

Nội dung bên dưới đã được viết và được cross-reference từ
[`14-memory/06-fragmentation.md`](../14-memory/06-fragmentation.md) (size class của struct) và
[`14-memory/05-slab-allocator.md`](../14-memory/05-slab-allocator.md) (cache locality). Đọc nó **sau khi**
profiling ([`08-observability/04-profiling.md`](../08-observability/04-profiling.md)) đã chỉ một flamegraph vào
một hot path đáng để tối ưu — mọi file ở đây đều mở đầu bằng việc nhấn
mạnh điều đó. Đọc về cache line và false sharing trước khi bạn có một phép
đo chỉ vào chúng sẽ tạo ra các micro-optimization cho code chưa bao giờ là
bottleneck. Điểm kích hoạt tự nhiên là [`proxy`](../../proxy) dưới tải của
[`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md), không phải một lab cụ thể nào.

## Files

- [`01-cpu-cache.md`](01-cpu-cache.md) — cache line, L1/L2/L3, vì sao data layout ảnh hưởng tới latency của hot path
- [`02-false-sharing.md`](02-false-sharing.md) — hai atomic không liên quan trên cùng một cache line âm thầm serialize hóa code "lock-free" của bạn
- [`03-numa.md`](03-numa.md) — non-uniform memory access, pin thread/bộ nhớ vào một node trên máy nhiều socket
- [`04-memory-layout.md`](04-memory-layout.md) — thứ tự field của struct, padding, `#[repr(C)]` so với layout mặc định của Rust
- [`05-branch-prediction.md`](05-branch-prediction.md) — vì sao branch khó đoán trên hot path tốn kém hơn vẻ ngoài của nó
- [`06-simd.md`](06-simd.md) — thao tác vector hóa, chỗ chúng xuất hiện trong một proxy (parse header, checksum)

I/O zero-copy (`sendfile`/`splice`/`mmap`) được nói ở
[`02-linux/11-zerocopy.md`](../02-linux/11-zerocopy.md), không lặp lại ở đây.
