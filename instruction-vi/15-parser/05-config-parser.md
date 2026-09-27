# Config Parser

Áp dụng lexer → parser → AST → visitor vào config format của chính một
proxy. File này nối [`15-parser/01-lexer.md`](01-lexer.md) đến [`15-parser/04-visitor.md`](04-visitor.md)
với mối quan tâm về runtime trong [`09-architecture/03-config.md`](../09-architecture/03-config.md).

## What to learn

### Quyết định đầu tiên: bạn có thực sự cần viết một parser không?
Với phần lớn proxy, câu trả lời là không — bạn định nghĩa config như các
struct `serde` và để `toml`/`yaml`/`json` lo việc parse. `serde` cho bạn
parsing, deserialization có type-check, và error message đủ tốt miễn phí,
và [`09-architecture/03-config.md`](../09-architecture/03-config.md) giả định chính xác điều này. Tự viết
một config parser chỉ hợp lý khi bạn cần thứ gì đó `serde` không thể diễn
đạt: cú pháp directive tùy chỉnh (block `location` của nginx), include,
nội suy biến, hay các section điều kiện. Viết parser vì *format* đòi hỏi
điều đó, không phải để luyện tập parsing.

```rust
#[derive(Deserialize)]           // đây là mặc định — không cần parser nào cả
struct Config {
    listen: SocketAddr,
    upstreams: HashMap<String, Upstream>,
    routes: Vec<Route>,
}
```

### Nếu bạn tự viết: pipeline
Các giai đoạn ghép lại đúng như các file trước mô tả: byte → lexer
([`15-parser/01-lexer.md`](01-lexer.md)) → recursive-descent parser
([`15-parser/02-parser.md`](02-parser.md)) → AST nếu format có include/nội suy
([`15-parser/03-ast.md`](03-ast.md)) → visitor validation và lowering
([`15-parser/04-visitor.md`](04-visitor.md)) → một `Config` runtime bất biến. Kiểu output
mới là toàn bộ vấn đề: mọi thứ ở thượng nguồn tồn tại để tạo ra một struct
đã validate, bất biến duy nhất mà proxy có thể hoán đổi vào.

### Validation là một giai đoạn tách biệt khỏi parsing
Parsing trả lời "cái này có well-formed không?"; validation trả lời "cái
này có *coherent* không?" — mọi `route` có gọi tên một `upstream` tồn tại
không, port của `listen` có nằm trong khoảng hợp lệ không, có tên server
nào trùng lặp không. Giữ chúng tách biệt: một lỗi parse là một lỗi cú
pháp, một lỗi validation là một lỗi ngữ nghĩa, và trộn lẫn chúng tạo ra
message gây bối rối. Cả hai đều phải mang theo source span
([`15-parser/03-ast.md`](03-ast.md)) để người vận hành thấy đúng dòng.

### Tính atomic: ràng buộc reload dẫn dắt thiết kế
[`09-architecture/03-config.md`](../09-architecture/03-config.md) yêu cầu một lần reload tệ không bao giờ làm
sập proxy — config đang chạy tiếp tục phục vụ trong khi cái mới bị từ
chối. Điều đó nghĩa là *toàn bộ* pipeline, từ parse qua validation, phải
hoàn tất và tạo ra một `Config` runtime được xây dựng đầy đủ trước khi bất
cứ thứ gì đi vào hoạt động. Không bao giờ mutate config đang sống một
cách dần dần trong khi parse. Xây cái mới hoàn toàn ở một chỗ riêng; chỉ
hoán đổi con trỏ khi thành công.

```rust
fn reload(text: &str, live: &ArcSwap<Config>) -> Result<(), Vec<ConfigError>> {
    let next = parse_and_validate(text)?; // tất cả hoặc không gì cả
    live.store(Arc::new(next));           // hoán đổi atomic, chỉ khi thành công
    Ok(())
}
```

Gotcha: gom *toàn bộ* lỗi trước khi return (phục hồi lỗi từ
[`15-parser/02-parser.md`](02-parser.md), validation nhiều lỗi từ [`15-parser/04-visitor.md`](04-visitor.md)).
Một người vận hành reload một config 500 dòng muốn thấy mọi vấn đề cùng
lúc, không phải một vòng sửa-một-chạy-lại — nhưng lần reload đó, xét tổng
thể, vẫn bị từ chối một cách atomic.

### Input dù ít đáng ngờ hơn vẫn cần giới hạn
Một config file đáng tin cậy hơn một network request, nhưng một file bị
lỗi hoặc độc hại vẫn nên fail một cách nhẹ nhàng, không làm crash thread
reload. Giới hạn độ sâu nesting từ [`15-parser/02-parser.md`](02-parser.md) cũng áp dụng
ở đây — một config với cả triệu block lồng nhau phải báo lỗi, không được
overflow stack và abort process giữa chừng reload.

## Practice
1. Định nghĩa config của proxy bạn dưới dạng struct `serde` trước và
   deserialize từ TOML — đây là baseline của [`labs/13-hot-reload`](../../labs/13-hot-reload). Chỉ
   tiến tới một parser tự viết nếu bạn thêm một tính năng `serde` không
   thể diễn đạt.
2. Nếu bạn tự viết: ráp toàn bộ pipeline từ [`15-parser/01-lexer.md`](01-lexer.md) đến
   [`04-visitor.md`](04-visitor.md) tạo ra một `Config` runtime bất biến, và giữ lỗi parse
   và lỗi validation là các kiểu tách biệt.
3. Implement reload atomic với `arc-swap`: xây config mới hoàn toàn trước
   khi hoán đổi, và chứng minh bằng một test rằng một config fail
   validation để config đang sống nguyên vẹn và vẫn đang phục vụ.
4. Làm cho reload báo cáo mọi lỗi cùng lúc: đưa vào một config có một lỗi
   cú pháp, một tham chiếu upstream chưa định nghĩa, và một directive
   trùng lặp, xác nhận cả ba đều quay lại trong một lần gọi, mỗi cái kèm
   theo dòng của nó.
5. Đưa vào một config lồng nhau bệnh hoạn và xác nhận thread reload trả
   về một lỗi thay vì abort process — rồi nối reload này vào [`proxy`](../../proxy) và
   chạy nó dưới traffic của [`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md) để xác nhận
   request đang xử lý dở không bao giờ bị rớt bởi một lần reload.
