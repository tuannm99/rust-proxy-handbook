# Skip List

Một cấu trúc có thứ tự đạt cùng độ phức tạp kỳ vọng O(log n) cho
search/insert/delete như một balanced tree, nhưng với câu chuyện concurrent
đơn giản hơn nhiều — đáng biết đến như lựa chọn "ordered map khi
`BTreeMap` không ổn dưới tranh chấp".

## What to learn

### Cấu trúc: các level ngẫu nhiên, không phải rebalancing
Một skip list là một linked list có thêm các tầng "làn nhanh": mọi node
tồn tại ở base list (level 0); mỗi node *cũng* được thăng lên level 1 với
xác suất `p` (thường là 1/2), lên level 2 với xác suất `p²`, và cứ thế.
Một lần search bắt đầu từ level trên cùng và tụt xuống một level mỗi khi
sắp đi quá, bỏ qua phần lớn base list trên đường đi:

```rust
struct SkipNode<K, V> {
    key: K,
    value: V,
    // một con trỏ forward cho mỗi level mà node này tham gia
    forward: Vec<Option<usize>>, // chỉ số trong arena, như trong lru.md
}
```

Không có rotation, không rebalancing — insertion chỉ tung đồng xu để quyết
định node mới tham gia bao nhiêu level, rồi chèn nó vào từng level đó
giống hệt cách insert của một linked list thường. Search/insert/delete kỳ
vọng là O(log n); đây là xác suất chứ không phải worst case đảm bảo, đó là
cái giá phải trả để không cần logic rebalancing cây nào cả.

### Vì sao điều này quan trọng hơn bản thân bound độ phức tạp gợi ý
Rebalancing của một balanced tree (rotation trong red-black tree, split
node trong B-tree) chạm vào nhiều node như một thao tác logic duy nhất,
điều này khó làm lock-free hay thậm chí concurrent thô mà không block cả
một vùng rộng của cấu trúc. Insert của skip list chỉ bao giờ chèn con trỏ
ở các level mà node mới tham gia — một tập nhỏ, cục bộ các cập nhật con
trỏ — đó là lý do skip list lock-free (dùng trong một số in-memory
database và thiết kế memtable đứng sau nhiều storage engine dạng LSM-tree)
khá phổ biến, trong khi balanced tree lock-free hiếm gặp và nổi tiếng là
khó làm đúng.

### Vị trí của nó trong handbook này
Không có lab nào yêu cầu skip list trực tiếp, nhưng nó là cấu trúc tự
nhiên cho bất kỳ bài toán "giữ các entry có thứ tự theo một key thay đổi,
và map bị sửa dưới truy cập đồng thời" — ví dụ theo dõi các cache entry
theo thời gian expiry để [`labs/10-cache`](../../labs/10-cache) tìm "cái gì expire tiếp theo"
mà không cần quét toàn bộ cache, một lựa chọn thay thế cho timer wheel
([`13-algorithms/priority-queue.md`](priority-queue.md)) khi thứ tự chính xác quan trọng hơn
việc phân bucket O(1).

## Practice
1. Implement một skip list single-threaded như một ordered set, với thăng level
   ngẫu nhiên (`p = 0.5`) và node được lưu trong arena.
2. Đo số lượng level qua 10.000 lần insert ngẫu nhiên và xác nhận phân
   phối chiều cao level khớp gần đúng với phân phối hình học mà `p` dự
   đoán.
3. Dùng nó trong [`labs/10-cache`](../../labs/10-cache) để giữ các entry có thứ tự theo thời gian
   expire; implement "evict mọi thứ đã expire" như một lần đi từ key nhỏ
   nhất thay vì quét toàn bộ, và so sánh chi phí với một lần quét tuyến
   tính ở 10.000 entry.
4. (Nâng cao) Đọc về một thiết kế skip list lock-free (ví dụ thiết kế dùng
   trong `ConcurrentSkipListMap` của Java) và viết ra, bằng lời của bạn, vì
   sao insert của nó có thể làm với một số lượng thao tác CAS bounded, trong khi rebalancing của balanced tree thì nhìn chung không thể.
