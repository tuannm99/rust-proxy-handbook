# TinyLFU / W-TinyLFU

[`13-algorithms/lru.md`](lru.md) giới thiệu TinyLFU sơ lược như "một cửa sổ admission
LRU nhỏ đứng trước một cache chính dựa trên tần suất." File này là phần
đào sâu: đây là thiết kế eviction/admission mà đa số cache Rust và JVM
production thực sự dùng (`moka`, Caffeine), và đáng để hiểu vì sao nó
thắng các cách tiếp cận trực tiếp hơn của ARC và LFU.

## What to learn

### Đổi khung vấn đề: admission, không phải eviction
Mọi policy trước đó (LRU, LFU, ARC) trả lời câu hỏi "entry đang có nên
evict cái nào?" TinyLFU đặt một câu hỏi khác trước tiên: **"item mới, chưa
có trong cache này, có nên được cho vào không?"** Một candidate chỉ được
admit nếu tần suất truy cập ước tính của nó cao hơn tần suất của entry sẽ
phải bị evict để nhường chỗ cho nó. Nếu không, candidate đơn giản bị bỏ
qua — nội dung cache không đổi, và không có sổ sách eviction nào xảy ra
cả.

Chính cách đổi khung này khiến TinyLFU chống scan *bằng cấu trúc*: các
item của một lần scan one-shot có tần suất ~1, gần như luôn thấp hơn bất
cứ thứ gì đang có sẵn và "ấm", nên chúng bị từ chối ngay ở cửa thay vì
đẩy trôi working set theo cách một lần scan đẩy trôi LRU.

### Bộ ước tính tần suất: count-min sketch, không phải counter mỗi key
Lưu một counter chính xác cho mỗi key (như LFU thường làm) tốn bộ nhớ tỷ
lệ với số lượng key khác nhau từng thấy, điều này không bounded với
một cache của proxy. Thay vào đó, TinyLFU dùng một count-min sketch
([`13-algorithms/count-min-sketch.md`](count-min-sketch.md)) — kích thước cố định, nhỏ (vài bit
cho mỗi key dự kiến), với sai số overestimation bounded và không có
allocation nào cho mỗi key.

```rust
struct TinyLfu {
    sketch: CountMinSketch, // 13-algorithms/count-min-sketch.md
    total_increments: u64,
    reset_threshold: u64,   // ví dụ 10 lần số key dự kiến của sketch
}
// admit(candidate_key, victim_key):
//   sketch.estimate(candidate_key) > sketch.estimate(victim_key)
```

Gotcha: nếu không có aging định kỳ, các counter của sketch chỉ tăng và
cuối cùng bão hòa, lúc đó bộ ước tính mất hết khả năng phân biệt — mọi key
đủ phổ biến đều trông "phổ biến vô hạn" như nhau. Hãy chia đôi mọi counter
trong sketch mỗi khi `total_increments` vượt `reset_threshold`. Đây chính
là vấn đề aging mà [`13-algorithms/lfu.md`](lfu.md) gặp phải với counter chính xác,
nhưng được giải quyết ở đây bằng cách reset định kỳ một cấu trúc kích
thước cố định thay vì decay một map ngày càng lớn.

### Doorkeeper: đừng để các key chỉ trúng một lần làm nhiễu sketch
Một cache đứng trước traffic internet tùy ý sẽ thấy một lượng khổng lồ các
key chỉ được truy cập đúng một lần, mãi mãi. Ghi nhận tất cả chúng vào
count-min sketch lãng phí dung lượng sketch và thêm nhiễu làm giảm chất
lượng ước tính cho các key thực sự quan trọng. Cách sửa tiêu chuẩn là một
**doorkeeper**: một Bloom filter mà một key phải đi qua hai lần trước khi
được đếm vào sketch — lần truy cập đầu chỉ set bit Bloom; chỉ lần truy cập
*thứ hai* (một lần lặp lại thật sự) mới tăng sketch. Điều này giữ số bit
hạn chế của sketch dành cho các key đã xuất hiện nhiều hơn một lần.

