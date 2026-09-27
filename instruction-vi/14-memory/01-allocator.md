# General-Purpose Allocators

[`14-memory/06-fragmentation.md`](06-fragmentation.md) nói về việc gì xảy ra theo thời gian. File
này nói về thứ thực sự đứng sau `malloc`/global allocator của Rust, và vì
sao đổi nó là một trong những thay đổi đơn lẻ có đòn bẩy cao nhất cho một
proxy multi-threaded.

## What to learn

### Size class: cách allocator tự chia nhóm
Một general-purpose allocator không trả về đúng chính xác kích thước được
yêu cầu; nó làm tròn mỗi request lên một trong các size class cố định
(ví dụ 8, 16, 32, 48, 64, 96, 128, ... byte) và phục vụ từ một free list
của class đó. Cách này giới hạn fragmentation thành "lãng phí bên trong
một size class" thay vì external fragmentation tùy tiện, đổi lại là một ít
lãng phí nội bộ — xem [`06-fragmentation.md`](06-fragmentation.md) để biết failure mode cụ thể mà
cách này đánh đổi.

### Thread-local arena: tránh một lock toàn cục
Một free list toàn cục duy nhất sẽ serialize hóa mọi allocation trên mọi
thread trong một proxy multi-threaded — một lock nằm ngay trên hot path
nóng nhất có thể. Các allocator production (jemalloc, mimalloc, tcmalloc)
cho mỗi thread arena riêng với free list riêng, nên phần lớn alloc/free
không bao giờ chạm vào lock chung; free chéo giữa các thread (buffer được
cấp phát trên thread này, free trên thread khác — phổ biến trong một async
runtime di chuyển task giữa các worker thread) là điểm cần phối hợp còn
lại, mỗi allocator xử lý theo thiết kế riêng.

### Trait `GlobalAlloc` của Rust và cách thay thế nó
```rust
use std::alloc::{GlobalAlloc, Layout, System};

struct MyAllocator; // bọc mimalloc, jemalloc, hoặc allocator tự viết
unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 { /* ... */ System.alloc(layout) }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) { /* ... */ System.dealloc(ptr, layout) }
}

#[global_allocator]
static GLOBAL: MyAllocator = MyAllocator;
```
Trong thực tế bạn nên dùng crate `mimalloc` hoặc `tikv-jemallocator` thay
vì tự viết tay — đoạn code trên chỉ để minh họa `#[global_allocator]`
thực sự thay thế cái gì: mọi allocation của `Box`, `Vec`, `String` trong
toàn bộ binary, trên toàn process, không có chỗ nào opt-out theo từng
call site.

### Vì sao default của glibc thường là lựa chọn sai cho một proxy
`ptmalloc` của glibc là một allocator tổng quát hợp lý nhưng khá bảo thủ
trong việc trả bộ nhớ lại cho OS, và nhân số arena per-thread lên dưới áp
lực tranh chấp ([`06-fragmentation.md`](06-fragmentation.md) nói về khía cạnh `MALLOC_ARENA_MAX`).
mimalloc và jemalloc đều được thiết kế đúng cho loại workload mà một proxy
có — nhiều allocation nhỏ, ngắn hạn, trên nhiều thread — và luôn benchmark
tốt hơn trên loại tải này; đây là một quyết định đáng để đưa ra một cách
chủ động thay vì kế thừa mặc định.

### Gotcha: benchmark đúng dạng tranh chấp thực tế
Một microbenchmark alloc/free đơn luồng trong một vòng lặp gần như không
nói lên được điều gì về allocator nào thắng dưới tải thực tế của một
proxy: nhiều thread, alloc/free ở tốc độ khác nhau, object đôi khi được
free trên một thread khác với thread đã cấp phát nó. Benchmark với traffic
đồng thời thật của [`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md), không phải một vòng
lặp đơn luồng tổng hợp, trước khi chọn.

## Practice
1. Đổi global allocator của [`proxy`](../../proxy) (hoặc một crate trong [`labs/`](../../labs)) sang
   `mimalloc` qua `#[global_allocator]` và xác nhận binary vẫn build và
   test vẫn pass.
2. Benchmark một hot path nặng về allocation (ví dụ parse header trên mỗi
   request trong [`labs/01-http-parser`](../../labs/01-http-parser)) dưới tải đồng thời với system
   allocator, rồi mimalloc, rồi jemalloc; so sánh throughput và tail
   latency, không chỉ thời gian allocation trung bình.
3. Cố tình tái tạo pattern free chéo giữa các thread (cấp phát trên một
   tokio worker, gửi value qua channel, free trên worker khác) và kiểm tra
   xem docs của allocator bạn chọn có nói đây là một slow path hay không.
4. Chạy lại thí nghiệm RSS-theo-thời-gian của [`14-memory/06-fragmentation.md`](06-fragmentation.md)
   với allocator bạn chọn và so sánh mức plateau với system allocator.
