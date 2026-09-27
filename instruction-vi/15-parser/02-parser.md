# Parser

Biến một token stream phẳng thành dữ liệu có cấu trúc. Đây là lý thuyết
tổng quát; `05-http-stack/01-parser.md` là ứng dụng riêng cho HTTP và
`labs/01-http-parser` là nơi bạn implement nó.

## What to learn

### Recursive descent: lựa chọn mặc định bạn nên dùng
Recursive descent viết một hàm cho mỗi rule của grammar, và việc lồng
nhau trong grammar trở thành việc lồng nhau trong call stack. Đây là kỹ
thuật đứng sau phần lớn parser production (kể cả `rustc` của chính Rust)
vì nó dễ đọc, debug được bằng một stack trace bình thường, và cho kiểm
soát chính xác với error message.

```rust
// grammar:  block := '{' entry* '}'   entry := ident value ';'
fn parse_block(&mut self) -> Result<Block, ParseError> {
    self.expect(Token::LBrace)?;
    let mut entries = Vec::new();
    while !self.check(Token::RBrace) {
        entries.push(self.parse_entry()?);
    }
    self.expect(Token::RBrace)?;
    Ok(Block { entries })
}
```

Gotcha: recursive descent đệ quy theo độ sâu nesting, nên input lồng quá
sâu (`{{{{...}}}}`, hoặc một JSON array sâu cả triệu tầng) có thể overflow
stack — trong Rust việc này abort process, không thể catch được. Bất kỳ
parser nào tiếp xúc với input không đáng tin cậy **phải** giới hạn độ sâu
nesting một cách tường minh bằng một counter, y hệt cách
`05-http-stack/01-parser.md` giới hạn số lượng header và kích thước body.
Đây là một vector DoS thật sự, không phải lý thuyết.

### Predictive parsing và lookahead một token
Một grammar dễ parse theo kiểu top-down khi rule hiện tại có thể được
chọn bằng cách nhìn một token (LL(1)). `match self.peek()` của bạn chọn
nhánh; không cần backtrack. Khi một token không đủ để quyết định, bạn
hoặc peek xa hơn, tái cấu trúc grammar, hoặc chấp nhận backtracking — và
backtracking trên input không đáng tin cậy tái tạo lại đúng rủi ro bùng nổ
theo cấp số mũ như một backtracking regex (`13-algorithms/regex-engine.md`).
Ưu tiên một grammar bạn có thể parse với lookahead cố định.

### Precedence: chỗ recursive descent ngây thơ trở nên xấu xí
Grammar biểu thức có precedence (`a + b * c`) viết theo recursive descent
thuần cần một hàm cho mỗi mức precedence, khá dài dòng. Pratt parsing
(precedence-climbing) gộp việc đó thành một vòng lặp duy nhất được dẫn dắt
bởi một bảng binding-power — mẹo tiêu chuẩn khi bạn cần operator
precedence. Một proxy hiếm khi cần biểu thức đầy đủ, nhưng một ngôn ngữ
rule của WAF hay một config với điều kiện `a && b || c` thì có.

### Parser combinator: idiom còn lại
Các thư viện như `nom` và `winnow` xây một parser bằng cách ghép các hàm
nhỏ (`tag`, `take_while`, `alt`, `many0`) thay vì viết một state machine
tường minh. Chúng rất tốt cho các định dạng nhị phân và mạng — `nom` được
dùng rộng rãi chính xác cho kiểu parsing giao thức ở mức byte mà một proxy
làm. Đánh đổi: error message khó làm cho chính xác hơn, và các kiểu
combinator có thể trở nên rối rắm. Dùng combinator cho binary framing,
recursive descent viết tay cho bất cứ chỗ nào chất lượng error quan trọng
với người chỉnh sửa input.

### Lỗi: phục hồi, đừng chỉ bail
Một parser chết ngay ở lỗi đầu tiên buộc người dùng phải sửa config từng
dòng một mỗi lần chạy. Parser thật sự phục hồi: khi gặp lỗi, bỏ qua token
tới một điểm đồng bộ đã biết (`;` hoặc `}` tiếp theo), ghi lại lỗi, và
tiếp tục để một lượt chạy báo cáo được nhiều vấn đề. Với một config
hot-reload (`09-architecture/03-config.md`) đây là khác biệt giữa một
công cụ dùng được và một công cụ gây bực bội — dù lưu ý rằng với config
bạn vẫn từ chối *toàn bộ* lần reload một cách atomic; bạn chỉ phục hồi để
gom hết lỗi hiển thị cùng lúc.

## Practice
1. Mở rộng lexer từ `15-parser/01-lexer.md` thành một recursive-descent
   parser tạo ra một config struct có kiểu, một hàm cho mỗi rule grammar.
   Đây là giai đoạn parse của `labs/13-hot-reload`.
2. Thêm một counter độ sâu nesting tường minh và một test đưa vào các
   block lồng sâu, chứng minh parser trả về một lỗi sạch thay vì overflow
   stack.
3. Implement phục hồi lỗi: khi gặp một entry sai, đồng bộ về `;` tiếp
   theo và tiếp tục, để một config có ba lỗi báo cáo cả ba trong một lần
   chạy.
4. Viết một Pratt parser cho một grammar biểu thức boolean nhỏ
   (`&&`/`||`/`!` với dấu ngoặc) kiểu mà một điều kiện WAF
   (`07-security/06-waf.md`) sẽ dùng, và kiểm chứng precedence bằng test.
5. Viết lại một rule bằng `nom` hoặc `winnow` và so sánh với phiên bản
   viết tay của bạn về độ dễ đọc và chất lượng error message; quyết định
   idiom nào phù hợp với config parsing so với binary framing.
