# AST

Cái cây mà một parser tạo ra, và câu hỏi liệu bạn có cần nó hay không.
Nối tiếp [`15-parser/02-parser.md`](02-parser.md).

## What to learn

### AST là gì, và nó cố tình bỏ qua điều gì
Một Abstract Syntax Tree biểu diễn *cấu trúc* của input đã parse, đã lột
bỏ cú pháp không mang ý nghĩa: dấu ngoặc, dấu phẩy, whitespace, cách viết
chính xác của một keyword. `(a + b)` và `a + b` tạo ra cùng một AST vì dấu
ngoặc chỉ dẫn hướng cho việc parse; cái cây đã mã hóa sẵn việc gom nhóm
đó.

```rust
enum Expr {
    Number(u64),
    Ident(String),
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
}
```

Gotcha: `Box<Expr>` đệ quy là không thể tránh khỏi với một cây có độ sâu
không biết trước, nhưng nó nghĩa là mỗi node là một allocation heap riêng.
Với một config chỉ parse một lần lúc khởi động, điều này không quan trọng;
với bất cứ thứ gì được parse theo từng request, đó là lý do để cân nhắc
biểu diễn dạng arena bên dưới.

### Parse thẳng vào type của bạn, hay xây một cây trước?
Quyết định trung tâm. Có hai câu trả lời hợp lệ:

- **Parse trực tiếp vào struct đích.** Các hành động của parser xây dựng
  `Config`/`Request` của bạn khi nó chạy; không có cây trung gian nào cả.
  Đây là những gì [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) làm — một HTTP request đã
  parse *chính là* cấu trúc hữu ích, và chèn thêm một AST chỉ là overhead
  thuần túy. Chọn cách này khi dạng đã parse chính là dạng bạn dùng.
- **Xây một AST trước, rồi xử lý nó.** Chọn cách này khi cùng một input đã
  parse nuôi *nhiều* consumer, hoặc cần nhiều pass: validate, rồi resolve
  reference, rồi lower xuống dạng runtime. Một config hỗ trợ directive
  `include`, nội suy biến, hay default kế thừa từ block cha thì sạch hơn
  nhiều khi là một cây bạn duyệt qua (xem [`15-parser/04-visitor.md`](04-visitor.md)) so
  với thứ được ráp trong một pass.

Failure mode là xây một AST theo phản xạ chỉ vì tutorial làm vậy. Nếu chỉ
có đúng một consumer và một pass, AST là một lớp gián tiếp không mang lại
gì.

### Cây dạng arena: hình dạng idiomatic trong Rust
Một cây gồm các node `Box` với con trỏ tới cha đánh nhau với borrow
checker ([`03-rust/01-ownership.md`](../03-rust/01-ownership.md)) và làm fragmentation heap. Câu trả lời
idiomatic, giống hệt mẹo trong [`13-algorithms/lru.md`](../13-algorithms/lru.md), là lưu mọi node
trong một `Vec` duy nhất và liên kết chúng bằng index `usize`:

```rust
struct Ast { nodes: Vec<Node> }
struct Node { kind: NodeKind, children: Vec<u32> } // index, không phải Box
```

Cách này biến toàn bộ cây thành một allocation duy nhất, thân thiện với
cache khi duyệt, `Clone` được tầm thường, và không vướng vào các câu đố về
lifetime. Nhược điểm: index không được type-check theo cách reference
được, nên một index cũ (stale) là một bug logic mà compiler sẽ không bắt
được — hãy ràng buộc vòng đời của chúng theo vòng đời của arena.

### Span: giữ vị trí source trên mỗi node
Gắn byte range mà mỗi node đến từ đó. Các lỗi ngữ nghĩa phát hiện *sau*
khi parse ("upstream `web` được tham chiếu ở đây chưa bao giờ được định
nghĩa") khi đó có thể trỏ tới đúng dòng, giống như lỗi của lexer
([`15-parser/01-lexer.md`](01-lexer.md)). Một cây không có span buộc mọi lỗi về sau chỉ
có thể nói "đâu đó trong config của bạn."

## Practice
1. Với grammar config từ [`15-parser/02-parser.md`](02-parser.md), quyết định tường minh
   xem có parse thẳng vào struct `Config` của bạn hay qua một AST — ghi
   lại lý do. Nếu format không có include hay nội suy, câu trả lời thành
   thật thường là "không cần AST."
2. Thêm một tính năng buộc phải có cây: `include "other.conf"` hoặc nội
   suy `$var`. Giờ xây AST và quan sát vì sao một lần parse duy nhất
   không thể xử lý sạch sẽ điều này.
3. Biểu diễn AST đó dưới dạng arena (`Vec` + index `u32`), không phải
   node `Box`, và duyệt nó để xác nhận không có ma sát về lifetime.
4. Gắn một source span cho mỗi node và tạo ra một lỗi ngữ nghĩa sau-khi-
   parse ("directive `listen` trùng lặp") trỏ đúng vào dòng gây lỗi.
5. Đối chiếu điều này với [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md): diễn giải vì sao
   một HTTP request được parse trực tiếp vào một struct mà không có AST,
   và điều gì phải thay đổi về bài toán để một cái cây trở nên đáng giá.
