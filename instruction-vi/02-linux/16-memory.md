# Memory

Virtual memory, page cache, NUMA.

## What to learn

### Cơ bản về virtual memory
Mỗi process có một virtual address space riêng. Page table của kernel ánh xạ mỗi virtual page tới một
physical frame, hoặc tới trạng thái "chưa có, hãy báo fault cho tôi". Điều này giải thích nhiều chuyện:

- `fork()` rẻ, vì page table được copy kiểu copy-on-write chứ không copy bộ nhớ.
- Buffer của một `Vec` có thể được resize mà OS không phải zero hàng gigabyte ngay từ đầu, vì các page được
  cấp lười (lazy/overcommitted).
- RSS và virtual size là hai con số khác nhau, và cả hai đều quan trọng khi bạn chọn giới hạn memory cho
  proxy.

Về lý thuyết: khi physical frame cạn, kernel loại bỏ page nào (FIFO, LRU, clock) và vì sao xảy ra thrashing? Xem
[`22-theory/03-page-replacement.md`](../22-theory/03-page-replacement.md).

### Page, fault, và "allocate" thực sự làm gì
Bộ nhớ được quản lý theo **page** (4 KiB). `malloc` hay `Vec` hỏi allocator, allocator xin kernel thêm
address space (`brk` cho heap nhỏ, `mmap` cho các khối lớn). Kernel trả về một địa chỉ *ảo*
**mà chưa có bộ nhớ vật lý nào phía sau**.

Lần đầu tiên bạn đọc hoặc ghi mỗi page, CPU phát ra một **page fault**
([`02-hardware-basics.md`](02-hardware-basics.md)). Kernel cấp một physical page đã được zero và map nó vào.
Có hai loại fault:

- **Minor fault** (~0,1–1 µs, không đụng đĩa).
- **Major fault** cần I/O: đọc một page có file backing mà chưa nằm trong page cache, hoặc đưa một page đã bị
  swap-out quay lại từ đĩa (mất vài mili giây).

`minflt`/`majflt` trong `/proc/<pid>/stat` (và `perf stat -e minor-faults,major-faults`) đếm chúng.

Hiệu ứng thực tế:
- Các request *đầu tiên* sau khi khởi động chậm hơn, vì buffer, code page và cache đang fault vào dần
  ("warm-up").
- Một `vec![0; N]` mới được kernel hỗ trợ lười bằng zero page cho đến khi bạn ghi.
- Khi một đợt connection mới dồn tới, và mỗi connection chạm vào buffer mới, nó hiện ra thành một cơn bão
  fault (fault storm) trong thời gian CPU `sy`.

Để trả chi phí này trước, hãy pre-touch bộ nhớ, hoặc dùng `MAP_POPULATE`/`mlock`
([`20-limits-and-proc.md`](20-limits-and-proc.md)).

### mmap: bộ nhớ anonymous và file-backed
`mmap` ánh xạ một thứ gì đó vào address space của bạn.

- Mapping **anonymous** là bộ nhớ thông thường (heap lớn, stack của thread).
- Mapping **file-backed** là cửa sổ nhìn vào một file qua page cache. Binary chương trình và shared library
  được map theo cách này, và các physical page được chia sẻ giữa mọi process chạy binary đó. Một static file
  bạn phục vụ cũng có thể được map ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md),
  [`18-zerocopy.md`](18-zerocopy.md)).

Mapping là **private** (ghi thì copy-on-write vào page riêng của bạn, người khác không thấy) hoặc **shared**
(ghi đi vào file, hoặc tới các process khác cùng map, xem [`10-ipc.md`](10-ipc.md)).

Gotcha: nếu một process khác truncate file mà bạn đã map, khi bạn chạm vào các page đã biến mất sẽ nhận
**`SIGBUS`**, chứ không phải một giá trị lỗi trả về. Vì vậy `mmap` các file có thể bị sửa từ bên ngoài là nguy
hiểm trong một server.

### Đọc số liệu bộ nhớ cho đúng
Mỗi process có bốn con số khác nhau:

- **VSZ** (virtual size): mọi thứ đã reserve, phần lớn vô nghĩa.
- **RSS** (resident): các page thực sự đang nằm trong RAM. Con số này *bao gồm cả* shared library, bị đếm ở
  mọi process dùng chung.
- **PSS** (proportional): các shared page được chia cho những bên chia sẻ. Tổng PSS qua các process là con số
  trung thực.
- **USS** (unique): phần riêng của process này, tức là thứ bạn lấy lại được khi giết nó.

