# Page Cache

Cache của kernel cho dữ liệu file trong RAM. Thứ khiến việc serve static
file ([`05-http-stack/05-static.md`](../05-http-stack/05-static.md)) nhanh, và thứ khiến việc đo bộ nhớ của
proxy bạn trở nên khó hiểu.

## What to learn

### Mọi lần đọc file đều đi qua nó
`read()` trên một file không chạm tới disk nếu dữ liệu đã được cache:
kernel giữ nội dung file theo đơn vị kích thước trang (4 KB) trong page
cache, khóa theo (inode, offset). Một cú hit copy từ RAM; một cú miss kích
hoạt disk I/O, điền vào cache, rồi mới copy.

Hai hệ quả cho một proxy serve static file: request thứ hai cho một file
được phục vụ từ RAM bất kể ứng dụng bạn làm gì, và cache riêng của bạn ở
tầng userspace cho nội dung file có thể đang **trùng lặp** với page cache
— trả tiền hai lần cho bộ nhớ để có cùng một tốc độ. Hãy cache các artifact
đã parse hoặc nén, không phải raw byte của file mà kernel đã đang giữ sẵn.

### Vì sao `free -h` trông đáng báo động mà thực ra không phải vậy
Page cache hiện dưới dạng "buff/cache" và thường xuyên chiếm hết mọi RAM
lẽ ra đang rảnh. Đây là hành vi đúng — các trang cache được backing bởi
một file chưa sửa đổi có thể được thu hồi ngay lập tức, vì dữ liệu đã tồn
tại trên disk. Con số cần theo dõi là `available`, không phải `free`.

Gotcha: bên trong một container, chuyện này không còn vô hại nữa. Page
cache do một cgroup tạo ra tính vào giới hạn bộ nhớ của cgroup đó, nên một
proxy đang stream file lớn có thể bị OOM-kill vì bộ nhớ *có thể thu hồi
được* — kernel thường thu hồi trước khi kill, nhưng dưới áp lực bộ nhớ kết
hợp với một đợt dirty page dồn dập, nó không phải lúc nào cũng thắng cuộc
đua. Nếu proxy của bạn chết vì OOM mà RSS trông thấp hơn nhiều so với giới
hạn, đây là điều đầu tiên cần kiểm tra.

### Zero-copy phụ thuộc hoàn toàn vào nó
`sendfile()` và `splice()` (xem [`02-linux/11-zerocopy.md`](../02-linux/11-zerocopy.md)) chuyển dữ liệu
từ page cache tới một socket mà không copy qua user space. Điều đó chỉ
nhanh khi cache *hit* — khi miss, syscall block trên disk I/O, và trong
một async runtime, việc đó block toàn bộ worker thread (vấn đề
cooperative-scheduling ở [`03-rust/05-async.md`](../03-rust/05-async.md)), làm đình trệ mọi kết nối
khác trên đó.

Đây là cái bẫy của "cứ dùng sendfile cho static file": nó tuyệt vời cho
một working set nóng và là quả mìn latency cho một working set lạnh.
`tokio::fs` né được bằng cách đẩy sang một thread pool blocking, tốn một
lần copy nhưng giữ reactor phản hồi được.

Gotcha: `mmap` có cùng hình dạng và còn tệ hơn trong ngữ cảnh async — một
page fault trên một mapping lạnh block mà không có ranh giới syscall nào
để runtime quan sát, nên nó vô hình với mọi công cụ chẩn đoán của tokio.

### Readahead
Kernel phát hiện truy cập tuần tự và prefetch trước người đọc, đó là lý do
việc stream một file lớn tuần tự nhanh hơn nhiều so với đọc cùng số byte
đó một cách ngẫu nhiên. `posix_fadvise` cho phép bạn khai báo ý định tường
minh: `SEQUENTIAL` để tăng readahead, `RANDOM` để tắt nó, `WILLNEED` để
prefetch trước khi bạn cần, `DONTNEED` để evict.

Với một proxy, `WILLNEED` trên một file sắp stream có thể biến latency
chunk đầu tiên từ một lần seek disk thành một cú cache hit. `DONTNEED` sau
khi stream một file one-shot rất lớn ngăn nó đẩy văng working set thực sự
nóng của bạn — phiên bản page cache của chính vấn đề scan-pollution ở
[`13-algorithms/lru.md`](../13-algorithms/lru.md).

### Eviction là CLOCK, và dirty page thì khác
Trang sạch bị loại bỏ khi thu hồi, gần như miễn phí. Trang bẩn (đã ghi
nhưng chưa được persist) phải được ghi ngược lại disk trước, nên thu hồi
chúng có thể block. Writeback được điều khiển bởi `vm.dirty_ratio` và
`vm.dirty_background_ratio`; vượt qua tỷ lệ cứng khiến *người ghi* phải
block đồng bộ cho tới khi writeback bắt kịp.

Một proxy ghi access log ra disk ([`08-observability/01-logging.md`](../08-observability/01-logging.md)) là
một nguồn sinh dirty page. Dưới việc log nặng trên storage chậm, một lần
ghi log có thể block một thread đang xử lý request — chính xác là lý do
file đó khuyến nghị `tracing_appender::non_blocking`.

Chính sách thu hồi là một biến thể LRU xấp xỉ: hai list (active và
inactive) với reference bit, thuộc họ thuật toán CLOCK được mô tả ở
[`13-algorithms/lru.md`](../13-algorithms/lru.md). Cùng lý do: việc "phẫu thuật" list per-access của
LRU thật sự là không kham nổi ở quy mô page cache.

### Đo lường
`/proc/meminfo` cho biết `Cached`, `Dirty`, và `Writeback` trên toàn hệ
thống. Với một file cụ thể, `mincore()` báo trang nào đang resident (công
cụ `vmtouch` bọc lại lời gọi này). Để test hành vi cold-cache một cách
trung thực, drop cache giữa các lần chạy bằng
`echo 3 > /proc/sys/vm/drop_caches` — nếu không, benchmark static-file của
bạn đang đo RAM và không nói lên điều gì về production, nơi working set
lớn hơn bộ nhớ.

## Practice
1. Đọc một file lớn hai lần, đo thời gian cả hai. Drop cache, lặp lại, và
   xác nhận timing của lần đọc đầu tiên quay trở lại.
2. Dùng `vmtouch` (hoặc `mincore` trực tiếp) để xem trang nào của một file
   đang resident sau khi [`labs/04-static-server`](../../labs/04-static-server) đã serve nó một lần.
3. Benchmark [`labs/04-static-server`](../../labs/04-static-server) với một working set vừa trong RAM,
   rồi một working set lớn hơn nhiều lần. So sánh p99 latency và giải
   thích khoảng cách đó dựa trên những gì bạn biết về cache hit và
   blocking.
4. Chứng minh nguy cơ blocking: serve một file lớn lạnh bằng một đường
   kiểu `sendfile` trên một runtime một-worker và đo latency của các
   request đồng thời trong lúc đọc disk.
5. Thêm `posix_fadvise(WILLNEED)` trước khi stream và đo thay đổi trong
   latency byte-đầu-tiên trên một file lạnh.
6. Chạy proxy trong một container có giới hạn bộ nhớ, stream các file có
   tổng dung lượng vượt giới hạn, và theo dõi `memory.current` của cgroup
   cùng cách nó tính page cache.
