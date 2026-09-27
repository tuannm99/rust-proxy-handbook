# Priority Queue ở Quy mô Lớn: Timer Wheel

`13-algorithms/heap.md` nói về binary heap backing một priority queue nhỏ
(một entry cho mỗi upstream). File này nói về chuyện gì xảy ra khi công
việc của priority queue là lên lịch cho các *timeout* — có thể một cho mỗi
kết nối, ở quy mô của một proxy — nơi chi phí O(log n) mỗi thao tác và
việc hủy khó xử của một heap không còn "đủ tốt" nữa.

## What to learn

### Vì sao một heap chật vật khi làm kho lưu timeout
Một proxy lên lịch một timeout cho gần như mọi kết nối và request (idle
timeout, read timeout, retry deadline) và hủy phần lớn chúng sớm khi thao
tác hoàn thành bình thường. Một heap xử lý điều này như
insert-rồi-thường-hủy-trước-khi-nổ, và việc hủy một entry bất kỳ trong một
binary heap (không chỉ root) cần cùng cơ chế decrease-key mà `heap.md` mô
tả cho việc cập nhật — một lần tìm-và-xóa O(log n) cho mỗi lần hủy, với
tần suất "một lần mỗi request," là chi phí thật ở số lượng kết nối cao.

### Timer wheel: gộp theo thời điểm, không theo thứ tự chính xác
Một timer wheel đánh đổi thứ tự chính xác để lấy insert O(1) và hủy O(1)
bằng cách gộp các deadline vào một số lượng cố định các khe thời gian
(một mảng vòng, giống cấu trúc của `13-algorithms/ring-buffer.md`, nhưng
được index theo thời gian tương lai thay vì theo thứ tự insert):

```rust
struct TimerWheel {
    slots: Vec<Vec<TimerId>>, // slots[i] = các timer nổ ở tick i
    current_slot: usize,
    tick_duration: std::time::Duration,
}
// schedule(deadline): tính slot = (current_slot + ticks_until(deadline)) % slots.len()
//                     push timer id vào slots[slot]
// cancel(id): gỡ id khỏi bất kỳ slot nào đang giữ nó — O(1) với một index
//             phụ từ id -> slot
// ở mỗi tick: tiến current_slot, kích hoạt (và rút hết) mọi thứ trong đó
```

Một wheel đơn chỉ bao phủ các deadline trong khoảng
`slots.len() * tick_duration`; các deadline dài hơn cần một **hierarchical
timer wheel** (nhiều wheel ở độ chi tiết tăng dần — giây, rồi phút, rồi
giờ — nơi một timer được insert lại vào một wheel chi tiết hơn khi
deadline của nó tới gần). Đây chính xác là điều mà implementation timer
của chính kernel Linux và `HashedWheelTimer` của Netty làm, và đây là
thiết kế mà `tokio::time` dùng bên trong cho driver đứng sau mỗi lệnh gọi
`tokio::time::sleep` và `timeout`.

### Đánh đổi giữa độ chính xác và chi phí
Một timer wheel không nổ đúng tại deadline chính xác — nó nổ tại ranh giới
tick mà deadline rơi vào, nên độ chính xác bị giới hạn bởi `tick_duration`.
Với timeout kết nối/request (đo bằng chục mili giây tới vài giây) một tick
10-50ms là không cảm nhận được; với bất cứ thứ gì cần độ chính xác dưới
mili giây, thứ tự chính xác của một heap đáng giá chi phí mỗi thao tác cao
hơn của nó. Phần lớn các use case timeout của proxy nằm chắc chắn trong
nhóm "một wheel là đủ" — kiểm tra xem bạn thực sự đang đo cái gì trước khi
cho rằng bạn cần độ chính xác của heap.

### Gotcha: đừng tự xây cái của riêng bạn nếu runtime đã có sẵn một cái
`tokio::time::sleep`/`timeout`/`interval` đã được backing chính xác bởi cơ
chế này bên trong runtime của tokio rồi. Xây một timer wheel thứ hai,
riêng biệt trong `proxy/` cho thứ mà `tokio::time` đã bao phủ chỉ nhân đôi
một hệ thống con đã được test kỹ mà không có lợi ích gì — dùng nội dung
file này khi bạn cần hiểu *vì sao* `tokio::time::sleep` rẻ để tạo và hủy
theo hàng nghìn, không phải để thay thế nó.

## Practice
1. Implement một timer wheel một tầng và benchmark insert, fire, và cancel
   so với implementation `heap.md` của bạn ở 100 nghìn timeout đã lên
   lịch với tỉ lệ hủy sớm 90% (mô phỏng các request hoàn thành trước khi
   timeout của chúng nổ).
2. Mở rộng nó thành một wheel phân cấp hai tầng (ví dụ mili giây và giây)
   và xác nhận một timer được lên lịch vượt phạm vi của wheel đầu tiên di
   chuyển đúng vào wheel chi tiết khi deadline của nó tới gần.
3. Trong `labs/05-reverse-proxy`, thay một pattern
   `tokio::time::sleep`-cho-mỗi-timeout-mỗi-kết-nối ngây thơ bằng một
   thiết kế suy luận về việc hủy hàng loạt (ví dụ đóng kết nối hủy mọi
   timer đang chờ của nó), và giải thích bằng văn bản vì sao
   `tokio::time` đã tránh được chi phí mà bạn vừa đo ở bước 1.
