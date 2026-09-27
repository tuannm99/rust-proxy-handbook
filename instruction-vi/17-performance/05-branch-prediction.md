# Branch Prediction

Vì sao một branch khó đoán trên hot path tốn kém hơn nhiều so với vẻ
ngoài của một phép so sánh đơn thuần. Giống phần còn lại của
[`17-performance/`](.), chỉ theo đuổi điều này sau khi một profile chỉ vào nó.

## What to learn

### Cơ chế
CPU hiện đại pipeline sâu khoảng ~15–20 instruction và bắt đầu thực thi
qua một branch *trước khi* biết nó sẽ đi hướng nào, bằng cách đoán hướng
đi. Một dự đoán đúng gần như miễn phí. Một dự đoán sai vứt bỏ toàn bộ công
việc suy đoán và nạp lại pipeline — một stall khoảng ~15–20 chu kỳ. Một
branch mà bộ dự đoán đoán đúng 99% thời gian gần như miễn phí; một branch
đoán đúng 50% thời gian (một cú tung đồng xu trên dữ liệu) là một trong
những thứ tốn kém nhất mà một vòng lặp nóng có thể chứa.

### Có thể đoán trước vs không thể đoán trước
Bộ dự đoán rất giỏi với các pattern: luôn-đúng, luôn-sai, xen kẽ, cạnh lùi
của vòng lặp. Nó bất lực trước các branch đi theo *dữ liệu* khi dữ liệu đó
ngẫu nhiên. Một bounds check không bao giờ fail, một error path gần như
không bao giờ kích hoạt — rẻ, vì hướng đi nhất quán. Một branch dựa trên
việc một byte ngẫu nhiên có `>` một ngưỡng hay không — đắt, vì không có
pattern nào để học.

```rust
// Sắp xếp input trước làm branch này có thể đoán trước và vòng lặp
// nhanh hơn nhiều lần — cùng instruction, cùng dữ liệu, thứ tự khác.
for &b in &data {
    if b >= threshold { sum += b as u64; }  // dữ liệu ngẫu nhiên => ~50% đoán sai
}
```

### Chỗ một proxy gặp phải nó
Parse từng byte một với một branch mỗi ký tự ([`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md))
là hot spot chân thật — một branch quyết định "đây có phải delimiter
không" chạy hàng triệu lần. Kiểm tra policy theo từng request (rule WAF
[`07-security/06-waf.md`](../07-security/06-waf.md), IP filtering [`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md))
là các branch trên dữ liệu request. Thường thì chúng *có thể đoán trước*
(hầu hết traffic được cho phép, hầu hết byte không phải delimiter), đó là
lý do vì sao chúng rẻ trong thực tế — nguy hiểm là một branch thực sự chia
50/50 trên dữ liệu nóng.

### Làm cho branch biến mất
Khi một branch vốn dĩ khó đoán, cách sửa nhanh nhất thường là loại bỏ nó —
tính cả hai bên mà không cần branch:

- **Branchless select:** `sum += (b >= threshold) as u64 * b as u64;`
  biến điều kiện thành phép toán số học mà CPU không bao giờ đoán sai.
- **Table lookup:** thay một chuỗi `if`/`match` trên một byte bằng một
  bảng tra cứu 256 mục có index (một bảng phân loại byte), mẹo đứng sau
  các bộ scan header HTTP nhanh.
- **SIMD:** xử lý 16–32 byte cùng lúc mà không cần branch theo từng byte
  nào cả — xem [`17-performance/06-simd.md`](06-simd.md), nơi việc scan header cuối
  cùng sẽ dẫn tới.

Gotcha: code branchless không tự động nhanh hơn. Loại bỏ một branch *có
thể đoán trước* chỉ thêm phép toán mà bộ dự đoán đã ẩn đi miễn phí, và có
thể chậm hơn. Branchless thắng cụ thể khi branch đó khó đoán. Đây là quy
tắc của cả thư mục này thu nhỏ lại: đo tỷ lệ đoán sai trước.

### Gợi ý likely/unlikely
Với một branch bạn *biết* là lệch hẳn về một phía (một error path gần như
không bao giờ được chạy), bạn có thể gợi ý cho compiler đặt nhánh lạnh ra
ngoài dòng, giữ instruction của hot path dày đặc trong cache. Trong Rust
đây là `likely`/`unlikely` của `core::hint` (hoặc `cold_path`) ở nơi đã
stable, và `#[cold]` trên hàm hiếm khi được gọi. Nó giúp cho layout code
nhiều hơn là bản thân việc dự đoán, và chỉ trên các branch thực sự lệch
hẳn.

## Practice
1. Tái tạo hiệu ứng đã-sắp-xếp-vs-chưa-sắp-xếp: cộng các phần tử trên một
   ngưỡng qua dữ liệu ngẫu nhiên, rồi qua dữ liệu đã sắp xếp, và đo khác
   biệt chỉ từ việc dự đoán. Đọc `perf stat branch-misses` cho cả hai.
2. Viết lại vòng lặp đó theo kiểu branchless (nhân `as u64`) và so sánh —
   xác nhận nó thắng trường hợp *chưa sắp xếp* nhưng kiểm tra xem nó có
   thắng trường hợp *đã sắp xếp* (có thể đoán trước) không.
3. Thay một bộ phân loại byte kiểu `if`/`match` trong một vòng lặp kiểu
   HTTP-parser ([`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md)) bằng một bảng tra cứu 256
   mục và đo.
4. Đo một branch WAF/IP-filter ([`07-security/06-waf.md`](../07-security/06-waf.md)) dưới traffic
   thực tế phần lớn được cho phép và xác nhận nó *có thể đoán trước* và
   do đó rẻ — luyện tập việc nhận ra một branch không đáng để đụng vào.
5. Thêm `#[cold]` vào một error path thật trong [`proxy`](../../proxy), kiểm tra xem code
   của hot path có dày đặc hơn không, và xác nhận bằng một benchmark rằng
   bạn không làm mọi thứ tệ hơn — rồi quyết định xem bước tiếp theo có
   phải là [`17-performance/06-simd.md`](06-simd.md) không.
