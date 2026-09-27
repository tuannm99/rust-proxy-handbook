# Bloom Filter

`13-algorithms/tinylfu.md` dùng một Bloom filter làm "doorkeeper" để chặn
các one-hit-wonder không cho vào frequency sketch của nó. File này nói về
bản thân cấu trúc: kiểm tra *membership* xác suất, nhanh, nhỏ gọn, không
có false negative.

## What to learn

### Cấu trúc và sự đảm bảo
Một mảng `m` bit và `k` hàm hash độc lập. **Insert**: set các bit
`h_1(key) % m`, ..., `h_k(key) % m`. **Query**: báo "có thể có mặt" nếu cả
`k` bit đều được set, "chắc chắn không có mặt" nếu ngược lại.

```rust
struct BloomFilter {
    bits: Vec<u64>, // m bit, đóng gói
    k: usize,
}
// insert/query đều tính k vị trí hash và set/check bit
```

Vì insertion chỉ set bit chứ không bao giờ khác, một query không bao giờ
có thể cho ra false *negative* — nếu các bit nói vắng mặt, key đó chưa
từng được insert, chấm hết. Nó có thể cho ra false *positive*: đủ các key
khác vô tình che phủ đúng các vị trí của key này.

### Sizing: tỉ lệ false-positive là một cái núm vặn, không phải tình cờ
Với `n` lần insertion dự kiến, số hàm hash tối ưu là
`k = (m/n) * ln(2)`, và tỉ lệ false-positive kết quả xấp xỉ
`(1 - e^(-kn/m))^k`. Cụ thể: 10 bit cho mỗi phần tử dự kiến với `k` tối ưu
(~7) cho tỉ lệ false-positive khoảng 1%, không phụ thuộc vào bản thân các
key là gì. Hãy quyết định tỉ lệ false-positive chấp nhận được trước, rồi
size `m` và `k` từ đó — đừng chọn số tròn rồi hy vọng.

### Nơi nó có chỗ đứng trong handbook này
- **Doorkeeper của TinyLFU** (`13-algorithms/tinylfu.md`): một key phải
  xuất hiện hai lần trước khi được đếm vào frequency sketch, và Bloom
  filter là bước kiểm tra lần-xuất-hiện-đầu-tiên rẻ tiền.
- **Kiểm tra sơ bộ IP/rule blocklist** (`07-security/06-waf.md`,
  `07-security/08-ip-filtering.md`): việc check một deny-list lớn thường
  bị chi phối bởi "trường hợp phổ biến là không nằm trong danh sách" —
  một Bloom filter đứng trước lookup thật trả lời "chắc chắn không bị
  chặn" cho phần lớn traffic trong O(k) mà không cần truy cập bộ nhớ nào
  ngoài bản thân filter, và chỉ rơi xuống kiểm tra thật đắt tiền khi có
  khả năng trùng.

### Gotcha: không xóa được, và không được vượt capacity
Một Bloom filter thuần không thể un-set một bit cho một key mà không có
khả năng phá vỡ membership của mọi key khác cũng set bit đó — xóa cần một
cấu trúc khác (**counting Bloom filter**, dùng bộ đếm nhỏ thay vì bit đơn,
với bộ nhớ tăng theo tỉ lệ). Và một filter được size cho `n` phần tử mà
nhận số insertion nhiều hơn đáng kể so với `n` sẽ thấy tỉ lệ false-positive
tăng vọt qua khỏi con số bạn đã thiết kế — hãy size theo cardinality dự
kiến thật sự của bạn, và giám sát số insertion thật so với con số đó nếu
nó có thể tăng vô hạn.

## Practice
1. Implement một Bloom filter được size cho một `n` và mục tiêu
   false-positive đã chọn; đo thực nghiệm tỉ lệ false-positive thật so với
   một `HashSet` làm ground truth và xác nhận nó khớp công thức.
2. Dùng nó làm doorkeeper đứng trước frequency sketch
   `13-algorithms/tinylfu.md` của bạn (`labs/10-cache`) và xác nhận các
   key one-hit-wonder không bao giờ chạm tới sketch.
3. Xây một bước kiểm tra sơ bộ đứng trước một IP blocklist
   (`labs/12-waf` hoặc `labs/11-rate-limit`) và đo tỉ lệ traffic được cho
   phép mà Bloom filter giải quyết được mà không cần chạm vào danh sách
   thật.
4. Cố tình insert vượt xa `n` đã size và đo lại tỉ lệ false-positive để
   thấy sự suy giảm mà công thức sizing đã dự đoán.
