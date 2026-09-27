# Ownership

## What to learn

### Move semantics
Mỗi giá trị trong Rust có đúng một owner. Gán hoặc truyền một giá trị
không phải `Copy` sẽ move nó — binding cũ trở nên không hợp lệ ngay tại
compile time, nên không tốn chi phí runtime và không có khả năng
use-after-free từ một alias cũ. Đây chính là thứ cho phép một proxy chuyền
một `Vec<u8>` read buffer qua nhiều lớp (parser -> router -> upstream
writer) mà không bao giờ phải copy nó.

```rust
let buf: Vec<u8> = read_request();
let parsed = parse(buf); // buf moved into parse; buf is no longer usable here
```

Gotcha: move một giá trị *ra khỏi* một field của struct trong khi một
borrow `&mut` của toàn bộ struct đang sống thì không compile được — đây
chính xác là hình dạng bạn gặp khi cố lấy ownership của read buffer của
một connection trong lúc vẫn đang giữ một mutable reference tới connection
đó để bookkeeping. Tách struct ra hoặc dùng `Option::take`.

### Copy vs Clone
Các type `Copy` (integer, bool, dữ liệu kích thước cố định nhỏ) được nhân
bản bitwise ngầm định khi gán — không có move nào xảy ra, cả hai binding
đều còn hợp lệ. `Clone` là một deep copy tường minh, có thể tốn kém, bạn
chủ động chọn dùng qua `.clone()`. Trên một hot request path, một
`.clone()` vô tình trên một header map nhiều kilobyte là một bug hiệu năng
thật sự, không chỉ là chuyện style — ưu tiên `Arc` (xem `03-rust/04-sync.md`)
hoặc borrowing thay vì clone buffer trên từng request.

```rust
#[derive(Clone, Copy)]
struct ConnId(u64); // rẻ, Copy là đúng đắn

struct Headers(Vec<(String, String)>); // KHÔNG Copy — clone là O(n) và có allocate
```

### Quy tắc borrowing
Tại bất kỳ thời điểm nào bạn chỉ được có một `&mut T` hoặc bất kỳ số lượng
`&T` nào, không bao giờ cả hai cùng lúc — được enforce tại compile time,
không tốn chi phí runtime. Đây là thứ khiến zero-copy parsing không cần
`unsafe` trở nên khả thi: một parser có thể trả ra các slice `&[u8]` trỏ
vào read buffer gốc thay vì allocate substring, vì borrow checker đảm bảo
buffer sống lâu hơn các slice đó.

```rust
fn parse_method(buf: &[u8]) -> &[u8] {
    &buf[..buf.iter().position(|&b| b == b' ').unwrap()]
}
```

Gotcha: borrow không thể được giữ qua một điểm `.await` nếu future đó
cũng cần là `Send` và dữ liệu được borrow nằm trên stack frame của caller
mà stack frame đó sẽ di chuyển — đây là nguyên nhân gốc của rất nhiều lỗi
"future cannot be sent between threads" khi trộn borrowed slice với các
async fn. Xem `03-rust/02-lifetimes.md` và `03-rust/05-async.md`.

### Drop order và RAII
Các giá trị được drop theo thứ tự khai báo ngược lại khi ra khỏi scope;
các field của struct drop theo thứ tự khai báo. Bọc một resource thô
(socket fd, vùng mmap, lock guard) trong một type có `Drop` giải phóng nó
nghĩa là "quên dọn dẹp" trở thành một việc không-thể-xảy-ra tại compile
time thay vì một memory leak tại runtime — đây chính là mẹo mà
`MutexGuard` và `TcpListener` dùng.

```rust
struct UpstreamConn {
    id: u64,
    // socket dropped automatically when UpstreamConn drops
}
```

Gotcha: `std::mem::forget` (hoặc một panic trong lúc unwind với
`catch_unwind`) bỏ qua `Drop` — điều này liên quan nếu bạn từng đưa một
raw fd cho code `libc` (xem `03-rust/03-unsafe.md`) và dựa vào `Drop` của
Rust để đóng nó.

## Practice
1. Viết một hàm nhận ownership của một `Vec<u8>` request buffer, parse ra
   một view method/path/headers dưới dạng borrowed slice, và trả về một
   struct giữ cả buffer lẫn các slice — để ý vì sao việc này cần một
   lifetime parameter (tiếp tục ở `03-rust/02-lifetimes.md`).
2. Chủ động gây ra rồi sửa một lỗi "use of moved value" bằng cách tái cấu
   trúc một hàm để borrow thay vì lấy ownership.
3. Benchmark (bằng một `std::time::Instant` nhanh) việc clone một header
   map 8KB 1 triệu lần so với bọc nó trong `Arc` rồi clone `Arc` — xác
   nhận sự khác biệt là có thật trước khi tin vào nó.
4. Trong `labs/00-tcp-server`, quyết định xem read buffer trên mỗi
   connection của bạn thuộc sở hữu của task hay được borrow từ một pool,
   và giải thích trong một comment.
5. Viết một type nhỏ với một `Drop` impl tự viết in ra khi nó chạy; xác
   nhận thứ tự qua các struct lồng nhau và `Vec<T>` khớp với dự đoán của
   bạn.
