# Binary Heap

Biến thể least-outstanding-requests của `06-proxy/02-load-balancer.md`
cần luôn biết upstream nào hiện có ít request đang xử lý dở nhất nhất.
Một binary heap là cấu trúc trả lời "cái nào nhỏ nhất" trong O(log n) cho
mỗi lần cập nhật thay vì quét mọi upstream ở mỗi request.

## What to learn

### Cấu trúc: một cây hoàn chỉnh đóng gói trong một mảng
Một binary heap lưu một cây nhị phân hoàn chỉnh trực tiếp trong một `Vec`,
không có con trỏ: với chỉ số `i`, cha nằm ở `(i - 1) / 2` và con nằm ở
`2i + 1` và `2i + 2`. Bất biến của heap — mọi cha đều `<=` (min-heap) hoặc
`>=` (max-heap) con của nó — được duy trì bằng hai thao tác: **sift-up**
(một phần tử mới hoặc bị giảm nổi dần về phía root khi nó nhỏ hơn cha) và
**sift-down** (root, sau khi bị xóa, chìm dần khi nó lớn hơn con nhỏ nhất
của nó).

```rust
// về mặt khái niệm, đây là những gì std::collections::BinaryHeap làm bên trong,
// dưới dạng một min-heap trên (outstanding_count, upstream_id)
fn sift_down(heap: &mut Vec<(u32, usize)>, mut i: usize) {
    loop {
        let (l, r) = (2 * i + 1, 2 * i + 2);
        let mut smallest = i;
        if l < heap.len() && heap[l] < heap[smallest] { smallest = l; }
        if r < heap.len() && heap[r] < heap[smallest] { smallest = r; }
        if smallest == i { break; }
        heap.swap(i, smallest);
        i = smallest;
    }
}
```

Peek-min là O(1) (nó luôn là root, chỉ số 0); insert và remove-min đều là
O(log n).

### Khoảng trống mà `BinaryHeap` chuẩn để lại: decrease-key
Least-outstanding-requests cần *thay đổi* key của một upstream (số request
đang xử lý dở tăng khi dispatch, giảm khi hoàn thành) và giữ heap hợp lệ
— một thao tác "decrease-key" mà `std::collections::BinaryHeap` của Rust
hoàn toàn không expose; nó chỉ hỗ trợ push và pop-max/min. Hai cách sửa
thực tế:
- **Lazy deletion**: push một entry mới với số đếm đã cập nhật, để entry
  cũ nằm nguyên tại chỗ, và khi pop, bỏ qua (loại bỏ) bất kỳ entry nào
  không còn khớp với số đếm hiện tại của upstream (được theo dõi riêng
  trong một `HashMap<UpstreamId, u32>`). Đơn giản, nhưng heap có thể tích
  lũy các entry cũ giữa các lần pop thật.
- **Indexed heap**: duy trì một bảng phụ ánh xạ key → chỉ số trong heap,
  và cập nhật bảng đó ở mỗi lần swap trong quá trình sift-up/down, để một
  decrease-key có thể tìm trực tiếp phần tử của nó và sift nó trong
  O(log n) mà không cần quét. Nhiều sổ sách hơn, không tích lũy entry cũ.

### d-ary heap: ít tầng hơn, hành vi cache tốt hơn
Một heap với `d` con cho mỗi node (thay vì 2) có độ sâu `log_d(n)` — ít
tầng hơn để sift qua — với cái giá là phải so sánh tới `d` con ở mỗi bước
sift-down thay vì 2. Với `d` nhỏ (4, đôi khi 8) đây là một chiến thắng
ròng trong thực tế vì nó thực hiện ít lần nhảy qua ranh giới cache-line
hơn cho một tổng số phép so sánh tương tự; các implementation
priority-queue production (một số OS scheduler, một số implementation LB)
dùng d-ary heap chính vì lý do này. Xem `17-performance/01-cpu-cache.md`
trước khi micro-tune `d` — đo trước đã.

## Practice
1. Trong `labs/06-load-balancer`, implement least-outstanding-requests
   dùng một min-heap có key là số request đang xử lý dở; bắt đầu với
   cách lazy-deletion cho decrease-key.
2. Tái tạo vấn đề entry cũ: dispatch và hoàn thành request thật nhanh và
   xác nhận kích thước heap tăng lên so với số upstream thật; đo xem nó
   tốn bao nhiêu trước khi một lần pop dọn sạch nó.
3. Thay lazy deletion bằng một indexed heap (một `HashMap` từ upstream ID
   tới chỉ số heap hiện tại, cập nhật ở mỗi lần swap) và xác nhận kích
   thước heap luôn bị giới hạn đúng bằng số upstream.
4. (Stretch) Implement một biến thể 4-ary heap và benchmark chi phí
   sift-down so với phiên bản nhị phân ở vài trăm upstream.
