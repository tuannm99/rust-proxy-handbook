# RSS (Receive Side Scaling)

[`16-kernel/04-interrupt.md`](04-interrupt.md) nói về việc vì sao một core xử lý mọi
interrupt packet trở thành trần throughput. RSS là cách fix ở tầng
hardware: chính NIC dàn packet đến — và interrupt của chúng — ra nhiều
core trước khi kernel kịp nhìn thấy chúng.

## What to learn

### Hashing bằng hardware vào nhiều queue
Một NIC multi-queue hash header của mỗi packet đến (thường là 5-tuple:
IP/port nguồn/đích, protocol) bằng một hàm hash cài trong hardware
(Toeplitz là loại phổ biến) để chọn một trong nhiều receive queue. Mỗi
queue có interrupt riêng, và mỗi interrupt có thể được điều hướng (qua
`/proc/irq/<n>/smp_affinity` hoặc `irqbalance`) tới một core cụ thể. Kết
quả: các packet thuộc cùng một flow (cùng 5-tuple, tức cùng một kết nối
TCP) luôn rơi vào cùng một queue và cùng một core, trong khi các kết nối
khác nhau dàn ra nhiều queue tùy theo NIC và driver hỗ trợ bao nhiêu.

### Vì sao chính điều này cho phép bạn dùng nhiều core hơn cho networking
Không có RSS (NIC một queue, hoặc RSS bị tắt), interrupt của mọi packet
đều rơi vào một core — thường là core 0 mặc định — bất kể ứng dụng của
bạn spawn worker thread trên bao nhiêu core. RSS chính là thứ thực sự biến
"xử lý packet" thành một workload song song, đa-core ở tầng hardware; nếu
không có nó, trần của [`16-kernel/04-interrupt.md`](04-interrupt.md) áp dụng bất kể ứng dụng
được kiến trúc thế nào.

### Kiểm tra và tinh chỉnh
```
ethtool -l eth0   # xem số queue hiện tại/tối đa
ethtool -L eth0 combined 8   # yêu cầu 8 combined queue
ethtool -x eth0   # xem indirection table hiện tại (ánh xạ hash -> queue)
```
Indirection table ánh xạ các bucket hash tới queue; trên hầu hết NIC bạn
không tự tay chỉnh bảng đó, chỉ chỉnh số queue và IRQ affinity của từng
queue.

### Gotcha: tập trung kết nối phía sau một proxy/NAT khác
Hash 5-tuple giả định có sự đa dạng về IP/port nguồn để dàn tải đều. Một
proxy nằm sau một load balancer khác đang làm source NAT, hoặc phục vụ một
workload mà phần lớn traffic đến từ một số ít IP upstream/client, có thể
thấy hash tập trung vào một vài queue dù RSS đã được cấu hình đúng — hàm
hash không có gì để làm việc khi không gian input hẹp. Kiểm tra số packet
theo từng queue (`ethtool -S eth0 | grep rx_queue`) thay vì giả định RSS
được cấu hình đồng nghĩa với cân bằng *đạt được* đồng đều.

## Practice
1. Kiểm tra số queue và indirection table của hệ thống test bằng
   `ethtool -l` / `ethtool -x` (NIC virtio-net của một cloud VM có thể chỉ
   báo một queue — ghi chú lại nếu vậy, vì nó thay đổi những gì thực sự đạt
   được ở đây).
2. Nếu có nhiều queue, set IRQ affinity từng queue để dàn ra nhiều core và
   xác nhận qua `/proc/interrupts` rằng interrupt của các queue khác nhau
   rơi vào các core khác nhau.
3. Tạo tải vào [`proxy`](../../proxy) từ nhiều source port/kết nối khác nhau và so sánh
   số packet theo từng queue (`ethtool -S`) với một bài test tái sử dụng
   rất ít kết nối nguồn — quan sát hiệu ứng tập trung mà gotcha ở trên mô
   tả.
4. Đối chiếu CPU usage theo từng core (`mpstat -P ALL`) với số packet theo
   từng queue trong một bài load test để xác nhận ánh xạ queue-tới-core
   của RSS thực sự phản ánh vào việc scale throughput ở tầng ứng dụng.
