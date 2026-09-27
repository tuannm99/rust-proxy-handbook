# Lexer

Biến một stream byte phẳng thành một stream token có ý nghĩa — giai đoạn
trước khi bất kỳ cấu trúc nào tồn tại. [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) làm
việc này inline cho HTTP; file này là lý thuyết tái sử dụng bên dưới, áp
dụng được y hệt cho config parser trong [`09-architecture/03-config.md`](../09-architecture/03-config.md).

## What to learn

### Token là gì, và vì sao tách ra lại có ích
Một lexer (scanner, tokenizer) gộp các chuỗi byte thô thành một tập nhỏ
các token có kiểu: byte `Host:` trở thành một token `HeaderName("Host")`,
`{` trở thành `LBrace`, `3600` trở thành `Number(3600)`. Parser sau đó làm
việc trên token thay vì ký tự, nên nó không bao giờ phải nghĩ về
whitespace, ghép số từng chữ số, hay một từ kết thúc ở đâu.

```rust
enum Token {
    Ident(Range<usize>),   // một slice vào input, không phải String sở hữu
    Number(u64),
    LBrace,
    RBrace,
    Colon,
    Eof,
}
```

Gotcha: hãy để token mượn input (lưu byte range), không sở hữu bản copy.
Một config file hay HTTP message chỉ được parse một lần rồi bỏ đi; copy
mỗi identifier vào một `String` nhân đôi allocation mà không mang lại lợi
ích gì. Đây là cùng kỷ luật "byte, không phải String" như trong
[`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md).

### Vòng lặp cốt lõi: nhìn trước một byte
Phần lớn lexer là một vòng lặp duy nhất với một con trỏ cursor và một
`peek()` byte tiếp theo. Bạn phân loại theo byte hiện tại, rồi tiêu thụ
một chuỗi tối đa cùng loại. Đây là một DFA viết tay — mỗi nhánh `match` là
một state.

```rust
fn next_token(&mut self) -> Token {
    self.skip_whitespace();
    match self.peek() {
        None => Token::Eof,
        Some(b'{') => { self.bump(); Token::LBrace }
        Some(c) if c.is_ascii_digit() => self.lex_number(),
        Some(c) if is_ident_start(c) => self.lex_ident(),
        Some(c) => self.error_unexpected(c),
    }
}
```

Gotcha: quyết định trước xem whitespace và comment sẽ bị *bỏ qua* hay
*phát ra thành token*. Một config format thường bỏ qua chúng; một
formatter hay linter lại cần giữ lại. Sửa lại quyết định này sau nghĩa là
phải đụng vào mọi call site, nên hãy chọn một cách chủ động.

### Maximal munch và các nhập nhằng của nó
Quy tắc "tiêu thụ token hợp lệ dài nhất" (maximal munch) là thứ khiến
`3600ms` được lex thành `Number(3600)` rồi `Ident("ms")` thay vì báo lỗi
ngay tại `m`. Nó cũng tạo ra những cái bẫy kinh điển: `>=` phải là một
token duy nhất, nên khi thấy `>` bạn phải peek xem có `=` không trước khi
commit. Làm sai chỗ này biến `a>=b` thành `>` `=` và một lỗi cú pháp.

### Theo dõi vị trí không phải là tùy chọn
Một lexer báo "unexpected `}`" mà không có dòng và cột thì vô dụng trên
một config file thật. Luôn theo dõi byte offset, và dòng/cột nếu lỗi hiển
thị cho người dùng. Phiên bản rẻ: mang offset hiện tại trong range của
token và chỉ tính dòng/cột một cách lazy khi thực sự có lỗi được raise —
đừng trả giá cho việc theo dõi vị trí trên mỗi byte ở happy path.

### Ranh giới giữa lexing và parsing đôi khi mờ nhạt
Không phải format nào cũng có một ranh giới lexer/parser rõ ràng. Dòng
request của HTTP/1.1 đơn giản đến mức [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) scan
byte trực tiếp mà không cần kiểu token nào — đưa một lexer vào đó chỉ thêm
nghi thức mà không mang lại giá trị. Việc tách ra chỉ đáng giá khi grammar
có nesting và precedence thực sự (một config language, một expression
language), nơi một token stream thực sự đơn giản hóa parser. Chỉ dùng một
lexer riêng khi parser nếu không sẽ bị rối với whitespace và phân loại ký
tự, không phải trước đó.

## Practice
1. Viết một lexer cho một grammar config nhỏ — dòng `key value;`, block
   `{}`, comment `#`, số với hậu tố đơn vị tùy chọn (`10s`, `4k`) — phát ra
   token dạng borrowed-range. Đây là nửa đầu của phần config trong
   [`labs/13-hot-reload`](../../labs/13-hot-reload).
2. Xử lý đúng maximal munch cho một operator hai ký tự (`>=` hoặc `//`):
   viết test fail trước (`a>=b` phải lex thành ba token), rồi mới viết
   logic peek để nó pass.
3. Thêm theo dõi byte-offset và biến một lỗi lexer thành một message có
   dòng và cột, tính lazy từ offset thay vì theo dõi từng byte.
4. Benchmark token dạng borrowed-range so với một phiên bản cấp phát một
   `String` mỗi identifier trên một config lớn; xác nhận khác biệt về số
   lượng allocation.
5. So sánh scanner viết tay của bạn với việc scan HTTP inline của
   [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) và diễn giải vì sao HTTP *không* dùng một
   kiểu token riêng — khi nào việc tách ra có ích và khi nào nó chỉ là
   nghi thức.
