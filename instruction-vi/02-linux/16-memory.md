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

### Page, fault, và "cấp phát" thực sự làm gì
Bộ nhớ được quản lý theo **page** (4 KiB). `malloc`/`Vec` hỏi allocator, allocator hỏi kernel xin address space (`brk` cho heap nhỏ, `mmap` cho các khối lớn) — và kernel trả về địa chỉ *ảo*
**mà chưa có bộ nhớ vật lý nào phía sau**. Lần đầu bạn đọc hoặc ghi mỗi page, CPU nâng một **page fault** ([`02-hardware-basics.md`](02-hardware-basics.md)); kernel cấp một physical page đã zero và map nó. Đó là
một **minor fault** (~0,1–1 µs, không đụng đĩa). Một **major fault** cần I/O: đọc một page có file backing không nằm trong page cache, hoặc mang một page đã swap-out trở lại từ đĩa (mili giây).
`minflt`/`majflt` trong `/proc/<pid>/stat` (và `perf stat -e minor-faults,major-faults`) đếm chúng. Hiệu ứng thực tế: các request *đầu tiên* sau khi khởi động chậm hơn vì buffer, code page và cache đang fault vào
("warm-up"); một `vec![0; N]` mới được kernel hỗ trợ lười bằng zero page cho tới khi ghi; và một đợt connection mới dồn dập mà mỗi cái chạm buffer mới hiện ra thành một bão fault trong thời gian CPU `sy`.
Để trả chi phí trước, hãy pre-touch bộ nhớ hoặc dùng `MAP_POPULATE`/`mlock` ([`20-limits-and-proc.md`](20-limits-and-proc.md)).

### mmap: bộ nhớ anonymous vs file-backed
`mmap` ánh xạ một thứ gì đó vào address space của bạn. Mapping **anonymous** là bộ nhớ thường (cấp phát heap lớn, stack của thread). Mapping **file-backed** là các cửa sổ nhìn vào một file qua page cache: binary
chương trình và shared library được map theo cách này (cùng các physical page được chia sẻ bởi mọi process chạy binary đó), và một static file bạn phục vụ cũng có thể
([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md), [`18-zerocopy.md`](18-zerocopy.md)). Mapping là **private** (ghi thì copy-on-write vào page riêng của bạn, vô hình với người khác) hoặc **shared** (ghi đi vào file hoặc tới các mapper khác —
[`10-ipc.md`](10-ipc.md)). Gotcha: nếu process khác truncate một file bạn đã map, chạm vào các page đã biến mất sẽ nâng **`SIGBUS`**, không phải một giá trị trả về lỗi —
lý do `mmap` các file bị sửa từ bên ngoài là nguy hiểm trong một server.

### Đọc số liệu bộ nhớ một cách trung thực
Bốn con số cho mỗi process, đều khác nhau: **VSZ** (virtual size — mọi thứ đã reserve, phần lớn vô nghĩa), **RSS** (resident — các page thực sự trong RAM, *gồm cả* shared library được đếm ở mọi process),
**PSS** (proportional — các shared page chia cho những bên chia sẻ; tổng trung thực qua các process), và **USS** (riêng của process này; thứ bạn giải phóng khi giết nó). `smaps_rollup` cho tất cả.
Toàn hệ thống, `free -h` thường bị đọc sai: **"free" không phải con số quan trọng** — kernel cố ý lấp RAM trống bằng page cache, thứ nó bỏ ngay khi cần. Hãy đọc **"available"**
(`MemAvailable` trong `/proc/meminfo`), ước lượng thứ có thể cấp phát mà không swap. Trong container, `memory.current` của cgroup gồm cả page cache; kernel sẽ reclaim cache trước khi giết bất cứ thứ gì, nên con số
mà OOM killer hành động gần với "working set" hơn (`memory.current` trừ inactive file cache) — hãy vẽ biểu đồ cái đó, không phải `memory.current` thô ([`13-containers.md`](13-containers.md)).

