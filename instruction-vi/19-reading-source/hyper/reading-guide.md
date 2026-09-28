# Reading guide: hyper

Một lộ trình đi qua phần implementation HTTP/1 của hyper, kèm các câu hỏi
cần trả lời trên đường đi. Đây là guide, không phải notes: các file theo
project liệt kê trong [`00-README.md`](00-README.md) là để bạn tự viết. Câu trả lời không có
ở đây — so các lựa chọn của hyper với những lựa chọn bạn đã đưa ra trong
[`labs/01-http-parser`](../../../labs/01-http-parser) chính là bài tập.

Repository: `github.com/hyperium/hyper` (1.x), thư mục `src/`. hyper giao
việc tách token header thô cho crate riêng `httparse`
(`github.com/seanmonstar/httparse`), bạn cũng sẽ ghé qua. Đường dẫn khớp
với version tại thời điểm viết; nếu một đường dẫn đã chuyển chỗ, hãy
search tên type được nêu.

## Khi nào đọc phần nào

| Đọc | Sau khi | Vì sao lúc đó |
| --- | --- | --- |
| Điểm dừng 1-3: parsing và framing | [`labs/01-http-parser`](../../../labs/01-http-parser), bước 8 Practice của [`05-http-stack/01-parser.md`](../../05-http-stack/01-parser.md) | Mỗi khác biệt so với parser của bạn là một case bạn bỏ sót hoặc chọn khác |
| Điểm dừng 4: state machine của connection | [`labs/02-http-server`](../../../labs/02-http-server), [`05-http-stack/04-keepalive.md`](../../05-http-stack/04-keepalive.md) | Bạn đã tự debug keep-alive |
| Điểm dừng 5: dispatcher | [`03-rust/18-async-traits.md`](../../03-rust/18-async-traits.md), [`labs/02-http-server`](../../../labs/02-http-server) | Bạn đã implement một `Service` |

## Lộ trình

### Điểm dừng 1: tách token header trong `httparse`
Đọc `Request::parse` của `httparse` và phần parse header mà nó gọi.
- Nó báo "cần thêm byte" khác với "cái này sai định dạng" thế nào, và so với kiểu trả về trong parser của bạn ra sao?
- Nó dùng SIMD ở đâu, và cho phần nào của request?
- Nó nhận một mảng header do caller cung cấp thay vì tự allocate. Điều đó buộc caller phải quyết định gì từ đầu, và chuyện gì xảy ra nếu request có nhiều header hơn sức chứa của mảng?

### Điểm dừng 2: biến token thành request: `proto/h1/role.rs`
Hàm parse phía server xây request head từ output của `httparse`.
- Tìm chính xác chỗ `Content-Length` và `Transfer-Encoding` cùng xuất hiện. hyper làm gì, và nó có khớp với [`07-security/05-request-smuggling.md`](../../07-security/05-request-smuggling.md) không?
- Nó làm gì với nhiều header `Content-Length` trùng giá trị? Khác giá trị?
- Giới hạn kích thước header được enforce ở đâu, và client thấy lỗi gì?

### Điểm dừng 3: framing body: `proto/h1/decode.rs` và `encode.rs`
Body decoder có một số ít kiểu (độ dài cố định, chunked, đọc tới EOF).
- Đi qua state machine của chunked decoder. Có những state nào, và mỗi state từ chối những input sai định dạng nào?
- Chunk extension và trailer được xử lý thế nào — chấp nhận, bỏ qua, hay từ chối?
- Khi nào framing "đọc tới EOF" được chọn, và vì sao nó chỉ hợp lệ trong một số trường hợp?

### Điểm dừng 4: connection: `proto/h1/conn.rs` và `proto/h1/io.rs`
`conn.rs` theo dõi trạng thái đọc, ghi và keep-alive; `io.rs` quản lý buffer.
- Điều gì quyết định một connection có được dùng lại sau một response không? Liệt kê mọi điều kiện bạn tìm được.
- Read buffer lớn lên thế nào, tối đa bao nhiêu, và chuyện gì xảy ra khi một request vượt quá nó?
- Chuyện gì xảy ra nếu client pipeline request thứ hai trước khi response đầu tiên được ghi?

### Điểm dừng 5: dispatcher: `proto/h1/dispatch.rs`
Đây là nơi `Service` của bạn được gọi.
- Dispatcher đan xen việc đọc request kế tiếp, poll future của service, và ghi response thế nào?
- Backpressure được áp ở đâu khi client đọc body response chậm ([`01-network/11-http2.md`](../../01-network/11-http2.md) bàn phiên bản HTTP/2 của vấn đề này)?

## Viết gì vào notes
`interesting-code.md` nên liệt kê, cho mỗi điểm dừng, một quyết định hyper
đưa ra khác với [`labs/01-http-parser`](../../../labs/01-http-parser) của bạn và bây giờ bạn nghĩ cái nào
đúng. `what-to-learn.md` nên ánh xạ những gì bạn tìm được về
[`05-http-stack/01-parser.md`](../../05-http-stack/01-parser.md), [`05-http-stack/04-keepalive.md`](../../05-http-stack/04-keepalive.md), và
[`07-security/05-request-smuggling.md`](../../07-security/05-request-smuggling.md).