### W-TinyLFU: thêm lại một cửa sổ recency
Admission thuần túy dựa trên tần suất có điểm mù riêng: một item *mới*
đang hot có tần suất 0 ở lần thấy đầu tiên và sẽ không bao giờ được admit
chỉ dựa trên tần suất, dù nó sắp trở nên hot đến đâu — một vấn đề cold-start
cho bất cứ thứ gì thực sự mới. **W-TinyLFU** (windowed TinyLFU) sửa điều
này bằng cách trích ra một phần nhỏ của cache (thường ~1%) làm một
**window** LRU thuần, và chỉ chạy logic admission/TinyLFU trên phần
**main** cache còn lại ~99%:

- Entry mới luôn vào window LRU nhỏ trước.
- Một entry bị evict khỏi window sẽ cạnh tranh để được admit vào main
  cache thông qua kiểm tra tần suất TinyLFU ở trên.
- Bản thân main cache thường là một **Segmented LRU (SLRU)**: một segment
  *probationary* (vừa được admit) và một segment *protected* (đã sống sót
  qua lần truy cập thứ hai) — về khái niệm gần với tầng "đã thấy nhiều hơn
  một lần" của LFU, nhưng có thứ tự theo LRU trong từng segment thay vì
  chia bucket theo tần suất.

Đây chính là thiết kế của `moka` và Caffeine: window LRU cho recency ở
cold-start, admission được gác bởi count-min sketch cho khả năng chống
scan, main cache SLRU cho phân tầng tần suất xấp xỉ với chi phí rẻ —
không có sổ sách bốn-danh-sách kiểu ARC, không có chi phí bộ nhớ counter
chính xác kiểu LFU thuần.

### Vì sao cách này thắng ARC và LFU thuần trong thực tế
Bộ ước tính tần suất của TinyLFU có bộ nhớ cố định O(1) bất kể cardinality
của key (ARC và LFU chính xác thì không — sổ sách của chúng scale theo số
key khác nhau đã chạm tới, kể cả các key ghost/đã evict). Thiết kế
admission-trước của nó nghĩa là phần lớn quyết định "cái này có đáng cache
không" chỉ là vài lần tra sketch, không phải một điệu nhảy thăng cấp qua
nhiều danh sách. Đánh đổi là tín hiệu tần suất chỉ là xấp xỉ (overestimation
bounded, không bao giờ underestimate, từ count-min sketch) thay vì
chính xác — một đánh đổi mà trên thực tế đo lường (các benchmark công bố
của chính Caffeine) gần như không tốn gì về hit rate trong khi tốn ít hơn
nhiều về bộ nhớ và CPU so với ARC hay LFU lý tưởng.

## Practice
1. Trong [`labs/10-cache`](../../labs/10-cache), implement bộ ước tính tần suất dựa trên
   count-min-sketch với chia đôi định kỳ, rồi nối nó vào một kiểm tra
   admission đứng trước LRU đã có ([`13-algorithms/lru.md`](lru.md)) làm main cache
   — đây là TinyLFU chưa có phần sửa cold-start theo windowed.
2. Tái hiện vấn đề cold-start có chủ đích: đưa vào một key hoàn toàn mới
   sắp trở nên rất hot, và xác nhận admission chỉ-dựa-tần-suất từ chối nó
   mọi lần lúc đầu. Sau đó thêm window LRU nhỏ ở trước và xác nhận cùng
   key đó có cơ hội chứng minh bản thân trước khi bị đánh giá theo tần
   suất.
3. Thêm doorkeeper Bloom filter và đo chất lượng sketch (so sánh tần suất
   ước tính với tần suất thật cho một tập key đã biết) có và không có nó,
   trên một trace bị chi phối bởi các key chỉ trúng một lần.
4. Chạy lại cùng bài so sánh ba workload bạn dùng cho [`13-algorithms/arc.md`](arc.md)
   (tập hot ổn định, scan định kỳ, độ phổ biến thay đổi) trên
   implementation W-TinyLFU của bạn, và so sánh cả hit rate *lẫn* mức dùng
   bộ nhớ với ARC và LRU thuần trên cùng trace.
