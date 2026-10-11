# Memory Basics

File này thuộc nhóm fundamentals. Xem [`02-linux/01-fundamentals.md`](01-fundamentals.md) để có mục lục đầy đủ. File được cố
tình viết ngắn. [`02-linux/16-memory.md`](16-memory.md) mới là nơi virtual memory, overcommit, page cache và NUMA được đi sâu.
File này chỉ có mặt để bạn không bắt đầu từ con số 0.

## What to learn

### Mỗi process có address space ảo riêng
"Thế giới riêng" của một process ([`03-processes-and-threads.md`](03-processes-and-threads.md)) không phải là RAM vật lý trực
tiếp. Đó là một address space **ảo**, được kernel ánh xạ tới bộ nhớ vật lý qua page table (hoặc ánh xạ tới trạng thái "chưa có,
lỗi page fault khi truy cập lần đầu").

Chương trình của bạn chỉ bao giờ thấy địa chỉ ảo. Việc ánh xạ tới RAM thật là việc của kernel, và bạn chỉ cảm nhận được nó qua
hệ quả về hiệu năng. Hai process có thể cùng dùng địa chỉ ảo `0x1000` cho dữ liệu hoàn toàn khác nhau một cách an toàn, vì
page table của kernel ánh xạ `0x1000` của mỗi process tới vùng bộ nhớ vật lý khác nhau.

Bạn chỉ cần đến đây là đủ để câu mở đầu của [`02-linux/16-memory.md`](16-memory.md) ("mỗi process nhận một virtual address space
riêng; page table của kernel ánh xạ virtual page tới physical frame") nghe như lời nhắc lại, không phải thông tin mới.

### Thứ bậc bộ nhớ: register, cache, RAM, disk
Đại khái, đi từ nhanh nhất/nhỏ nhất/đắt nhất theo byte tới chậm nhất/lớn nhất/rẻ nhất:

1. **Register** của CPU.
2. **Cache** (L1/L2/L3, vài MB, nằm sẵn trong CPU).
3. **RAM** (hàng gigabyte, vẫn volatile: mất dữ liệu khi tắt nguồn).
4. **Disk/SSD** (lớn hơn nhiều, chậm hơn nhiều, bền vững).

Mỗi tầng đóng vai trò cache cho tầng bên dưới. RAM cache nội dung của disk (page cache trong
[`16-memory.md`](16-memory.md) chính là việc này). CPU cache cache nội dung của RAM.

Các con số quan trọng nhất trong thực tế:
- Một L1 cache hit tốn khoảng 1 nanosecond.
- Một truy cập RAM tốn khoảng 100 nanosecond.
- Một truy cập SSD tốn hàng chục *microsecond*.
- Một seek trên đĩa quay tốn hàng *millisecond*.

Mỗi bước xuống chậm hơn khoảng 1–2 bậc độ lớn. Đây là toàn bộ động lực của
[`17-performance/01-cpu-cache.md`](../17-performance/01-cpu-cache.md) (các lựa chọn data layout để giữ dữ liệu nóng trong
cache), và là lý do page cache trong [`02-linux/16-memory.md`](16-memory.md) quan trọng với một proxy phục vụ static file: một
cache hit ở đó là một truy cập RAM, còn một cache miss là một truy cập disk, chậm hơn 100–1000 lần.

### Stack và heap, nói ngắn gọn
Mỗi thread ([`03-processes-and-threads.md`](03-processes-and-threads.md)) có **stack** riêng. Đó là một vùng được quản lý tự
động, theo thứ tự cố định, dùng cho biến local và call frame của hàm. Allocate trên stack nhanh (chỉ di chuyển một pointer) và
tự giải phóng khi hàm return.

**Heap** được chia sẻ giữa mọi thread trong một process. Nó dùng cho bất cứ thứ gì cần sống lâu hơn hàm đã tạo ra nó, hoặc có
kích thước không biết lúc compile. `Box`, `Vec`, `String` của Rust allocate ở đây. Allocate trên heap chậm hơn stack (nó đi qua
allocator, đôi khi cần syscall, xem [`14-memory/01-allocator.md`](../14-memory/01-allocator.md)) và không tự giải phóng như một
stack frame. Trong Rust, `Drop` gắn việc giải phóng heap với lúc kết thúc scope, mà không cần garbage collector.

### Vì sao điều này quan trọng riêng với proxy
Toàn bộ profile hiệu năng của proxy là một câu chuyện về thứ bậc bộ nhớ:
- Giữ route table nóng đủ nhỏ để nằm gọn trong cache ([`17-performance/01-cpu-cache.md`](../17-performance/01-cpu-cache.md)).
- Để page cache của kernel hấp thụ các lần đọc static file lặp lại, thay vì tự cài lại cache đó trong userspace
  ([`16-memory.md`](16-memory.md)).
- Tránh allocate heap không cần thiết trên hot path của request
  ([`14-memory/02-arena.md`](../14-memory/02-arena.md), [`14-memory/03-object-pool.md`](../14-memory/03-object-pool.md)).

Cả ba đều là các biến thể của cùng một nguyên tắc: giữ dữ liệu càng gần đỉnh của thứ bậc này càng lâu càng tốt.

## Practice
1. Chạy `free -h` và xác định tổng RAM, phần đã dùng, và "available" (không giống "free"; [`16-memory.md`](16-memory.md) giải
   thích vì sao khi bạn tới đó).
2. Viết một chương trình Rust nhỏ allocate một `Vec<u8>` lớn bằng `with_capacity` (chỉ reserve bộ nhớ ảo). Viết thêm một phiên
   bản ghi vào từng byte (buộc physical page phải có thật). Theo dõi `VmRSS` trong `/proc/self/status` trước và sau mỗi bước,
   và ghi lại phiên bản nào thực sự làm RSS tăng.
3. Tra cứu (hoặc đo bằng micro-benchmark) latency xấp xỉ của một L1 cache hit, một truy cập RAM và một lần đọc SSD trên phần
   cứng máy bạn. Ghi lại ba con số đó và tỷ lệ giữa chúng.
4. Đọc phần mở đầu của [`14-memory/02-arena.md`](../14-memory/02-arena.md) ngay khi khái niệm stack và heap còn mới. Giải thích
   trong một câu vì sao bump allocation gần với tinh thần của stack allocation hơn là một heap allocator đa dụng.
