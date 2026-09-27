# Object Pools

`14-memory/02-arena.md` giải phóng toàn bộ allocation của một request cùng
lúc nhưng bắt đầu lại từ đầu ở request sau. Object pool dành cho pattern
ngược lại: *cùng một loại* object, được tái sử dụng qua nhiều request, nên
không bao giờ thực sự bị free — chỉ được checkout và trả lại.

## What to learn

### Checkout/return, thường qua RAII
Một pool giữ một tập object đã được dựng sẵn (connection struct, I/O
buffer) trong một free list; bên gọi checkout một cái, dùng nó, rồi trả
lại khi xong. Trả lại thủ công rất dễ quên trên một error path, nên phiên
bản idiomatic trong Rust bọc việc checkout trong một guard mà `Drop` của
nó tự động trả object về pool:

```rust
struct PooledBuffer<'a> {
    buf: Vec<u8>,
    pool: &'a Pool,
}
impl Drop for PooledBuffer<'_> {
    fn drop(&mut self) {
        self.buf.clear();                 // reset trước khi trả lại
        self.pool.push(std::mem::take(&mut self.buf));
    }
}
```
`Drop` impl chính là thứ làm cho cách này an toàn khi có early return hoặc
panic — object quay lại pool (hoặc thực sự bị drop, nếu pool đã biến mất)
bất kể phạm vi của việc checkout kết thúc thế nào.

### Object pool vs arena: tái sử dụng qua nhiều request vs free hàng loạt trong một
Điểm khác biệt quan trọng: nội dung của một arena đều "chết" cùng request
đã cấp phát chúng, và bản thân arena thường sống ngắn (một arena mỗi
request, hoặc tái sử dụng sau khi reset toàn bộ). Object của một pool sống
lâu hơn bất kỳ request đơn lẻ nào — một connection struct hay buffer được
checkout, dùng cho một request, trả lại, rồi được checkout lại bởi một
request hoàn toàn khác sau đó. Dùng pool khi *danh tính* của object cần
tồn tại và được tái sử dụng (một `Vec<u8>` mà bạn muốn giữ lại phần bộ nhớ
backing của nó); dùng arena khi bạn chỉ muốn nhiều allocation nhỏ chết
cùng nhau.

### Gotcha: reset state trước khi tái sử dụng, mọi lần
Toàn bộ lợi ích của pool — bỏ qua alloc/free — trở thành một bug về tính
đúng đắn và có thể là bảo mật nếu một object được trả lại không được reset
trước lần checkout tiếp theo. Một buffer được trả lại với length hoặc byte
cũ từ dữ liệu của request trước, rồi bị ghi đè một phần bởi một request
tiếp theo ngắn hơn, có thể làm rò rỉ byte của caller trước vào response
của caller hiện tại nếu code tin vào length cũ của buffer thay vì độ dài
thực tế của lần ghi mới. Reset (clear, zero, hoặc ít nhất truncate về độ
dài thực của nội dung mới) trong `Drop` impl hoặc tại thời điểm checkout —
chọn một chỗ, và làm cho việc bỏ qua nó là không thể.

### Gotcha: một pool không giới hạn là một memory leak có hình dạng pool
Một pool chỉ tăng trưởng (checkout nhiều hơn trả lại, hoặc tăng để phục vụ
một đợt tăng traffic rồi không bao giờ co lại) đạt tới cùng mức đỉnh không
kiểm soát mà `06-fragmentation.md` mô tả cho chính allocator — chỉ khác là
giờ code của bạn đang giữ bộ nhớ thay vì allocator. Giới hạn kích thước tối
đa của pool, và khi đầy, hoặc block việc checkout, hoặc fallback về một
allocation thật, hoặc từ chối — quyết định cái nào một cách chủ động, thay
vì mặc định tăng trưởng vô hạn.

## Practice
1. Implement một pool được bảo vệ bằng RAII gồm các buffer I/O `Vec<u8>`
   tái sử dụng cho đường copy theo từng connection của `labs/05-reverse-proxy`.
2. Thêm một test rằng một buffer, sau khi được trả lại pool và checkout
   lại, không bao giờ chứa byte từ lần dùng trước đó.
3. Benchmark alloc-mỗi-request so với buffer đã pool dưới tải đồng thời;
   đo cả throughput lẫn áp lực lên allocator (số lượng allocation).
4. Giới hạn kích thước pool và thêm một load test đẩy nhiều checkout đồng
   thời hơn giới hạn; xác nhận hành vi overflow bạn chọn (block, fallback
   về allocation mới, hoặc từ chối) thực sự xảy ra.
