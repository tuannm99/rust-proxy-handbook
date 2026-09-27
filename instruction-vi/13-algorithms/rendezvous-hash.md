# Rendezvous Hashing (HRW)

Highest Random Weight: một lựa chọn thay thế cho consistent hashing
([`13-algorithms/consistent-hash.md`](consistent-hash.md)) không cần ring, không cần virtual
node, và không cần shared mutable state.

## What to learn

### Thuật toán
Với một key, tính `hash(key, upstream)` cho *mọi* upstream và chọn upstream
có giá trị cao nhất. Toàn bộ thuật toán chỉ có vậy.

```rust
fn pick<'a>(key: &str, upstreams: &'a [Upstream]) -> &'a Upstream {
    upstreams
        .iter()
        .max_by_key(|u| hash64(key, &u.id))
        .expect("pool is non-empty")
}
```

Vì điểm số chỉ phụ thuộc vào `(key, upstream_id)`, mọi instance proxy tự
tính ra cùng một đáp án một cách độc lập mà không cần phối hợp gì, và
không có cấu trúc dữ liệu nào cần xây, xây lại, hay khóa.

### Vì sao nó remap tối thiểu
Xóa một upstream: chỉ những key có *người thắng* là upstream đó mới di
chuyển, và chúng chuyển sang bất kỳ upstream nào có điểm cao thứ nhì — mọi
key khác không bị ảnh hưởng, vì xóa một upstream không phải người thắng
không thể thay đổi ai là max. Đó chính xác là tỷ lệ remap ~1/N mà
consistent hashing đạt được, nhưng ở đây nó đến từ chính thuật toán chứ
không phải từ việc tinh chỉnh số lượng virtual node.

Thêm một upstream là hình ảnh đối xứng: một key chỉ di chuyển nếu upstream
mới vượt điểm của người thắng hiện tại, xảy ra với ~1/(N+1) số key.

### Chất lượng phân phối so với consistent hashing
Độ cân bằng của consistent hashing phụ thuộc vào việc các virtual node
tình cờ rơi đều trên ring hay không — với quá ít virtual node bạn sẽ gặp
skew thật sự, đó là lý do 100-200 virtual node mỗi upstream là con số phổ
biến (và cũng là lý do ring tốn bộ nhớ tỷ lệ với `upstreams × vnodes`).
Rendezvous không có núm chỉnh nào như vậy: phân phối đều đến mức nào chỉ
phụ thuộc vào hash function của bạn, hết. Với một proxy có số lượng
upstream khiêm tốn, đây là cách ít khả năng làm sai hơn hẳn.

Gotcha: hash phải thực sự trộn upstream id vào hash của key.
`hash(key) ^ hash(upstream)` trông có vẻ ổn nhưng không hề — XOR với một
hằng số cố định theo từng upstream giữ nguyên cấu trúc bit của key, nên
các key tương quan sẽ có điểm số tương quan và phân phối bị skew. Hãy hash
chuỗi ghép (hoặc đưa cả hai vào cùng một hasher), và dùng một hash có
avalanche behavior tốt (xxHash, SipHash, tối thiểu là FNV-1a) thay vì
`DefaultHasher` mặc định, vốn có output không ổn định giữa các bản Rust —
một sự thật quan trọng ngay khi hai instance proxy trên hai build khác
nhau phải đồng thuận về cùng một người thắng.

### Rendezvous có trọng số
Trọng số được đưa vào qua một phép biến đổi log: tính điểm mỗi upstream là
`weight / -ln(h)` với `h` là hash được chuẩn hóa về (0,1). Upstream có
điểm biến đổi cao nhất thắng, và xác suất được chọn tỷ lệ với weight trong
khi vẫn giữ tính chất disruption tối thiểu. Việc này khó làm đúng hơn hẳn
so với smooth WRR ([`13-algorithms/smooth-wrr.md`](smooth-wrr.md)) — chỉ dùng nó khi bạn
cần cả trọng số *lẫn* affinity cùng lúc.

### Chi phí thực tế
Selection tốn O(N) hash mỗi request, so với O(log N) cho ring lookup và
O(1) cho Maglev ([`13-algorithms/maglev.md`](maglev.md)). Ở vài chục upstream, N hash
của một chuỗi ngắn chỉ tốn vài chục nanosecond và sự đơn giản thắng thế.
Ở hàng nghìn upstream đây là lựa chọn sai — đó là địa hạt của Maglev.

## Practice
1. Implement HRW trong [`labs/06-load-balancer`](../../labs/06-load-balancer) đằng sau cùng một trait với
   round-robin và consistent-hash của bạn, để cả ba có thể hoán đổi cho
   nhau.
2. Hash 100 nghìn key tổng hợp trên 10 upstream và báo cáo phân phối theo
   từng upstream — xác nhận nó lệch khỏi đều chỉ vài phần trăm.
3. Xóa một upstream, chạy lại cùng 100 nghìn key, và đo chính xác tỷ lệ
   thay đổi người thắng. Xác nhận nó ~1/10, và mỗi key di chuyển đều
   chuyển sang một upstream *khác* với upstream vừa bị xóa.
4. Thay hash bằng `hash(key) ^ hash(upstream_id)` và chạy lại bước 2 với
   các key có chung một prefix dài (ví dụ `session-00001`...); quan sát
   skew mà nó tạo ra.
5. So sánh latency selection của HRW với ring lookup của bạn ở 10, 100, và
   1000 upstream; tìm điểm giao nhau nơi ring thắng thế.
