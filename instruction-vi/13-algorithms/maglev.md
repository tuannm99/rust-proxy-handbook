# Maglev Hashing

Sơ đồ hashing của load balancer Google: lookup O(1), cân bằng gần như
hoàn hảo, gián đoạn tối thiểu. Lựa chọn khi ring của consistent hashing
(`13-algorithms/consistent-hash.md`) quá chậm hoặc quá lệch.

## What to learn

### Nó tối ưu cho điều gì
Maglev tính trước một **bảng lookup** kích thước cố định M (một số nguyên
tố, ví dụ 65537), nơi mỗi slot giữ một chỉ số upstream. Lookup khi đó là:

```rust
fn pick(table: &[usize], key_hash: u64) -> usize {
    table[(key_hash % table.len() as u64) as usize]
}
```

Một phép modulo và một lần index mảng — O(1), thân thiện với cache, không
đi qua cây, không hash theo từng upstream. Mọi công việc dồn vào việc xây
bảng, chỉ xảy ra khi membership của pool thay đổi.

Sự đánh đổi so với consistent hashing là rõ ràng: Maglev chấp nhận một tỉ
lệ remap *cao hơn một chút* khi membership thay đổi để đổi lấy cân bằng
gần hoàn hảo và lookup thời gian hằng số.

### Xây bảng: các permutation
Mỗi upstream sinh ra một **danh sách ưu tiên** — một permutation của tất
cả M slot, mô tả thứ tự nó muốn nhận chúng. Permutation được suy ra từ hai
hash độc lập của tên upstream:

```
offset = h1(upstream) % M
skip   = (h2(upstream) % (M - 1)) + 1
permutation[j] = (offset + j * skip) % M
```

Vì M là số nguyên tố và `skip` nằm trong `1..M`, việc bước theo `skip`
thăm đúng mỗi slot một lần trước khi lặp lại — đó là điều khiến danh sách
ưu tiên của mỗi upstream là một permutation thật sự, và đó chính xác là
lý do M phải là số nguyên tố. Chọn M hợp số và `skip` chia hết chung với
M, và upstream chỉ có thể chạm tới một phần của bảng.

### Xây bảng: vòng lặp population
Điền bảng theo round-robin qua các upstream: mỗi upstream, lần lượt, đề
xuất slot ưu tiên tiếp theo của nó; nếu slot đó trống nó nhận lấy, ngược
lại nó đi tiếp qua danh sách ưu tiên của chính nó cho tới khi tìm được một
slot trống. Lặp lại cho tới khi tất cả M slot được điền.

Mỗi upstream cuối cùng sở hữu trong khoảng một slot so với M/N — đó là
sự cân bằng gần hoàn hảo. Weight được áp dụng bằng cách để một upstream
weight-w lấy w lượt mỗi vòng thay vì một.

Gotcha: bước "đi tiếp cho tới khi trống" là nơi một implementation ngây
thơ trở thành bậc hai. Mỗi upstream phải giữ con trỏ của riêng nó vào
danh sách ưu tiên qua các vòng; khởi động lại việc tìm kiếm từ đầu mỗi lần
biến một lần điền cỡ O(M log M) thành O(M²) và trở nên rõ ràng như một lần
đứng hình vài giây ở mỗi lần reload config.

### Gián đoạn khi membership thay đổi
Gỡ một upstream giải phóng các slot của nó, và việc rebuild phân phối lại
chúng — nhưng thứ tự population dịch chuyển một chút cho mọi người, nên tỉ
lệ remap hơi cao hơn con số lý thuyết 1/N (thường tệ hơn vài phần trăm).
Tăng M giảm điều này: M nên ít nhất gấp ~100 lần số upstream, đây là lý do
65537 là mặc định phổ biến cho các pool cỡ hàng trăm.

Gotcha: bảng chỉ tất định qua các instance proxy nếu mọi instance đồng ý
về *tập* upstream, *thứ tự* dùng trong round-robin population, và các hàm
hash. Sắp xếp các upstream theo một id ổn định trước khi xây, và cố định
hash — nếu không hai instance sẽ xây ra hai bảng khác nhau từ cùng một
config và affinity sẽ âm thầm hỏng giữa chúng.

### Khi nào không nên dùng nó
Cái giá của Maglev là việc rebuild: O(M) công việc và một bảng cỡ M cho
mỗi pool. Với một vài upstream đứng sau một proxy, một ring `BTreeMap` hay
HRW thuần (`13-algorithms/rendezvous-hash.md`) đơn giản hơn, rebuild ngay
lập tức, và lookup O(log N) hay O(N) không phải nút thắt cổ chai của bạn.
Maglev xứng đáng với độ phức tạp của nó ở hàng trăm-tới-hàng-nghìn
upstream và tốc độ request cao.

## Practice
1. Trong `labs/06-load-balancer`, implement việc sinh permutation cho
   M=65537 và assert rằng danh sách ưu tiên của một upstream thăm đúng
   tất cả M slot một lần — đây là test bắt được một M không phải số
   nguyên tố.
2. Implement vòng lặp population với một con trỏ riêng cho mỗi upstream;
   xây một bảng cho 10 upstream và xác nhận mỗi cái sở hữu trong khoảng
   ±1 slot so với M/10.
3. Đo thời gian xây bảng ở 10, 100, và 1000 upstream. Sau đó cố tình khởi
   động lại việc quét danh sách ưu tiên từ chỉ số 0 mỗi vòng và đo lại —
   xác nhận sự bùng nổ bậc hai.
4. Gỡ một upstream khỏi 10 cái, rebuild, và đo phần trăm số slot trong M
   đổi chủ. So sánh với tỉ lệ ~1/10 mà ring consistent-hash của bạn đạt
   được trong bài tập của `13-algorithms/consistent-hash.md`.
5. Xây cùng một bảng hai lần từ cùng tập upstream với thứ tự input bị xáo
   trộn; xác nhận các bảng chỉ giống hệt nhau khi bạn sắp xếp theo id ổn
   định trước.
6. Benchmark độ trễ lookup của Maglev so với ring so với HRW ở 1000
   upstream.
