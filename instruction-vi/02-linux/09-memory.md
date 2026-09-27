# Memory

Virtual memory, page cache, NUMA.

## What to learn

### Cơ bản về virtual memory
Mỗi process nhận một virtual address space riêng; page table của kernel
ánh xạ virtual page tới physical frame (hoặc tới "chưa có, fault cho
tôi"). Đây là lý do `fork()` rẻ (page table copy-on-write, không phải
memory), vì sao buffer của một `Vec` có thể được resize mà OS không thực
sự zero hàng gigabyte ngay từ đầu (page lazy/overcommitted), và vì sao
RSS và virtual size là hai con số khác nhau đều quan trọng khi bạn định
cỡ giới hạn memory của một proxy.

### Overcommit và OOM
Mặc định Linux cho phép cấp phát ảo vượt quá physical + swap
(`vm.overcommit_memory`); process chỉ trả giá cho một trang khi nó được
ghi lần đầu. Gotcha: một proxy pre-allocate các buffer pool lớn có thể
*trông* ổn trên `malloc` rồi bị OOM-killed dưới tải một khi các buffer đó
thực sự bị đụng tới — giới hạn memory `cgroup` (phổ biến dưới Kubernetes)
enforce dựa trên RSS, không phải virtual size, nên hãy theo dõi memory
resident thật, không phải những gì `Vec::with_capacity` "đã reserve."

### Page cache và static file
Kernel giữ dữ liệu file vừa đọc trong RAM dưới dạng page cache, backing
cả `read()` và `mmap()`. Đây là lý do `sendfile()` (xem
`02-linux/11-zerocopy.md`) nhanh cho các static asset được phục vụ lặp
lại — dữ liệu thường đã resident sẵn, và kernel copy page-cache-tới-socket
mà hoàn toàn không đi vòng qua buffer userspace của process bạn. Điều
này ảnh hưởng trực tiếp tới cách `05-http-stack/05-static.md` nên phục vụ
file: để cache của kernel tự làm việc caching thay vì tự implement lại
một LRU ở userspace cho dữ liệu lạnh vốn đã nóng sẵn trong page cache.

### NUMA
Trên máy nhiều socket, memory được phân vùng theo từng socket ("node"),
và truy cập memory trên bộ điều khiển của một node *khác* tốn latency cao
hơn đáng kể so với memory cùng node. Các kiến trúc proxy thread-per-core /
shard-per-core (liên quan nếu bạn từng chuyển khỏi scheduler
work-stealing của tokio sang thứ gì đó như `glommio`) muốn mỗi core cấp
phát và giải phóng memory gắn với NUMA node của chính nó — `numactl
--hardware` cho bạn thấy topology, và `numactl --cpunodebind=N
--membind=N` là cách thô để pin một process trong lúc thử nghiệm.

### Vì sao lựa chọn allocator quan trọng dưới tải
Allocator glibc mặc định có arena theo từng thread có thể phân mảnh nặng
dưới các mẫu cấp phát multi-threaded, churn cao (ví dụ một buffer
request/response được cấp phát-rồi-giải-phóng ở mỗi kết nối). Các proxy
thường chuyển sang `jemalloc` hay `mimalloc` (crate `tikv-jemallocator`/
`mimalloc` trong Rust) để có tail latency dễ đoán hơn và phân mảnh thấp
hơn. Đây là một khác biệt thật, đo được cho `proxy` dưới thông lượng bền
vững, không phải một micro-optimization.

## Practice
1. Chạy `/proc/self/status` (`VmRSS` so với `VmSize`) trong một chương
   trình Rust nhỏ trước/sau một cấp phát `Vec::with_capacity` lớn,
   trước/sau khi thực sự ghi vào nó.
2. Tái tạo gotcha overcommit: cấp phát nhiều virtual memory hơn RAM vật
   lý, xác nhận nó "thành công," rồi ghi vào nó và xem RSS leo thang (làm
   việc này trong một container/VM có giới hạn memory, không phải máy
   chính của bạn).
3. Dùng `/proc/self/smaps` hoặc `pmap` để kiểm tra memory map của một
   proxy đang chạy và xác định vùng nào được page-cache backing so với
   vùng anonymous.
4. Đổi allocator của `proxy` sang `mimalloc` qua `#[global_allocator]` và
   benchmark xử lý request nặng về cấp phát trước/sau.
5. Nếu bạn có quyền truy cập một máy nhiều socket, chạy `numactl
   --hardware` và giải thích một truy cập memory "remote" sẽ tốn gì so
   với local.
