# RPS (Receive Packet Steering)

`16-kernel/05-rss.md` nói về việc dàn xử lý packet ra nhiều core bằng
hardware. RPS là phiên bản tương đương bằng phần mềm của kernel, dành cho
các NIC không cho đủ hardware queue để làm điều đó — phổ biến trên cloud
VM với interface virtio-net một queue.

## What to learn

### Cùng mục tiêu, chuyển sang phần mềm
Một khi packet đến bất kể queue/core nào mà NIC (hay queue đơn của nó)
đưa nó tới, RPS tính một hash trên header packet — về khái niệm giống hệt
hash 5-tuple mà RSS dùng — và nếu nó chọn ra một core *khác* với core đang
xử lý packet hiện tại, gửi một inter-processor interrupt (IPI) để điều
hướng phần còn lại của việc xử lý packet đó tới đó. Nó giải quyết cùng bài
toán "dàn tải ra nhiều core" như RSS, nhưng làm sau và bằng phần mềm, tốn
một IPI mà RSS phần cứng tránh được hoàn toàn.

### RFS: điều hướng về phía thread ứng dụng đang tiêu thụ dữ liệu
Việc điều hướng dựa trên hash của RPS không biết và không quan tâm core
nào mà thread *ứng dụng* đang đọc socket thực sự chạy — nó có thể điều
hướng việc xử lý packet tới một core, rồi sau đó thread ứng dụng (trên một
core khác) vẫn lấy dữ liệu, mất đi lợi ích về cache-locality mà việc điều
hướng vốn nhắm tới. **Receive Flow Steering (RFS)** cải tiến điều này bằng
cách theo dõi, theo từng flow, CPU nào gọi `recvmsg` cho nó lần cuối, và
điều hướng các packet tương lai của flow đó tới đó thay vì chỉ dựa vào
hash — khớp tốt hơn với nơi dữ liệu thực sự sẽ được tiêu thụ, đổi lại phải
duy trì một flow table.

### Cấu hình
```
# bitmap CPU đủ điều kiện điều hướng RPS cho mỗi receive queue
echo f > /sys/class/net/eth0/queues/rx-0/rps_cpus   # CPU 0-3

# RFS: kích thước flow table toàn cục và theo từng queue
echo 32768 > /proc/sys/net/core/rps_sock_flow_entries
echo 2048 > /sys/class/net/eth0/queues/rx-0/rps_flow_cnt
```

### Khi nào nó thực sự quan trọng với một proxy
Kiểm tra `ethtool -l` trước (theo bài tập của `16-kernel/05-rss.md`): nếu
NIC đã lộ ra nhiều hardware queue với RSS được dàn đúng ra các core, RPS
chỉ thêm overhead IPI mà không có lợi ích gì — nó là phương án dự phòng
cho khi điều hướng bằng hardware không khả dụng hoặc không đủ (một NIC
virtio-net một queue là trường hợp thực tế phổ biến trong môi trường
cloud), không phải một nâng cấp tuyệt đối so với RSS.

### Gotcha: chi phí IPI có thể trở thành nút thắt của chính nó
Ở tốc độ packet rất cao trên một máy nhiều core, các inter-processor
interrupt mà RPS phát ra để điều hướng packet bản thân chúng không miễn
phí — đủ nhiều trong số đó có thể tiêu tốn một phần đáng kể của một core,
đặc biệt nếu các target điều hướng bị dàn mỏng ra nhiều core, mỗi core chỉ
xử lý một lượng traffic nhỏ. Hãy đo xem bật RPS có thực sự cải thiện
throughput/latency cho traffic pattern của bạn không, thay vì giả định
"dàn ra nhiều core hơn" luôn tốt hơn vô điều kiện; đôi khi nó chỉ chuyển
nút thắt từ "một core làm mọi thứ" sang "mọi core đều tốn cycle cho IPI".

## Practice
1. Xác nhận qua `ethtool -l` rằng NIC của hệ thống test bạn chỉ lộ ra một
   (hoặc rất ít) hardware queue — đúng kịch bản RPS được sinh ra để giải
   quyết.
2. Bật RPS bằng cách set `rps_cpus` cho receive queue, và RFS qua các
   sysctl flow-table ở trên.
3. Tạo tải liên tục vào `proxy` và so sánh phân bố CPU theo từng core cùng
   throughput/latency khi RPS bật so với tắt.
4. Nếu bạn có quyền truy cập một NIC multi-queue từ bài tập
   `16-kernel/05-rss.md`, so sánh phân bố core mà RPS đạt được và overhead
   của nó với kết quả dựa-trên-hardware của RSS trên cùng một workload.
