# NUMA

Non-uniform memory access — trên một máy nhiều socket, bộ nhớ gắn với
socket khác chậm hơn để truy cập tới, và bỏ qua điều đó có thể làm giảm
một nửa throughput. Chỉ liên quan trên phần cứng nhiều socket; bỏ qua trên
một máy một socket.

## What to learn

### NUMA là gì
Trên một server nhiều socket, mỗi CPU socket có bộ điều khiển bộ nhớ riêng
và một dải RAM riêng (một "NUMA node"). Một core có thể đọc bộ nhớ của bất
kỳ node nào, nhưng đến một node *ở xa* phải đi qua đường kết nối liên
socket (UPI/QPI) — thường có latency gấp 1.5–2× và băng thông thấp hơn so
với bộ nhớ local. Trên một máy một socket chỉ có một node và không điều
gì trong này áp dụng, đây là phần lớn các deployment; điều này quan trọng
trên các máy 2 và 4 socket lớn mà một proxy lưu lượng cao thỉnh thoảng
chạy trên đó.

### Chính sách cấp phát mặc định và cái bẫy của nó
Linux dùng first-touch: một page được đặt lên node của core đầu tiên
*ghi* vào nó, không phải core đã cấp phát nó. Nên nếu một thread khởi
động khởi tạo một buffer pool lớn rồi các worker thread trên socket khác
sau đó dùng nó, mọi truy cập đều là remote. Cái pool "thuộc về" sai node
trong suốt vòng đời của nó. Đây là lỗi NUMA phổ biến nhất và nó vô hình
trong code — việc cấp phát trông hoàn toàn bình thường.

### Cách sửa: pin, và chạm cục bộ
Chiến lược là giữ bộ nhớ của mỗi worker trên node của chính worker đó:

- Pin worker thread vào các core cụ thể (`sched_setaffinity`, hoặc crate
  `core_affinity`) để một worker ở nguyên trên một socket.
- Để mỗi worker tự cấp phát và first-touch buffer của *chính nó*, để
  first-touch đặt chúng cục bộ, thay vì chia sẻ một pool toàn cục được
  khởi tạo ở nơi khác.
- Điều này biến toàn bộ proxy thành một thiết kế shared-nothing, theo
  từng core — đây chính là những gì pingora và các data plane kiểu DPDK
  làm, và nó kết hợp tốt với các counter theo core trong
  `17-performance/02-false-sharing.md` và cache theo core trong
  `13-algorithms/lru.md`.

```text
chạy theo từng socket:  ./proxy  →  numactl --cpunodebind=0 --membind=0 ./proxy (inst A)
                                    numactl --cpunodebind=1 --membind=1 ./proxy (inst B)
```

Gotcha: câu trả lời production thô nhưng hiệu quả thường không phải là
làm một process nhận biết NUMA, mà là chạy *một instance proxy mỗi
socket*, mỗi cái pin bằng `numactl`, phía sau một load balancer. Hai
instance shared-nothing né tránh mọi câu hỏi cross-node mà một process
đơn lẻ phải giải quyết cẩn thận.

### Interrupt và NIC cũng là một phần của bức tranh
Interrupt của một NIC rơi vào một node nào đó; nếu packet được DMA vào bộ
nhớ của node 0 nhưng được xử lý bởi một worker trên node 1, bạn trả giá
remote cho mỗi packet trước cả khi code của bạn chạy. Căn chỉnh NIC IRQ
affinity (và RSS/RPS, `16-kernel/`) khớp với các worker xử lý những packet
đó là nửa còn lại của việc tune NUMA, và thường quan trọng hơn cả việc
heap của bạn nằm ở đâu.

### Đo lường nó
`numastat` cho thấy việc cấp phát theo từng node và, quan trọng là, số
đếm `numa_miss` / `numa_foreign` — các truy cập remote đáng lẽ muốn là
local. `perf` có thể gán các stall do remote-memory. Như phần còn lại của
`17-performance/`, đừng tune một cách suy đoán: xác nhận bạn thực sự bị
giới hạn bởi NUMA (throughput scale kém qua các socket, số đếm truy cập
remote cao) trước khi pin bất cứ thứ gì, vì trên một máy một socket tất cả
việc này đều lãng phí công sức.

## Practice
1. Kiểm tra xem điều này có áp dụng không: `numactl --hardware` và
   `lscpu` để xem số node. Trên một máy dev một socket, ghi nhận rằng thư
   mục này là no-op ở đó và tiếp tục.
2. Trên một máy nhiều socket (hoặc một cloud instance phơi bày NUMA), tái
   tạo cái bẫy first-touch: khởi tạo một buffer lớn trong một thread, dùng
   nó từ các thread pin vào node khác, và đọc `numa_foreign` của
   `numastat`.
3. Sửa nó bằng cách để mỗi worker đã pin first-touch buffer của chính nó;
   đo lại `numa_miss`/`numa_foreign` và throughput.
4. So sánh hai hình dạng deployment cho `proxy`: một process nhận biết
   NUMA so với hai instance pin bằng `numactl` phía sau một balancer, dưới
   tải của `12-testing/01-load-testing.md`.
5. Căn chỉnh NIC IRQ affinity khớp với các socket của worker và đo xem chi
   phí remote-per-packet có giảm không — kết nối điều này với RSS/RPS
   trong `16-kernel/`.
