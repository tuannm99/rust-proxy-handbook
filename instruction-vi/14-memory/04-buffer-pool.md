# Buffer Pools

[`14-memory/03-object-pool.md`](03-object-pool.md) nói về pooling object cùng hình dạng nói
chung. Buffer I/O cần được xử lý riêng vì kích thước của chúng thay đổi
rất nhiều (một chunk đọc 4 KB so với một TLS record 64 KB), điều này phá
vỡ một pool được thiết kế quanh một hình dạng object cố định.

## What to learn

### Pool theo size-class, không phải một kích thước cố định
Một pool duy nhất gồm, ví dụ, buffer 8 KB lãng phí phần lớn buffer trên
một lần đọc 64 byte nhỏ và hoàn toàn không thể phục vụ một lần đọc 64 KB.
Buffer pool thường được chia theo size-class giống cách một allocator
tổng quát làm ([`14-memory/01-allocator.md`](01-allocator.md)): một vài tier cố định (ví dụ 4
KB, 16 KB, 64 KB), và một lần checkout chọn tier nhỏ nhất vừa đủ cho
request, thay vì một pool cố phục vụ mọi kích thước.

### Pooling ở đúng mức type mà `bytes`/`hyper` đã dùng
`hyper` và cả hệ sinh thái tokio truyền dữ liệu I/O qua lại dưới dạng
`bytes::Bytes`/`BytesMut` — buffer byte đếm tham chiếu, có thể slice
zero-copy, không phải `Vec<u8>` thô. Pooling ở mức `Vec<u8>` rồi copy vào
một `BytesMut` để đưa cho hyper làm mất phần lớn ý nghĩa của việc pooling;
hãy pool trực tiếp allocation của `BytesMut` (hoặc dùng một crate như
`bytes-pool` / tự viết một pool theo size-class trên `BytesMut::with_capacity`)
để một buffer đã checkout ghép nối với phần còn lại của stack mà không cần
copy thêm.

### Chỗ pooling hoàn toàn không áp dụng: các đường zero-copy thật sự
`sendfile`/`splice` của [`02-linux/18-zerocopy.md`](../02-linux/18-zerocopy.md) chuyển dữ liệu từ page
cache thẳng tới socket mà không bao giờ đi vào một buffer ở userspace —
không có gì để pool trên đường đó, vì userspace không bao giờ giữ các
byte. Buffer pooling quan trọng cho các đường *có* copy qua userspace:
mã hóa/giải mã TLS record (kernel không thể giải mã hộ bạn), kiểm tra body
của WAF ([`07-security/06-waf.md`](../07-security/06-waf.md), phải đọc byte để kiểm tra chúng), và bất
kỳ phép biến đổi nào chạm vào payload. Hãy biết đường nào của proxy bạn
thuộc loại nào trước khi giả định buffer pool giúp ích cho nó.

### Gotcha: độ mịn của tier là một tham số tuning thật sự, không phải chi tiết
Quá ít tier lãng phí bộ nhớ (một request 5 KB làm tròn lên buffer 64 KB
nếu đó là tier tiếp theo); quá nhiều tier lại fragmentation chính pool thành
nhiều free list nhỏ, hiếm khi được tái sử dụng, làm mất luôn lợi ích của
việc pooling. Chọn size tier từ phân bố kích thước payload thực tế của
traffic của bạn (đo nó — [`08-observability/02-metrics.md`](../08-observability/02-metrics.md) — chứ đừng đoán),
giống cách size class của một allocator tổng quát được chọn từ histogram
kích thước allocation thực tế.

## Practice
1. Xây một buffer pool theo size-class (ví dụ tier 4 KB / 16 KB / 64 KB)
   trả về `BytesMut` cho đường copy request/response của [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy).
2. Đo phân bố kích thước payload của một traffic mix đại diện và xác nhận
   (hoặc điều chỉnh) các tier bạn chọn dựa trên nó.
3. So sánh áp lực lên allocator (số lượng allocation, RSS tăng theo
   phương pháp của [`14-memory/06-fragmentation.md`](06-fragmentation.md)) có và không có pool
   dưới tải đồng thời kéo dài.
4. Xác định đường I/O nào của [`proxy`](../../proxy) là zero-copy thật sự
   ([`02-linux/18-zerocopy.md`](../02-linux/18-zerocopy.md)) và xác nhận pooling không có tác dụng gì
   trên chúng, so với các đường copy qua userspace thì được hưởng lợi.
