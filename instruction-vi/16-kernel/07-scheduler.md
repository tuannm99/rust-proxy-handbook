# CPU Scheduling

Vì sao p99 latency của một proxy đôi khi chẳng liên quan gì tới code của
chính nó: OS scheduler quyết định thread nào chạy trên core nào và khi
nào, và một host bận rộn có thể thêm latency mà profiler của bạn sẽ không
bao giờ chỉ ra.

## What to learn

### CFS: công bằng theo virtual runtime, không phải FIFO
Completely Fair Scheduler của Linux giữ các task có thể chạy trong một
red-black tree, sắp theo **virtual runtime** (`vruntime`) — thời gian CPU
tích lũy, có trọng số theo priority. Scheduler luôn chọn task có
`vruntime` thấp nhất để chạy tiếp, nên một task ít dùng CPU gần đây sẽ
được ưu tiên; việc chạy làm tăng `vruntime` của nó, cuối cùng khiến nó
không còn là nhỏ nhất nữa, và thứ khác được chạy. Đây là lập lịch "công
bằng", không phải "sẵn sàng trước, chạy trước" — không task nào được đảm
bảo chạy trong một khoảng thời gian cụ thể, chỉ đảm bảo thời gian CPU được
phân phối tỷ lệ theo trọng số theo thời gian.

### `nice`/priority điều khiển trọng số, không phải một đảm bảo cứng
Giá trị `nice` ánh xạ tới trọng số quyết định `vruntime` của một task tăng
nhanh thế nào (một task priority cao hơn có `vruntime` tăng chậm hơn trên
mỗi đơn vị thời gian CPU thực, nên nó "được nợ" CPU lâu hơn). Điều này
thay đổi *bao nhiêu* CPU một task nhận được so với các task khác, không
phải một đảm bảo về latency — một process `proxy` chạy priority cao trên
một máy rảnh rỗi hành xử giống hệt như chạy priority bình thường; sự khác
biệt chỉ hiện ra khi có thứ khác thực sự đang tranh giành CPU.

### CPU affinity: chống lại chi phí migration, không chỉ là fairness
Một task di chuyển giữa các core mất trạng thái cache L1/L2 còn nóng trên
core cũ và bắt đầu lạnh trên core mới (`17-performance/01-cpu-cache.md`,
`17-performance/03-numa.md`). Pin worker thread của một proxy vào các core
cụ thể (`taskset`, hoặc `sched_setaffinity` từ trong chương trình) đánh
đổi sự tự do của CFS trong việc cân bằng tải qua mọi core lấy việc thực
thi nhất quán, cache-nóng trên một tập cố định — một lợi ích tail-latency
thực sự, đổi lại mất khả năng tự động cân bằng tải nếu tập được pin trở
nên tải không đều.

### Gotcha: scheduler của tokio là một tầng thứ hai, tách biệt, nằm trên
tầng này
Work-stealing scheduler của tokio quyết định *task* nào (một async green
thread) chạy trên *tokio worker thread* nào mà nó quản lý — đó hoàn toàn
là sổ sách kế toán ở tầng userspace. OS scheduler, tách biệt, quyết định
OS thread thực sự nào của process bạn được cấp một CPU core, và khi nào.
Một task mà tokio coi là "sẵn sàng để poll" có thể ngồi không chạy một
lúc, không phải vì tokio làm gì sai, mà vì OS thread đáng lẽ sẽ poll nó
hiện không được lập lịch trên bất kỳ core nào — một host bị oversubscribe
(nhiều thread có thể chạy trên toàn bộ process hơn số core) gây ra chính
xác điều này, và nó thể hiện ra như latency mà chính metric của tokio sẽ
không giải thích được. Kiểm tra độ dài run-queue của `vmstat`/`mpstat`
(cột `r`) cùng với metric ở tầng tokio trước khi kết luận một đợt tăng
latency là vấn đề của tokio hay của ứng dụng.

## Practice
1. Chạy `proxy` (hoặc một crate trong `labs/`) dưới
   `12-testing/01-load-testing.md` trên một host bạn cố tình oversubscribe
   (khởi chạy đủ process nền CPU-bound để vượt số core) và quan sát tác
   động lên p99 latency so với một host không tải.
2. Kiểm tra cột run-queue của `vmstat 1` trong bài test đó và xác nhận nó
   tương quan với việc tăng latency, phân biệt điều này với một vấn đề lập
   lịch ở tầng tokio.
3. Pin worker thread của proxy vào một tập con các core bằng `taskset`
   (để các process oversubscribe kia trên phần còn lại) và đo lại p99
   latency.
4. So sánh việc tăng priority bằng `nice` với việc pin core cho cùng kịch
   bản oversubscribed, và viết ra cái nào thực sự giải quyết vấn đề
   tail-latency và vì sao.
