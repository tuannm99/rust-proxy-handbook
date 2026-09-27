# LRU và Cache Eviction

`05-http-stack/07-cache.md` nói về ngữ nghĩa cache HTTP (freshness, `Vary`,
invalidation). File này nói về chính sách eviction bên dưới: vứt bỏ cái gì
khi cache đầy.

## What to learn

### LRU: cấu trúc
Least Recently Used evict entry không bị chạm vào lâu nhất. Implementation
kinh điển ghép một hash map với một danh sách liên kết đôi intrusive: map
cho lookup O(1), danh sách cho move-to-front O(1) khi hit và eviction O(1)
từ đuôi.

```rust
struct Node<K, V> {
    key: K,
    value: V,
    prev: Option<usize>, // chỉ số vào một arena, không phải con trỏ thô
    next: Option<usize>,
}
```

Gotcha: viết cái này với `Rc<RefCell<Node>>` tạo ra các chu trình tham
chiếu không bao giờ được giải phóng, và viết nó với con trỏ thô nghĩa là
`unsafe` thật sự (xem `03-rust/03-unsafe.md`). Câu trả lời idiomatic trong
Rust là một arena — lưu node trong một `Vec` và dùng chỉ số `usize` làm
liên kết, điều này khiến toàn bộ cấu trúc an toàn, gọn, và thân thiện với
cache. Đây là cùng kỹ thuật với `13-algorithms/slab.md`; một slab là kho
lưu trữ tự nhiên cho một LRU. Trong production, dùng crate `lru` hoặc
`moka` thay vì tự viết tay.

### Vấn đề về concurrency
Điểm chết người của LRU trong một proxy là **mỗi lần đọc cũng là một lần
ghi**: một cache hit phải di chuyển entry lên đầu danh sách, nên một LRU
dùng chung cần một lock độc quyền cho mỗi lần lookup. Dưới mức độ đồng
thời của một proxy, cái lock duy nhất đó trở thành nút thắt cổ chai, và nó
xảy ra chính xác lúc cache đang hoạt động tốt (hit rate cao = lượng
traffic vào lock cao nhất).

Các cách giảm thiểu, theo mức độ tinh vi tăng dần:
- **Sharding** cache thành K LRU độc lập theo key `hash(key) % K`. Mỗi
  cái có lock riêng; contention giảm khoảng K lần. Eviction trở thành theo
  từng shard thay vì toàn cục, đây là một tổn thất độ chính xác nhỏ đổi
  lấy một lợi ích lớn. Đây là bước đi đầu tiên tiêu chuẩn.
- **Gộp các cập nhật recency**: ghi lại hit vào một ring buffer lock-free
  và áp dụng chúng vào danh sách theo chu kỳ dưới một lock. Đây là cách
  `moka` và Caffeine làm.
- **LRU xấp xỉ** (CLOCK, bên dưới), không cần danh sách nào cả.

### CLOCK: LRU xấp xỉ không cần danh sách
CLOCK giữ các entry trong một mảng vòng, mỗi cái có một bit tham chiếu.
Một hit set bit đó — một lần ghi atomic đơn, không lock, không phẫu thuật
con trỏ. Khi eviction, một "kim đồng hồ" quét qua vòng tròn: nếu bit được
set, xóa nó và đi tiếp; nếu chưa set, evict. Các entry được chạm vào kể từ
lượt quét trước sống sót một vòng, xấp xỉ recency đủ sát cho phần lớn
workload với một phần nhỏ chi phí điều phối. Đây là thứ mà page cache của
Linux dùng (`16-kernel/08-page-cache.md`).

### Điểm mù của LRU: scan
Một lượt quét qua một tập lớn các item chỉ dùng một lần (một crawler đi
qua mọi URL, một job backup) evict toàn bộ working set dù không item nào
trong số đó sẽ được đọc lại. LRU không thể phân biệt "vừa dùng một lần"
với "dùng liên tục," vì nó chỉ theo dõi recency, không bao giờ theo dõi
frequency.

LFU theo dõi frequency thay vào đó và kháng được scan, nhưng thích ứng kém
khi working set thực sự thay đổi — một item phổ biến hôm qua vẫn giữ số
đếm cao hôm nay. Các câu trả lời thực tế kết hợp cả hai:
- **ARC** cân bằng một danh sách recency và một danh sách frequency, dịch
  chuyển capacity giữa chúng dựa trên cái nào đang tạo ra hit.
- **TinyLFU / W-TinyLFU** đặt một cửa sổ admission LRU nhỏ trước một cache
  chính dựa trên frequency, dùng một count-min sketch
  (`13-algorithms/count-min-sketch.md`) để ước lượng frequency trong vài
  bit mỗi key. Một item mới chỉ được admit nếu frequency ước lượng của nó
  thắng entry mà nó sẽ evict. Đây là lựa chọn mặc định hiện tại — đây là
  thứ `moka` implement — và nó kháng scan theo thiết kế.

Gotcha: đo hit rate với traffic *của bạn* trước khi chọn. Một proxy đứng
trước một tập nhỏ endpoint hot hoạt động tốt với LRU sharded thuần; các
chính sách tinh vi hơn xứng đáng với độ phức tạp của chúng trên các
workload đuôi dài (long-tail).

### Sizing theo byte, không theo entry
Một cache response HTTP giữ các kích thước object cực kỳ khác nhau — JSON
200 byte cạnh video 50 MB. Một cache giới hạn theo *số lượng* entry có bộ
nhớ không giới hạn. Giới hạn theo tổng byte, và evict theo vòng lặp cho
tới khi object đến vừa. Cũng giới hạn kích thước object tối đa có thể
cache, nếu không một response lớn sẽ evict hàng nghìn response nhỏ đang
hot.

## Practice
1. Trong `labs/10-cache`, implement một LRU dựa trên arena (chỉ số, không
   phải con trỏ) giới hạn theo tổng byte response thay vì số lượng entry;
   thêm một giới hạn kích thước object tối đa có thể cache.
2. Viết test eviction quan trọng: insert cho tới đầy, xác minh entry
   *đọc* gần đây nhất ít nhất bị evict, không phải entry được *insert*
   gần đây nhất ít nhất.
3. Chứng minh nút thắt cổ chai của lock: chạy cache từ 8 task đồng thời
   với hit rate cao dùng một `Mutex<Lru>` duy nhất, ghi lại throughput,
   rồi shard nó 16 phần và đo lại.
4. Tái tạo ô nhiễm do scan: xây một working set hot, xác nhận hit rate
   cao, rồi chạy một lượt qua gấp 10 lần số key nguội riêng biệt và đo hit
   rate sau đó.
5. Implement CLOCK như một chính sách thay thế đứng sau cùng trait; so
   sánh hit rate và throughput đồng thời với LRU sharded trên cả workload
   hot lẫn workload scan từ bước 4.
