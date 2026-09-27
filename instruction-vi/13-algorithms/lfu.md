# LFU (Least Frequently Used)

`13-algorithms/lru.md` nói về eviction dựa trên recency và điểm mù của nó
trước các lượt scan. File này nói về giải pháp thay thế dựa trên
frequency và vì sao nó không phải một bản thay thế đơn giản.

## What to learn

### Ý tưởng cơ bản, và cái giá của implementation ngây thơ
LFU evict entry có số lần truy cập thấp nhất, dựa trên lý thuyết rằng các
item được dùng thường xuyên đáng giữ hơn các item mới được dùng gần đây.
Implementation ngây thơ — một hash map ánh xạ tới một số đếm, và một lượt
quét tuyến tính để tìm giá trị nhỏ nhất khi evict — là O(n) cho mỗi lần
evict, đây là lý do LFU có tiếng là chậm hơn LRU.

### LFU O(1): hai tầng cấu trúc liên kết
Cách xây O(1) chuẩn (Ketan Shah và cộng sự) giữ một danh sách liên kết đôi
các bucket theo *frequency*, mỗi bucket giữ một danh sách liên kết đôi các
key hiện đang ở frequency đó:

```rust
struct FreqNode<K> {
    freq: u64,
    keys: std::collections::HashSet<K>, // hoặc một intrusive list, để remove O(1)
    prev: Option<usize>,
    next: Option<usize>, // chỉ số arena, như trong lru.md — cùng lý do áp dụng
}
```

Một hit gỡ key khỏi bucket frequency hiện tại của nó, tăng số đếm, và
insert nó vào bucket cho số đếm mới (tạo bucket đó nếu chưa tồn tại, ngay
sau bucket cũ trong danh sách). Eviction gỡ một key khỏi bucket frequency
ở *đầu* (số đếm thấp nhất hiện có) — không bao giờ quét. Cả hai thao tác
đều O(1) khấu hao, cùng lớp độ phức tạp với LRU, với cái giá là một cấu
trúc phức tạp hơn và nhiều sổ sách hơn cho mỗi lần truy cập.

### Vấn đề mà LRU không có: stale winner
Số đếm của LFU chỉ có thể tăng (ở dạng thuần như trên), nên một item cực
kỳ phổ biến hôm qua và không bao giờ bị chạm tới nữa vẫn giữ số đếm cao
mãi mãi và trên thực tế không thể bị evict — nó bóp nghẹt các item đang
phổ biến *ngay bây giờ*. Đây là ảnh gương của lỗ hổng scan của LRU: LRU
quên hoàn toàn frequency, LFU thuần không bao giờ quên nó.

Cách giảm thiểu là **aging/decay**: định kỳ giảm một nửa mọi số đếm (hoặc
decay theo cấp số nhân theo chu kỳ, hoặc ở mỗi lần truy cập thứ N), để độ
phổ biến cũ phai đi và độ phổ biến mới có thể cạnh tranh công bằng. Điều
này biến LFU thuần thành một xấp xỉ của "frequency trong một cửa sổ gần
đây" thay vì "frequency từ trước tới giờ," đây gần như luôn là thứ bạn
thực sự muốn trong một cache.

### Vì sao cache production không ship LFU thuần
Giữa độ phức tạp của danh sách bucket và bài toán tuning aging, LFU thuần
hiếm khi được dùng nguyên bản trong cache production. Hai hướng sửa nó
theo cách khác nhau:
- **ARC** (`13-algorithms/arc.md`) theo dõi cả recency lẫn frequency và tự
  động điều chỉnh tỉ lệ giữa chúng, không cần một núm decay thủ công.
- **TinyLFU** (`13-algorithms/tinylfu.md`) giữ *ý tưởng* frequency nhưng
  thay bộ đếm chính xác bằng một count-min sketch xác suất
  (`13-algorithms/count-min-sketch.md`) có sẵn cơ chế aging định kỳ, và
  chỉ dùng nó như một bộ lọc *admission* đứng trước một cấu trúc chính đơn
  giản hơn nhiều (thường dựa trên LRU) thay vì làm chính sách eviction cho
  toàn bộ cache.

Gotcha: đừng tự viết tay LFU trong `proxy/` — nó ở đây để bạn nhận ra sự
đánh đổi bằng tên gọi và hiểu ARC với TinyLFU thực sự đang cải thiện điều
gì. Các hệ thống production (`moka`, Caffeine) dùng các thiết kế dẫn xuất
từ TinyLFU, không phải LFU thuần, chính vì lý do stale ở trên.

## Practice
1. Trong `labs/10-cache`, implement cấu trúc LFU O(1) (danh sách bucket
   frequency chứa danh sách key) đứng sau cùng một trait eviction bạn đã
   dùng cho LRU.
2. Cố tình tái tạo bug stale: làm một key cực kỳ phổ biến, ngừng chạm vào
   nó, rồi làm ngập cache bằng một working set khác, liên tục thay đổi —
   xác nhận key stale đó không bao giờ bị evict dù đã nguội, và đo mức
   giảm hit-rate so với LRU trên cùng trace.
3. Thêm việc giảm một nửa số đếm định kỳ (aging) và chạy lại cùng trace;
   xác nhận key stale cuối cùng trở nên có thể evict được và hit rate phục
   hồi.
4. So sánh độ phức tạp implementation và tính đúng đắn của eviction với
   LRU dựa trên arena từ `13-algorithms/lru.md` trên cùng một bộ benchmark,
   và viết ra, một cách cụ thể, hình dạng workload nào (tập hot ổn định so
   với độ phổ biến thay đổi so với scan one-shot) thiên về chính sách nào.