`smaps_rollup` cho bạn cả bốn.

Trên toàn hệ thống, `free -h` thường bị đọc sai: **"free" không phải con số quan trọng**. Kernel cố ý dùng RAM
trống để làm page cache, và nhả nó ra ngay khi cần. Hãy đọc **"available"** (`MemAvailable` trong
`/proc/meminfo`), là ước lượng lượng bộ nhớ có thể allocate mà không cần swap.

Trong container, `memory.current` của cgroup bao gồm cả page cache. Kernel sẽ reclaim cache trước khi giết bất
cứ thứ gì. Vì vậy con số mà OOM killer phản ứng gần với "working set" hơn (`memory.current` trừ đi inactive file
cache). Hãy vẽ biểu đồ con số đó, không phải `memory.current` thô ([`13-containers.md`](13-containers.md)).

### Swap, reclaim và vách đá latency
Khi bộ nhớ trống cạn, kernel **reclaim**: đầu tiên bỏ các clean page cache, rồi **swap** các anonymous page ra
đĩa. Một thread nền (`kswapd`) làm việc này từ trước khi cần.

Nếu bộ nhớ cạn nhanh hơn `kswapd` xử lý kịp, *chính thread đang allocate* bị buộc vào **direct reclaim**. Nó bị
đứng bên trong `malloc` hoặc một page fault trong lúc kernel giải phóng bộ nhớ. Kết quả là các đỉnh latency kéo
dài nhiều mili giây không giải thích được, trên một request path vốn không có gì chậm.

Vì vậy một proxy nhạy latency thường chạy với ít swap hoặc không swap (một buffer đã bị swap-out nghĩa là request
phải chờ đĩa). Nó cũng theo dõi **memory pressure** (`/proc/pressure/memory`, PSI) chứ không chỉ mức dùng bộ nhớ.
`vm.swappiness` quyết định kernel thiên về reclaim cache hay reclaim anonymous page.

### OOM killer trong thực tế
Nếu reclaim thất bại, **OOM killer** của kernel chọn một nạn nhân theo `oom_score` (đại khái: process dùng nhiều
bộ nhớ nhất; có thể chỉnh qua `oom_score_adj`, trong đó -1000 miễn trừ hoàn toàn một process) và gửi `SIGKILL`.
Không có handler nào chạy, không có graceful shutdown ([`17-signals.md`](17-signals.md)).

Có hai nơi OOM xảy ra: OOM **toàn cục** khi cả máy hết bộ nhớ, và OOM **cgroup** khi một container vượt
`memory.max` ([`13-containers.md`](13-containers.md)). Loại thứ hai phổ biến dưới Kubernetes, hiện là `OOMKilled`
với exit code **137** ([`04-process-lifecycle.md`](04-process-lifecycle.md)). Bằng chứng nằm ở
`dmesg -T | grep -i 'out of memory'` và bộ đếm `oom_kill` trong `memory.events`.

Bài học riêng cho proxy: mức dùng bộ nhớ là *buffer của từng connection nhân với số connection*. Buffer đọc 64 KiB
+ buffer ghi 64 KiB cho mỗi connection, ở 100.000 connection, là 12,5 GB trước khi có request body nào được
buffer. Hãy đặt trần có chủ đích cho kích thước buffer, kích thước request body và số connection đồng thời
([`07-security/11-load-shedding.md`](../07-security/11-load-shedding.md),
[`07-security/10-slowloris.md`](../07-security/10-slowloris.md)). Hãy lập ngân sách theo công thức
*limit = steady state + burst + overhead của allocator*, thay vì hy vọng may mắn.

### Huge page (nói ngắn gọn)
Một TLB miss tốn một lần duyệt page table. Vì vậy workload chạm vào nhiều bộ nhớ được lợi từ **huge page** (2 MiB
thay vì 4 KiB, ít hơn 512 lần số entry TLB cần dùng). **Transparent Huge Pages** (THP) làm việc này tự động.
Cái giá là việc compaction chạy nền có thể làm latency bị đứng. Nhiều hệ thống nhạy latency đặt THP thành
`madvise` hoặc `never` (`/sys/kernel/mm/transparent_hugepage/enabled`), và chỉ bật ở nơi đã đo thấy có ích
([`17-performance/`](../17-performance)).

### Overcommit và OOM
Mặc định Linux cho phép allocate bộ nhớ ảo vượt quá physical + swap (`vm.overcommit_memory`). Process chỉ trả giá
cho một page khi nó được ghi lần đầu.

