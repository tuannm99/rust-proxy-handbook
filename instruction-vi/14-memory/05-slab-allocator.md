# Slab Allocator

Allocate theo size-class cố định: câu trả lời ở mức allocator cho
fragmentation. [`13-algorithms/slab.md`](../13-algorithms/slab.md) nói về cấu trúc dữ liệu Rust bạn tự
viết; file này nói về chiến lược allocate đứng sau nó.

## What to learn

### Ý tưởng
Một allocator tổng quát phải phục vụ mọi kích thước, đó là điều khiến nó
dễ bị fragmentation ([`14-memory/06-fragmentation.md`](06-fragmentation.md)). Một slab allocator
từ bỏ tính tổng quát: nó chỉ phục vụ đúng một kích thước object, từ các
vùng đã được cắt sẵn ("slab") chia thành các slot bằng nhau.

Sự giới hạn đó mang lại ba thứ cùng lúc:
- **Không có external fragmentation, theo cấu trúc.** Mọi slot trống đều
  vừa với mọi request, vì mọi request đều cùng kích thước.
- **Alloc và free O(1).** Pop hoặc push vào một free list — không cần tìm
  kích thước, không coalescing, không splitting.
- **Cache locality.** Các object cùng loại nằm liền kề nhau, nên duyệt qua
  các connection đang hoạt động chạm vào các cache line liên tiếp (khía
  cạnh CPU cache của việc này được nói ở [`17-performance/01-cpu-cache.md`](../17-performance/01-cpu-cache.md)).

Ý tưởng này bắt nguồn từ kernel Solaris và là cách Linux allocate các
object kích thước cố định của chính nó (`task_struct`, inode, socket
buffer) — cùng lý do đó áp dụng cho một proxy allocate một connection
struct mỗi connection.

### Cấu trúc: slab, free list, và cache
Một allocator cho một size class giữ một tập slab, mỗi slab thường là một
hoặc vài page, được cắt thành N slot. Slab được theo dõi theo trạng thái
— full, partial, empty — và việc allocate ưu tiên một slab *partial*, để
các slab đang dùng dở được lấp đầy thay vì mọi slab đều nửa vơi nửa đầy.
Chỉ những slab empty mới có thể được trả lại cho OS.

Giống trong [`13-algorithms/slab.md`](../13-algorithms/slab.md), free list được xâu chuỗi xuyên qua
chính các slot trống, nên không tốn thêm bộ nhớ.

Gotcha: allocate từ slab partial trước, và thứ tự của free list quan trọng
hơn vẻ ngoài của nó. LIFO (tái sử dụng slot vừa mới free gần nhất) giữ
working set nóng trong cache; FIFO xoay vòng qua mọi slot và liên tục làm
evict. Đây là một khác biệt một dòng code nhưng có tác động đo được lên
throughput.

### Per-CPU cache
Free list là shared mutable state, nên một slab allocator ngây thơ
serialize hóa mọi allocation trên một lock duy nhất — không chấp nhận
được trên một proxy multi-threaded nơi allocation nằm trên hot path.

Các implementation thật giữ một "magazine" nhỏ per-CPU (hoặc per-thread)
gồm các object trống, được refill từ các slab dùng chung theo lô. Trường
hợp phổ biến chỉ chạm vào thread-local state, không có atomic nào cả; lock
dùng chung chỉ được lấy một lần mỗi lô thay vì mỗi lần allocation. Đây là
mẹo cốt lõi trong cả tcache của jemalloc lẫn mimalloc.

Gotcha: caching theo từng thread tạo ra sự mất cân bằng giữa các thread.
Một proxy nơi thread A accept connection còn thread B đóng chúng sẽ tích
lũy object trống trong cache của B trong khi A cạn kiệt và liên tục refill
lại từ pool dùng chung. Các allocator xử lý việc này bằng cách flush cache
định kỳ và các remote-free queue; nếu bạn tự viết một pool, sự bất đối
xứng này chính là bug bạn sẽ gặp, và nó xuất hiện dưới dạng bộ nhớ tăng
đều đặn chứ không phải một crash.

### Chỗ này thực sự thuộc về đâu trong một proxy
Gần như chắc chắn bạn không nên tự viết một slab allocator toàn cục.
jemalloc và mimalloc đã implement allocate theo size-class với per-CPU
cache, và chỉ cần đổi global allocator ([`02-linux/16-memory.md`](../02-linux/16-memory.md)) là bạn đã
có phần lớn lợi ích chỉ với một dòng code.

Thứ đáng để tự viết là một **typed pool** cho số ít object được allocate
một lần mỗi connection hoặc mỗi request — connection state và buffer I/O
(buffer pooling chuyên dụng được lên kế hoạch trong [`14-memory/00-README.md`](00-README.md)).
Đó là những object có kích thước đã biết, churn cao, và đủ sống lâu để
việc pooling loại bỏ hoàn toàn allocator khỏi hot path — đây là một chiến
thắng khác và lớn hơn so với việc chỉ làm cho allocation rẻ hơn.

Gotcha: một object đã pool phải được reset hoàn toàn khi release. Một
connection struct được trả về pool mà vẫn còn giữ header hay địa chỉ peer
của request trước là một bug rò rỉ dữ liệu mà type system sẽ không bắt
được, vì type vẫn hợp lệ — chỉ có nội dung là cũ. Reset khi release, không
phải khi acquire, để một object cũ bị rò rỉ không bao giờ được phát ra
ngay cả khi đường acquire sau này bị refactor.

### Giới hạn pool
Một pool không giới hạn biến một đợt tăng traffic thành một mức đỉnh bộ
nhớ vĩnh viễn. Giới hạn kích thước pool và để các allocation vượt quá giới
hạn rơi xuống global allocator (giảm hiệu năng, không giảm tính đúng đắn),
và export kích thước cùng hit rate của pool dưới dạng metric
([`08-observability/02-metrics.md`](../08-observability/02-metrics.md)) để giới hạn được tune dựa trên dữ liệu.

## Practice
1. Implement một slab allocator một size-class: cắt một page thành các
   slot, xâu chuỗi một free list qua các slot trống, và implement
   alloc/free. Xác nhận bằng một assertion rằng không có hai allocation
   đang sống nào chồng lên nhau.
2. Theo dõi slab full/partial/empty và xác nhận allocation ưu tiên slab
   partial; dựng một workload mà nếu không có quy tắc đó sẽ khiến mọi slab
   nửa vơi nửa đầy.
3. So sánh thứ tự free list LIFO vs FIFO dưới một benchmark nặng
   alloc/free và đo khác biệt về cache miss (`perf stat -e cache-misses`).
4. Thêm một magazine per-thread và benchmark so với phiên bản một lock
   duy nhất ở 1, 4, và 8 thread.
5. Tái tạo sự mất cân bằng giữa các thread: allocate trên một thread và
   free trên thread khác trong một vòng lặp, quan sát tổng bộ nhớ tăng
   lên. Sau đó thêm một đường flush hoặc remote-free và xác nhận nó ổn
   định lại.
6. Xây một buffer pool có giới hạn, có type, cho các buffer đọc theo từng
   connection của [`proxy`](../../proxy), với reset-on-release, và export metric
   size/hit-rate. So sánh số lượng allocation dưới tải có và không có nó.
