# ARC (Adaptive Replacement Cache)

`13-algorithms/lru.md` và `13-algorithms/lfu.md` mỗi cái nắm bắt một tín
hiệu — recency hoặc frequency — và mỗi cái đều có một workload đánh bại nó
(LRU: quét one-shot; LFU: stale winner). Điểm bán của ARC là nó không bắt
bạn phải chọn: nó theo dõi cả hai và *thích ứng* (adapt) tỉ lệ giữa chúng
theo thời gian thực, dựa trên tín hiệu nào đang thực sự tạo ra hit cho
traffic của bạn ngay lúc này.

## What to learn

### Bốn danh sách, không phải một
ARC chia các key đang theo dõi vào bốn danh sách theo thứ tự LRU:
- **T1** — các entry mới thấy đúng một lần gần đây (tín hiệu LRU thuần).
- **T2** — các entry đã thấy ít nhất hai lần gần đây (tín hiệu frequency).
- **B1** — danh sách "ghost": các key vừa bị evict khỏi T1 (chỉ có
  metadata, không có giá trị cache).
- **B2** — danh sách ghost: các key vừa bị evict khỏi T2.

`T1` và `T2` cùng nhau giữ các entry cache thật, giới hạn bởi tổng dung
lượng cache `c`. `B1` và `B2` chỉ giữ key, giới hạn xấp xỉ `c` mỗi cái —
chúng tồn tại thuần túy để phát hiện một pattern mà cache đang sống không
còn thấy được nữa.

```rust
struct Arc<K> {
    t1: LruList<K>, t2: LruList<K>,   // resident, tổng độ dài <= c
    b1: LruList<K>, b2: LruList<K>,   // ghost, chỉ chứa key
    p: usize,                          // kích thước mục tiêu của T1, thích ứng theo thời gian
    c: usize,                          // tổng dung lượng
}
```

### Quy tắc thích ứng
`p` là ranh giới mục tiêu hiện tại giữa "không gian recency" và "không
gian frequency" bên trong cache đang sống. Nó dịch chuyển ở mỗi lần hit
vào ghost-list, đây chính là cơ chế khiến ARC tự điều chỉnh:
- Một hit ở **B1** (một key bị evict khỏi danh sách recency quay lại) có
  nghĩa là workload muốn nhiều dung lượng recency hơn — cache đã evict nó
  quá sớm. Tăng `p` (tăng tỉ lệ mục tiêu của T1).
- Một hit ở **B2** (một key bị evict khỏi danh sách frequency quay lại) có
  nghĩa là workload muốn nhiều dung lượng frequency hơn. Giảm `p` (giảm tỉ
  lệ mục tiêu của T1, tăng của T2).

Không có tham số tuning bên ngoài nào quyết định tỉ lệ recency/frequency —
lịch sử miss gần đây của chính cache quyết định điều đó, liên tục, theo
hướng này hoặc hướng kia sau mỗi lần ghost hit.

### Vì sao nó tốt hơn việc chọn sẵn LRU hay LFU
Một workload trộn giữa một tập hot nhỏ (thiên về frequency) với thỉnh
thoảng một lượt scan one-shot lớn (không quan tâm recency, nhưng không
được evict mất tập hot) đánh bại LRU thuần (scan xả trôi các entry tương
đương T2) và không cho LFU thuần cách nào nhanh để admit các item mới trở
nên hot. Ghost list của ARC cho phép nó phát hiện, từ pattern miss thật,
rằng "đang có scan xảy ra nhưng không nên chiếm phần cache của frequency"
và tự điều chỉnh `p` tương ứng, mà không cần con người quyết định điều đó
trước. Khả năng thích ứng này, chứ không phải hit rate thô trên một trace
tĩnh nào đó, mới là đóng góp thật sự của ARC.

### Nơi nó được dùng, và cái giá phức tạp thành thật
ARC là code production thật, không chỉ là một bài báo: cache ARC của ZFS
được đặt tên theo và implement chính thuật toán này cho việc cache disk
block. Nhưng nó đòi hỏi nhiều state và sổ sách hơn đáng kể so với LRU hay
LFU riêng lẻ — bốn danh sách phải giữ nhất quán, và một lần cập nhật `p`
ở mỗi ghost hit không được race với eviction đang diễn ra đồng thời.
(Thuật toán gốc có trước một bằng sáng chế của IBM nay đã hết hạn; ngày
nay implement nó là miễn phí, nhưng lịch sử đó là một phần lý do vì sao nó
mất nhiều năm mới xuất hiện trong các cache mã nguồn mở phổ biến.)

Gotcha: việc sổ sách bốn danh sách chính xác là kiểu thứ dễ sai một cách
tinh vi dưới điều kiện đồng thời — một key không bao giờ được nằm trong
hơn một danh sách cùng lúc, và một lần promote từ T1 lên T2 (khi truy cập
lần hai) phải remove-rồi-insert một cách nguyên tử so với một lần eviction
đang diễn ra đồng thời, nếu không bạn sẽ có một entry trùng lặp hoặc bị rò
rỉ.

## Practice
1. Trong `labs/10-cache`, implement ARC như chính sách eviction thứ tư
   đứng sau trait dùng chung của bạn, dùng danh sách dựa trên arena
   (kỹ thuật của `13-algorithms/lru.md`) cho cả bốn danh sách.
2. Tái tạo kịch bản mà ARC sinh ra để giải quyết: một tập hot nhỏ ổn định
   cộng với một lượt scan one-shot lớn theo chu kỳ. Chạy LRU thuần, LFU
   thuần (`13-algorithms/lfu.md`), và ARC trên cùng một trace rồi so sánh
   hit rate — xác nhận `p` của ARC dịch về phía T1 trong lúc scan và phục
   hồi sau đó mà không mất tập hot.
3. Thêm một assertion rằng không key nào từng xuất hiện trong hơn một
   trong bốn danh sách cùng lúc, và chạy nó dưới truy cập đồng thời để bắt
   một race giữa promotion và eviction.
4. (Stretch) Ghi lại `p` theo thời gian trong một trace có workload dịch
   chuyển dần (thiên recency, rồi thiên frequency) và xác nhận nó theo kịp
   sự dịch chuyển mà không cần tuning thủ công.
