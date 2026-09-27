# Visitor

Duyệt và biến đổi một AST mà không rải logic duyệt cây khắp codebase của
bạn. Nối tiếp [`15-parser/03-ast.md`](03-ast.md).

## What to learn

### Vấn đề mà visitor pattern giải quyết
Một khi bạn có một cây ([`15-parser/03-ast.md`](03-ast.md)), bạn sẽ duyệt nó nhiều hơn
một lần: để validate, để resolve reference, để lower nó xuống một config
runtime, có thể để pretty-print nó ngược lại. Viết lại logic "đệ quy vào
mọi con" mới toanh ở mỗi lần đó là nơi bug sinh sôi — một pass quên đi
xuống một block lồng và âm thầm bỏ qua nửa config. Một visitor tách riêng
*việc duyệt* ra một lần, để mỗi pass chỉ cần cung cấp việc nó làm ở mỗi
node.

```rust
trait Visitor {
    fn visit_block(&mut self, b: &Block) { walk_block(self, b); }
    fn visit_entry(&mut self, e: &Entry) { walk_entry(self, e); }
}
// các hàm walk_* sở hữu phần đệ quy; method mặc định gọi chúng,
// nên một impl chỉ cần override những node nó quan tâm.
fn walk_block<V: Visitor + ?Sized>(v: &mut V, b: &Block) {
    for e in &b.entries { v.visit_entry(e); }
}
```

Gotcha: phần đệ quy phải nằm trong các hàm `walk_*` tự do, không phải
trong thân method của trait. Nếu một visitor override `visit_block` và
quên gọi `walk_block`, việc duyệt dừng lại ở đó. Tách `walk` riêng ra cho
phép một override vừa làm việc của nó *vừa* ủy quyền tiếp:
`fn visit_block(&mut self, b) { /* việc của tôi */ walk_block(self, b); }`.

### Duyệt shared vs mutable
Hai hình dạng, và chúng thực sự khác nhau trong Rust:

- **Duyệt `&self`** cho các pass chỉ đọc — validation, gom tên, tính một
  metric. Nhiều pass loại này thậm chí có thể chạy mà không xung đột.
- **Duyệt `&mut self`** cho biến đổi — constant folding, áp dụng default,
  viết lại. Ở đây quy tắc aliasing của Rust "cắn": bạn không thể giữ một
  mutable borrow của cả cây trong khi cũng mutate một node bên trong nó.
  Lối thoát thường thấy là dạng arena từ [`15-parser/03-ast.md`](03-ast.md) — duyệt
  theo index và index ngược lại vào `&mut nodes[i]`, để borrow chỉ diễn ra
  trên một node tại một thời điểm, không phải cả cây.

### Pass qua một `match` khổng lồ
Với một AST nhỏ, bạn không cần trait chút nào — một hàm đệ quy duy nhất
`fn lower(node) -> Runtime` với một `match` rõ ràng hơn cả bộ máy visitor.
Visitor xứng đáng với độ phức tạp của nó khi có *nhiều* loại node và
*nhiều* pass; dưới ngưỡng đó nó là over-engineering, cùng một loại quyết
định như "tôi có thực sự cần một AST không" trong [`15-parser/03-ast.md`](03-ast.md).

### Chỗ một proxy thực sự gặp điều này
Xử lý config là use case chân thật: parse ([`15-parser/02-parser.md`](02-parser.md)) →
AST ([`15-parser/03-ast.md`](03-ast.md)) → một validation visitor (mọi `upstream` được
`route` tham chiếu đều tồn tại, không có `listen` trùng lặp) → một
lowering visitor tạo ra config runtime bất biến mà proxy hoán đổi vào khi
hot reload ([`09-architecture/03-config.md`](../09-architecture/03-config.md)). Giữ validation và lowering
là các visitor tách biệt nghĩa là một config không hợp lệ bị từ chối
*toàn bộ*, trước khi bất kỳ phần nào của config mới đi vào hoạt động —
đây chính là tính atomic mà [`09-architecture/03-config.md`](../09-architecture/03-config.md) yêu cầu.

## Practice
1. Thêm một validation visitor chỉ đọc trên config AST từ
   [`15-parser/03-ast.md`](03-ast.md) kiểm tra cross-reference (mọi `upstream` của một
   `route` đều được định nghĩa) và báo cáo *toàn bộ* vi phạm kèm span,
   không chỉ cái đầu tiên.
2. Thêm một mutating visitor áp dụng default kế thừa (một block con không
   có `timeout` sẽ nhận của cha), dùng dạng arena/index để `&mut` borrow
   chỉ diễn ra theo từng node.
3. Viết một lowering visitor biến AST đã validate thành struct config
   runtime bất biến mà proxy sẽ hoán đổi vào — phần đuôi của
   [`labs/13-hot-reload`](../../labs/13-hot-reload).
4. Cố tình phá vỡ việc duyệt: override một `visit_*` mà không gọi
   `walk_*` của nó, đưa vào một config lồng nhau, và xác nhận pass âm
   thầm bỏ qua subtree đó — rồi sửa nó và ghi chú vì sao việc tách
   walk/visit ngăn được loại bug này.
5. Với một AST đồ chơi hai node, viết cùng phép lowering đó dưới dạng một
   `match` đệ quy thuần túy và so sánh; quyết định ở số lượng loại node và
   pass nào thì visitor thôi là over-engineering.