Gotcha: một proxy pre-allocate các buffer pool lớn có thể *trông* ổn lúc `malloc`, rồi bị OOM-kill dưới tải khi
các buffer đó thực sự bị chạm tới. Giới hạn memory của `cgroup` (phổ biến dưới Kubernetes) được tính theo RSS,
không phải virtual size. Vì vậy hãy theo dõi memory resident thật, không phải những gì `Vec::with_capacity` "đã
reserve".

### Page cache và static file
Kernel giữ dữ liệu file vừa đọc trong RAM dưới dạng page cache, dùng chung cho cả `read()` và `mmap()`. Đó là lý
do `sendfile()` (xem [`02-linux/18-zerocopy.md`](18-zerocopy.md)) nhanh với các static asset được phục vụ lặp
lại: dữ liệu thường đã nằm sẵn trong RAM, và kernel copy thẳng từ page cache tới socket mà không đi vòng qua
buffer userspace của process bạn.

Điều này ảnh hưởng trực tiếp đến cách [`05-http-stack/06-static.md`](../05-http-stack/06-static.md) nên phục vụ
file: hãy để cache của kernel làm việc caching, thay vì tự cài lại một LRU ở userspace cho dữ liệu vốn đã nóng sẵn
trong page cache.

### NUMA
Trên máy nhiều socket, bộ nhớ được chia theo từng socket (gọi là "node"). Truy cập bộ nhớ nằm trên bộ điều khiển
của một node *khác* có latency cao hơn đáng kể so với bộ nhớ cùng node.

Các kiến trúc proxy thread-per-core hoặc shard-per-core (liên quan nếu bạn chuyển từ scheduler work-stealing của
tokio sang thứ như `glommio`) muốn mỗi core allocate và giải phóng bộ nhớ gắn với NUMA node của chính nó.
`numactl --hardware` cho bạn xem topology. `numactl --cpunodebind=N --membind=N` là cách thô để pin một process
khi thử nghiệm.

### Vì sao lựa chọn allocator quan trọng khi tải cao
Allocator mặc định của glibc có arena riêng cho từng thread, và có thể bị fragmentation nặng với các mẫu allocate
multi-threaded có tần suất cao (ví dụ buffer request/response được allocate rồi giải phóng ở mỗi connection). Các
proxy thường chuyển sang `jemalloc` hoặc `mimalloc` (crate `tikv-jemallocator`/`mimalloc` trong Rust) để có tail
latency dễ đoán hơn và fragmentation thấp hơn. Với [`proxy`](../../proxy) dưới throughput bền vững, đây là khác
biệt thật và đo được, không phải một micro-optimization.

## Practice
1. Đọc `/proc/self/status` (`VmRSS` so với `VmSize`) trong một chương trình Rust nhỏ, trước và sau khi
   `Vec::with_capacity` một vùng lớn, rồi trước và sau khi thực sự ghi vào nó.
2. Tái hiện gotcha về overcommit: allocate nhiều virtual memory hơn RAM vật lý, xác nhận nó "thành công", rồi ghi
   vào và xem RSS tăng vọt. Làm việc này trong một container/VM có giới hạn memory, không phải trên máy chính của
   bạn.
3. Dùng `/proc/self/smaps` hoặc `pmap` để kiểm tra memory map của một proxy đang chạy, và xác định vùng nào được
   page cache backing, vùng nào là anonymous.
4. Đổi allocator của [`proxy`](../../proxy) sang `mimalloc` qua `#[global_allocator]` và benchmark một workload nặng
   về allocate, trước và sau.
5. Nếu bạn có máy nhiều socket, chạy `numactl --hardware` và giải thích một truy cập bộ nhớ "remote" tốn gì so với
   truy cập local.
6. Quan sát fault và OOM killer. Chạy một chương trình scratch chạm vào từng page của 1 GiB và đọc `minflt` từ
   `/proc/<pid>/stat` (hoặc `perf stat -e minor-faults`). Sau đó chạy nó trong container với `--memory=256m` (và
   `--memory-swap=256m`), allocate cho đến khi bị giết. Xác nhận exit code 137, `dmesg -T | grep -i oom`, và bộ
   đếm `oom_kill` trong `memory.events` của cgroup.

---

<!-- nav -->
← Trước: [`02-linux/15-io_uring.md`](15-io_uring.md) · [Mục lục](00-README.md) · Tiếp theo: [`02-linux/17-signals.md`](17-signals.md) →
