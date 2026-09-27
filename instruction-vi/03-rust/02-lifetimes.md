# Lifetimes

## What to learn

### Lifetime elision và annotation tường minh
Lifetime là các nhãn chỉ tồn tại tại compile time mà borrow checker dùng
để chứng minh một reference không bao giờ sống lâu hơn thứ nó trỏ tới —
chúng không thêm bất kỳ biểu diễn nào tại runtime. Các quy tắc elision
bao phủ các trường hợp phổ biến (một input reference -> output borrow từ
nó; method `&self` -> output borrow từ `self`), nên phần lớn bạn chỉ viết
`'a` tường minh khi một hàm hoặc struct gắn kết *nhiều* borrow độc lập với
nhau.

```rust
struct RequestView<'a> {
    method: &'a [u8],
    path: &'a [u8],
}

fn view<'a>(buf: &'a [u8]) -> RequestView<'a> { RequestView { method: buf, path: buf } }
```

### Struct giữ borrowed data
Một struct có lifetime parameter không thể sống lâu hơn dữ liệu nó borrow
— compiler enforce điều này ở mọi nơi struct được dùng, đây chính xác là
cách một `RequestView<'a>` zero-copy (ở trên) được giữ an toàn: nó không
bao giờ có thể được trả về hoặc lưu trữ ở một nơi sống lâu hơn read buffer
gốc.

Gotcha: đây là lý do vì sao zero-copy parser và self-referential state
không đi chung được với nhau. Một struct không thể vừa giữ một buffer vừa
giữ một borrow vào chính buffer đó như hai sibling field — borrow đó sẽ
cần tham chiếu tới một field của cùng struct nó đang sống bên trong, điều
mà mô hình ownership của Rust cấm nếu không có một lớp indirection
(`Pin`, xem [`03-rust/06-pin.md`](06-pin.md), hoặc đơn giản là lưu một offset/
`Range<usize>` thay vì một `&[u8]`, đây là cách phần lớn zero-copy parser
trong production làm).

### Lifetime với async
Các điểm `.await` là nơi những rắc rối lớn về lifetime xuất hiện trong một
proxy. Một `async fn` được desugar thành một struct state machine phải
giữ mọi thứ còn sống qua một `.await` — bao gồm cả các borrow. Nếu struct
đó cũng cần là `'static` (đúng với bất cứ thứ gì bạn `tokio::spawn`), nó
không thể giữ một borrow của bất cứ thứ gì có vòng đời ngắn hơn chính cái
task đó.

```rust
async fn handle(buf: &[u8]) { /* ... */ } // fine to call and .await inline

// but you CANNOT do:
// tokio::spawn(handle(&local_buf)); // error: `local_buf` does not live long enough
```

Cách sửa gần như luôn là làm cho task được spawn sở hữu dữ liệu của nó
(`Vec<u8>`, `Bytes`, hoặc `Arc<T>`) thay vì borrow nó — xem
[`03-rust/04-sync.md`](04-sync.md) về `Arc`, và [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) về
`bytes::Bytes` (một owned buffer clone rẻ, cách sửa chuẩn cho đúng vấn đề
này trong hệ sinh thái hyper).

### HRTB và trait object (ngắn gọn)
`for<'a> Fn(&'a T) -> ...` (higher-ranked trait bound) xuất hiện khi bạn
lưu một closure hoặc trait object phải hoạt động với *bất kỳ* lifetime nào
mà caller chọn, chứ không phải một lifetime cố định — ví dụ một
middleware trait có method `handle` nhận `&Request` với một lifetime được
chọn theo từng lần gọi. Bạn không cần viết những thứ này thường xuyên,
nhưng hãy nhận ra cú pháp này khi một lỗi compiler nhắc tới
"higher-ranked lifetime error" trong lúc xây một hệ thống plugin/middleware
([`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md)).

## Practice
1. Lấy `RequestView<'a>` từ bài tập của 01-ownership.md và làm cho
   compiler từ chối một phiên bản code của bạn cố trả về một
   `RequestView` sau khi buffer nguồn của nó đã bị drop.
2. Tái cấu trúc một hàm borrow một buffer thành một hàm sở hữu
   `bytes::Bytes` thay vào đó, và giải thích trong một comment khi nào
   mỗi lựa chọn là đúng đắn cho một type request/response di chuyển qua
   [`labs/02-http-server`](../../labs/02-http-server).
3. Tái tạo lỗi "does not live long enough" từ một borrow đi qua
   `tokio::spawn`, rồi sửa nó theo ba cách khác nhau: clone thành một
   owned type, bọc trong `Arc`, và tái cấu trúc để tránh spawn hoàn toàn.
4. Viết một struct cố tình không compile được vì nó cố giữ một buffer và
   một slice `&[u8]` của chính nó như hai sibling field; đọc lỗi và liên
   hệ nó với [`03-rust/06-pin.md`](06-pin.md).