### Swap, reclaim, và vách đá latency
Khi bộ nhớ trống cạn, kernel **reclaim**: đầu tiên bỏ clean page cache, rồi **swap** các anonymous page ra đĩa. Một thread nền (`kswapd`) làm việc này trước khi cần; nếu bộ nhớ cạn nhanh hơn nó xử lý được,
*chính thread đang cấp phát* bị buộc vào **direct reclaim** — nó khựng bên trong `malloc` hoặc một page fault trong khi kernel giải phóng bộ nhớ — hiện ra thành các đỉnh latency nhiều mili giây không giải thích được
trên một request path không làm gì chậm. Vì thế một proxy nhạy latency thường chạy với ít hoặc không swap (một buffer đã swap-out là một request chờ đĩa) và theo dõi **memory pressure**
(`/proc/pressure/memory`, PSI) chứ không chỉ mức dùng. `vm.swappiness` thiên vị việc reclaim cache hay anonymous.

### OOM killer trong thực tế
Nếu reclaim thất bại, **OOM killer** của kernel chọn một nạn nhân theo `oom_score` (đại khái: process dùng nhiều bộ nhớ nhất, điều chỉnh được qua `oom_score_adj`, trong đó -1000 miễn trừ một process) và gửi cho nó
`SIGKILL` — không handler nào chạy, không graceful shutdown ([`17-signals.md`](17-signals.md)). Có hai đấu trường: OOM **toàn cục** khi cả máy hết bộ nhớ, và OOM **cgroup** khi một container vượt `memory.max`
([`13-containers.md`](13-containers.md)) — cái phổ biến dưới Kubernetes, hiện là `OOMKilled`, exit code **137** ([`04-process-lifecycle.md`](04-process-lifecycle.md)). Bằng chứng: `dmesg -T | grep -i 'out of memory'`, `memory.events` (bộ đếm `oom_kill`).
Bài học riêng cho proxy là mức dùng bộ nhớ là *buffer theo từng connection nhân số connection*: buffer đọc 64 KiB + ghi 64 KiB cho mỗi connection ở 100.000 connection là 12,5 GB trước khi một request body nào được buffer. Hãy chặn
kích thước buffer, kích thước request body và số connection đồng thời một cách có chủ đích ([`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md), [`07-security/10-slowloris.md`](../07-security/10-slowloris.md)), và lập ngân sách
*limit = steady state + burst + overhead của allocator* thay vì hy vọng.

### Huge page (ngắn gọn)
Một TLB miss tốn một lần duyệt page-table, nên workload chạm vào bộ nhớ lớn được lợi từ **huge page** (2 MiB thay vì 4 KiB — ít hơn 512 lần entry TLB). **Transparent Huge Pages** (THP) làm việc này tự động,
với giá là việc compaction nền có thể gây khựng latency; nhiều hệ thống nhạy latency đặt THP thành `madvise` hoặc `never`
(`/sys/kernel/mm/transparent_hugepage/enabled`) và chỉ opt-in nơi đã đo thấy có ích ([`17-performance/`](../17-performance)).

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
[`02-linux/18-zerocopy.md`](18-zerocopy.md)) nhanh cho các static asset được phục vụ lặp
lại — dữ liệu thường đã resident sẵn, và kernel copy page-cache-tới-socket
mà hoàn toàn không đi vòng qua buffer userspace của process bạn. Điều
này ảnh hưởng trực tiếp tới cách [`05-http-stack/06-static.md`](../05-http-stack/06-static.md) nên phục vụ
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
hơn. Đây là một khác biệt thật, đo được cho [`proxy`](../../proxy) dưới thông lượng bền
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
4. Đổi allocator của [`proxy`](../../proxy) sang `mimalloc` qua `#[global_allocator]` và
   benchmark xử lý request nặng về cấp phát trước/sau.
5. Nếu bạn có quyền truy cập một máy nhiều socket, chạy `numactl
   --hardware` và giải thích một truy cập memory "remote" sẽ tốn gì so
   với local.
6. Quan sát fault và OOM killer: chạy một chương trình scratch chạm 1 GiB từng page và đọc `minflt` từ `/proc/<pid>/stat` (hoặc `perf stat -e minor-faults`); rồi chạy nó trong một container với `--memory=256m` (và `--memory-swap=256m`) cấp phát cho tới khi bị giết, và xác nhận exit code 137, `dmesg -T | grep -i oom`, và bộ đếm `oom_kill` trong `memory.events` của cgroup.
