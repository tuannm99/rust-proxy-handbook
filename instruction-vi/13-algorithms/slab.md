# Slab

Lưu trữ object kích thước cố định với insert và remove O(1), đánh địa chỉ
bằng chỉ số nguyên thay vì con trỏ. Cấu trúc dữ liệu đứng sau bảng kết
nối, node của LRU, và các đồ thị dựa trên arena.
[`14-memory/05-slab-allocator.md`](../14-memory/05-slab-allocator.md) nói về góc nhìn ở tầng allocator; file
này nói về cấu trúc dữ liệu bạn thực sự dùng trong Rust.

## What to learn

### Vì sao chỉ số thắng con trỏ trong Rust
Một proxy cần một tập hợp các object tồn tại lâu dài, có thể xóa từng cái
riêng lẻ — một entry cho mỗi kết nối đang hoạt động, một node cho mỗi entry
LRU. Các dạng hiển nhiên đều có vấn đề: `HashMap<Id, T>` hash mỗi lần truy
cập và rải rác các allocation; `Vec<T>` làm mọi chỉ số vô hiệu khi remove;
đồ thị `Rc<RefCell<T>>` leak khi có chu trình; con trỏ raw nghĩa là
`unsafe` ([`03-rust/03-unsafe.md`](../03-rust/03-unsafe.md)).

Một slab là một `Vec` gồm các slot mà **remove không dịch chuyển gì cả** —
slot vừa giải phóng gia nhập một free list, nên mọi chỉ số đang tồn tại
vẫn hợp lệ. Bạn có tốc độ truy cập kiểu mảng, handle ổn định, một
allocation liền mạch duy nhất, và không cần `unsafe`. Đó là lý do
`Slab<T>` xuất hiện trong các connection registry và vì sao một LRU dựa
trên arena ([`13-algorithms/lru.md`](lru.md)) là cách implement idiomatic trong
Rust.

### Free list sống ngay bên trong các slot
Phần tinh tế: free list không cần lưu trữ riêng. Một slot trống chứa chỉ
số của slot trống *tiếp theo*, nên toàn bộ free list được xâu chuỗi ngay
trong các slot chưa dùng mà không tốn thêm bộ nhớ nào.

```rust
enum Slot<T> {
    Occupied(T),
    Vacant { next_free: Option<usize> },
}

struct Slab<T> {
    slots: Vec<Slot<T>>,
    next_free: Option<usize>, // đầu của free list
    len: usize,
}
```

Insert pop phần đầu của free list (hoặc push một slot mới nếu rỗng);
remove ghi `Vacant` vào slot và push nó lên đầu. Cả hai đều O(1) và không
allocate ở trạng thái ổn định.

Gotcha: `Slot<T>` lớn bằng cái lớn hơn giữa `T` và một `usize`, cộng thêm
discriminant của enum. Với `T` nhỏ, chi phí đó là tương đối lớn; các
implementation slab production nén discriminant vào một bit dư hoặc giữ
một bitmap occupancy riêng.

### Vấn đề ABA / stale-handle
Đây là con bug hay cắn người. Kết nối 7 đóng, giải phóng slot 7; một kết
nối mới lập tức tái sử dụng slot 7. Bất kỳ đoạn code nào vẫn giữ chỉ số 7
— một timer đang chờ, một response đang bay, một callback metric — giờ
đọc hoặc sửa *nhầm kết nối*. Không có lỗi kiểu dữ liệu, không panic; đó là
sự nhiễu chéo âm thầm giữa các client không liên quan, mà trong một proxy
nghĩa là response của người dùng này đến tay người khác.

Cách sửa là một **generation counter**: gắn mỗi slot với một counter tăng
lên mỗi lần remove, và biến handle công khai thành `(index, generation)`.
Khi truy cập, kiểm tra generation của slot khớp với generation trong handle
không, trả về `None` nếu không khớp, biến một bug aliasing âm thầm thành
một lượt lookup miss bình thường.

Gotcha: generation phải tăng khi *remove*, không phải khi insert, và nó
phải đủ rộng để không bị tràn (wrap) trong suốt vòng đời của bất kỳ handle
nào còn tồn tại — một `u32` khi connection churn cao là điều đáng cân nhắc
kỹ chứ không nên mặc định là đủ.

### Capacity và vấn đề shrink
Một slab không bao giờ tự co lại: sau khi một đợt spike traffic tạo ra
100 nghìn slot kết nối, `Vec` vẫn giữ nguyên độ rộng 100 nghìn slot ngay
cả khi chỉ còn 100 kết nối active. Với một proxy chạy dài hạn, đó là một
mức đỉnh bộ nhớ vĩnh viễn được set bởi đợt spike tệ nhất của bạn
(xem [`14-memory/06-fragmentation.md`](../14-memory/06-fragmentation.md)).

Nén lại nghĩa là di chuyển các entry đang chiếm dụng vào các slot thấp,
điều này làm vô hiệu chỉ số của chúng — đúng chính cái tính chất mà slab
tồn tại để cung cấp. Các lựa chọn thực tế là chặn trên slab và từ chối khi
vượt capacity (hợp lý: nó kiêm luôn vai trò giới hạn số kết nối), hoặc
chấp nhận mức đỉnh đó như một chi phí. Pre-size với `with_capacity` cho
tải đỉnh dự kiến cũng tránh được việc reallocate lặp lại trong lúc ramp-up,
khi proxy vốn đã đang chịu áp lực.

## Practice
1. Implement `Slab<T>` với free list nội tại ở trên — `insert`, `remove`,
   `get`, `get_mut` — và assert rằng remove một chỉ số thấp không làm ảnh
   hưởng đến các chỉ số khác.
2. Tái hiện bug stale-handle: giữ một chỉ số qua một chu trình remove+insert
   và quan sát nó đọc phải người chiếm chỗ mới. Sau đó thêm generation
   counter và xác nhận cùng một truy cập giờ trả về `None`.
3. Dùng nó trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) làm connection registry, đánh
   chỉ mục bằng một handle có generation thay vì bằng địa chỉ.
4. So sánh với `HashMap<u64, T>` qua 100 nghìn chu trình insert/remove/lookup
   — đo cả thời gian lẫn bộ nhớ đỉnh.
5. Chứng minh vấn đề shrink: tăng lên 100 nghìn entry, remove hết trừ 100,
   và cho thấy bộ nhớ không quay về. Thêm một giới hạn capacity và test từ
   chối insert vượt quá nó.
6. Dùng slab của bạn thay cho một `Vec` trần để làm nền cho LRU trong
   [`13-algorithms/lru.md`](lru.md).
