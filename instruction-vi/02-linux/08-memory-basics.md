# Memory Basics

Một phần của chuỗi fundamentals từ-con-số-0 — xem
[`02-linux/01-fundamentals.md`](01-fundamentals.md) để có index đầy đủ. Cố tình viết ngắn gọn:
[`02-linux/16-memory.md`](16-memory.md) mới là nơi virtual memory, overcommit, page cache,
và NUMA được phát triển sâu thật sự — file này tồn tại chỉ để bạn không
bắt đầu từ con số 0.

## What to learn

### Mỗi process có address space ảo riêng
"Thế giới riêng của nó" của một process ([`03-processes-and-threads.md`](03-processes-and-threads.md))
không phải RAM vật lý trực tiếp — đó là một address space **ảo** mà
kernel ánh xạ tới physical memory (hoặc tới "chưa có, fault vào khi truy
cập lần đầu") qua page table. Chương trình của bạn chỉ bao giờ thấy địa
chỉ ảo; việc ánh xạ tới RAM thật là việc của kernel, vô hình với bạn trừ
hệ quả về hiệu năng. Hai process đều có thể dùng địa chỉ ảo `0x1000` cho
dữ liệu hoàn toàn khác nhau, một cách an toàn, vì page table của kernel
ánh xạ `0x1000` của mỗi process tới physical memory khác nhau.

Đó thực sự là tất cả những gì bạn cần ở đây để đoạn mở đầu của
[`02-linux/16-memory.md`](16-memory.md) — "mỗi process nhận một virtual address space
riêng; page table của kernel ánh xạ virtual page tới physical frame" —
cảm giác như một lời nhắc lại thay vì thông tin mới.

### Thứ bậc memory: register, cache, RAM, disk
Đại khái, từ nhanh nhất/nhỏ nhất/đắt nhất theo byte tới chậm nhất/lớn
nhất/rẻ nhất: **register** của CPU, rồi **cache** (L1/L2/L3, vài MB, xây
sẵn trong CPU), rồi **RAM** (hàng gigabyte, vẫn volatile — mất khi tắt
nguồn), rồi **disk/SSD** (lớn hơn nhiều, chậm hơn nhiều, persistent). Mỗi
tầng đóng vai trò cache cho tầng bên dưới nó: RAM cache nội dung disk
(phần page cache trong [`16-memory.md`](16-memory.md) chính xác là điều này), CPU cache
cache nội dung RAM.

Con số quan trọng nhất trong thực tế: một L1 cache hit tốn khoảng
1 nanosecond; một truy cập RAM tốn khoảng 100 nanosecond; một truy cập
SSD tốn hàng chục *microsecond*; một seek trên đĩa quay tốn hàng
*millisecond* — mỗi bước xuống chậm hơn khoảng 1-2 bậc độ lớn. Đây là
toàn bộ động lực đằng sau sự tồn tại của [`17-performance/01-cpu-cache.md`](../17-performance/01-cpu-cache.md)
(các lựa chọn data layout giữ dữ liệu nóng trong cache) và đằng sau vì
sao page cache trong [`02-linux/16-memory.md`](16-memory.md) quan trọng đến vậy với một
proxy phục vụ static file: một cache hit ở đó là một truy cập RAM; một
cache miss là một truy cập disk, chậm hơn 100-1000 lần.

### Stack vs heap, ngắn gọn
Mỗi thread ([`03-processes-and-threads.md`](03-processes-and-threads.md)) có **stack** riêng — một vùng
được quản lý tự động, hướng cố định, cho biến local và call frame của
hàm, allocate nhanh (chỉ di chuyển một pointer) và tự động giải phóng khi
hàm return. **Heap** được chia sẻ giữa mọi thread trong một process, dùng
cho bất cứ thứ gì cần sống lâu hơn hàm đã tạo ra nó hoặc có kích thước
không biết tại compile time — `Box`, `Vec`, `String` của Rust đều allocate ở đây. Allocate heap chậm hơn allocate stack (nó đi qua một
allocator, đôi khi một syscall — [`14-memory/01-allocator.md`](../14-memory/01-allocator.md)) và không
tự giải phóng theo cách một stack frame làm; trong Rust, `Drop` là thứ
gắn việc giải phóng heap với việc kết thúc một scope, mà không cần garbage
collector.

### Vì sao điều này quan trọng riêng với một proxy
Toàn bộ profile hiệu năng của một proxy là một câu chuyện về thứ bậc
memory: giữ một route table nóng đủ nhỏ để nằm gọn trong cache
([`17-performance/01-cpu-cache.md`](../17-performance/01-cpu-cache.md)), để page cache của kernel hấp thụ các
lần đọc static-file lặp lại thay vì tự implement lại cache đó ở userspace
([`16-memory.md`](16-memory.md)), và tránh allocate heap không cần thiết trên hot path
của request ([`14-memory/02-arena.md`](../14-memory/02-arena.md), [`14-memory/03-object-pool.md`](../14-memory/03-object-pool.md)) đều
là các biến thể khác nhau của "giữ dữ liệu càng gần đỉnh thứ bậc này càng
tốt, càng lâu càng tốt."

## Practice
1. Chạy `free -h` và xác định tổng RAM, đã dùng, và "available" (không
   giống "free" — [`16-memory.md`](16-memory.md) giải thích vì sao khi bạn tới đó).
2. Viết một chương trình Rust nhỏ allocate một `Vec<u8>` lớn bằng
   `with_capacity` (chỉ reserve virtual memory) so với một phiên bản còn
   ghi vào từng byte (buộc physical page phải backing nó) — theo dõi
   `VmRSS` trong `/proc/self/status` trước/sau mỗi bước và ghi lại phiên
   bản nào thực sự làm RSS tăng.
3. Tra cứu (hoặc đo, bằng một micro-benchmark) latency xấp xỉ của một L1
   cache hit, một truy cập RAM, và một lần đọc SSD trên lớp phần cứng của
   chính máy bạn — ghi lại ba con số đó và tỉ lệ giữa chúng.
4. Đọc phần mở đầu của [`14-memory/02-arena.md`](../14-memory/02-arena.md) ngay khi stack-vs-heap còn
   mới, và giải thích trong một câu vì sao bump allocation gần với tinh
   thần của stack allocation hơn là một heap allocator đa dụng.
