# Memory

Quản lý bộ nhớ ở mức allocator và layout, sâu hơn một bậc so với phần xử lý
ở mức OS của `02-linux/09-memory.md` (virtual memory, paging). Đây là về
việc process của bạn *làm gì* với bộ nhớ đã được cấp cho nó.

## Trạng thái: đã viết; thứ tự hợp lý là allocator → arena → object-pool → buffer-pool → slab-allocator → fragmentation

`01-allocator.md` trước (trường hợp tổng quát), rồi tới hai chiến lược
theo phạm vi request đánh đổi với nó (`02-arena.md`, `03-object-pool.md`,
`04-buffer-pool.md` — file cuối dựa trên pattern của `03-object-pool.md`),
sau đó `05-slab-allocator.md` (bổ sung cho `13-algorithms/slab.md`), và
`06-fragmentation.md` cuối cùng vì nó giải thích chính cái failure mode mà
tất cả các phần trên tồn tại để tránh.

## Đã viết

- `06-fragmentation.md` — vì sao process chạy lâu dài xuống cấp theo thời gian, và pooling/arena tránh nó thế nào
- `05-slab-allocator.md` — cấp phát theo size-class cố định, per-CPU cache, typed pool; bổ sung cho `13-algorithms/slab.md`
- `01-allocator.md` — size class, thread-local arena, `#[global_allocator]`, vì sao default của glibc thường không phải lựa chọn đúng
- `02-arena.md` — bump allocation cho dữ liệu theo phạm vi request, giải phóng hàng loạt
- `03-object-pool.md` — tái sử dụng object đã cấp phát trên heap (buffer, connection struct) thay vì alloc/free mỗi request
- `04-buffer-pool.md` — pooling byte buffer riêng cho I/O, các tier theo size-class, liên kết với `02-linux/11-zerocopy.md`
