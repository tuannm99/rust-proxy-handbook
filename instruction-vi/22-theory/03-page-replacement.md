# Page Replacement Algorithm

## What to learn

### Bài toán: nhiều virtual page hơn physical frame
Khi physical memory đầy và một process cần một page mới, OS phải evict
cái gì đó — page replacement là chính sách chọn cái gì. Đây là bức tranh
virtual memory của [`02-linux/16-memory.md`](../02-linux/16-memory.md) (xây trên primer của
[`02-linux/08-memory-basics.md`](../02-linux/08-memory-basics.md)) với phần còn thiếu được lấp đầy: chuyện gì
thực sự xảy ra khi page fault mà không còn frame rỗng nào.

### FIFO, và vì sao nó tệ hơn trực giác gợi ý
Evict page load lâu nhất. Đơn giản, nhưng gặp nghịch lý Bélády: tăng số
physical frame có thể, ngược trực giác, *tăng* số page fault với một số
access pattern — một kết quả đủ bất ngờ để trở thành ví dụ chuẩn cho
"trực giác của bạn về caching policy không phải là chứng minh."

### LRU: chuẩn thực dụng, và chi phí chính xác của nó
Evict page ít dùng gần đây nhất — cùng chính sách với cache eviction của
[`13-algorithms/lru.md`](../13-algorithms/lru.md), áp dụng cho physical page frame thay vì một cache
tầng ứng dụng. LRU thật cần một cập nhật timestamp (hoặc vị trí) trên mỗi
lần truy cập memory, quá đắt để implement chính xác ở quy mô liên quan
tới page-fault trong hardware hoặc OS — hệ thống thật xấp xỉ nó.

### Clock (second-chance): xấp xỉ thực dụng của LRU
Các page nằm trong một danh sách vòng, mỗi page có một bit "referenced"
duy nhất, được hardware set khi truy cập. Con trỏ eviction quét vòng
tròn: nếu bit của một page đã set, clear nó về 0 và đi tiếp (cho nó một
"cơ hội thứ hai"); nếu bit đã clear, evict nó. Cái này xấp xỉ thứ tự
recency của LRU dùng một bit mỗi page thay vì một timestamp — cơ chế thật
bên trong page reclaim của Linux, qua một biến thể active/inactive list
tinh chỉnh hơn.

```text
pages: [A(1), B(0), C(1), D(0)]   (bit = referenced)
quét tới A: bit=1 -> clear về 0, bỏ qua
quét tới B: bit=0 -> evict B
```

### Optimal (giải thuật Bélády's) như một benchmark không thể chạm tới
Evict page sẽ không được dùng lâu nhất trong tương lai — chứng minh được
là tối thiểu hóa page fault, và chứng minh được là không thể implement
online, vì nó cần biết access pattern tương lai. Giá trị của nó là một
upper bound lý thuyết: giải thuật thật được đánh giá bằng việc chúng gần
optimal đến đâu trên một trace cho trước, không phải theo một chuẩn tuyệt
đối nào.

### Thrashing và mô hình working-set
Khi tổng working set đang hoạt động của các process vượt physical memory,
hệ thống tốn nhiều thời gian phục vụ page fault hơn làm việc có ích —
throughput sụp đổ dù CPU utilization trông có vẻ bận (nó bận paging,
không phải tính toán). Mô hình working-set hình thức hóa "working set
đang hoạt động" là các page được tham chiếu trong Δ đơn vị thời gian gần
nhất, và là nền tảng lý thuyết cho lời khuyên thực dụng của
[`02-linux/16-memory.md`](../02-linux/16-memory.md): biết footprint memory thật của một process trước
khi đặt một giới hạn memory cgroup, vì một giới hạn dưới working set
không làm process chậm lại nhẹ nhàng — nó làm process đó thrash.

## Practice
1. Implement mô phỏng page replacement FIFO và LRU trên một access trace
   cố định (một danh sách số page) với một số frame cố định nhỏ, và đếm
   page fault cho mỗi cái.
2. Tái tạo nghịch lý Bélády: tìm hoặc dựng một access trace nơi số fault
   của FIFO *tăng* khi bạn thêm một frame, và xác nhận LRU không thể hiện
   nghịch lý tương tự trên trace đó.
3. Implement clock/second-chance trên cùng trace và so sánh số fault của
   nó với LRU thật — xác nhận nó gần nhưng không giống hoàn toàn.
4. Implement giải thuật optimal (offline, "ăn gian") trên cùng trace bằng
   cách biết trước toàn bộ access tương lai, và dùng nó làm baseline để
   chấm FIFO/LRU/clock gần nó đến đâu.
5. Nối lại với [`02-linux/16-memory.md`](../02-linux/16-memory.md): chạy một process có working set
   lớn hơn một giới hạn memory cgroup bạn đặt, và quan sát thrashing (qua
   cột `si`/`so` của `vmstat` hoặc counter page fault của `/proc/vmstat`)
   thay vì một sự chậm lại nhẹ nhàng.
