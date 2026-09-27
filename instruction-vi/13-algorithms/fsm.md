# FSM (Finite State Machine) như một Design Pattern

`13-algorithms/dfa.md` nói về lý thuyết automaton accept/reject. File này
nói về finite state machine như một thứ bạn cố tình dùng đến khi *viết*
Rust — protocol parser và vòng đời kết nối là state machine dù bạn có mô
hình hóa chúng như vậy hay không, và việc mô hình hóa chúng một cách tường
minh chính là thứ ngăn các bug trạng-thái-không-hợp-lệ xuất hiện khi bạn
không làm vậy.

## What to learn

### Vượt ra ngoài accept/reject: máy có hành động
Một DFA chỉ trả lời "accept hay reject." Một **máy Mealy** (hành động ở
mỗi transition) hay **máy Moore** (hành động ở mỗi state) gắn hành vi vào
chính các transition — đây chính xác là bản chất của một kết nối HTTP/1.1:
`Idle -> ReadingRequestLine -> ReadingHeaders -> ReadingBody -> Idle`
(keep-alive) hoặc `-> Closed`, với công việc thật xảy ra ở mỗi cạnh, chứ
không chỉ một câu trả lời có/không ở cuối. Xem `05-http-stack/01-parser.md`
và `05-http-stack/04-keepalive.md` để có các state cụ thể; file này nói về
cách mã hóa bản thân cái máy đó.

### Enum + match: trường hợp phổ biến
```rust
enum ParseState {
    RequestLine,
    Headers { partial: Vec<u8> },
    Body { remaining: usize },
    Done,
}

fn advance(state: ParseState, byte: u8) -> ParseState {
    match state {
        ParseState::RequestLine => { /* ... */ ParseState::Headers { partial: vec![] } }
        // ...
        _ => state,
    }
}
```
Rẻ để viết, dễ đọc, và việc kiểm tra tính đầy đủ (exhaustiveness) của
`match` mà compiler làm nghĩa là thêm một state mới buộc bạn phải xử lý nó
ở mọi nơi — một sự bảo vệ thật sự, miễn phí.

### Typestate: đẩy các transition bất hợp lệ lên compile time
Cách tiếp cận enum vẫn cho phép code *lúc chạy* gọi sai method trên sai
state (không gì ngăn bạn cố đọc body trước khi header parse xong ngoại
trừ một arm `match` mà bạn phải nhớ viết). **Typestate pattern** mã hóa
mỗi state thành một type riêng biệt, để lời gọi bất hợp lệ trở thành lỗi
compile-time, không phải một nhánh runtime:

```rust
struct Idle;
struct HeadersRead { content_length: Option<usize> }

impl Idle {
    fn read_headers(self) -> HeadersRead { /* ... */ HeadersRead { content_length: None } }
}
impl HeadersRead {
    fn read_body(self) -> Body { /* chỉ gọi được khi header đã tồn tại */ Body }
}
```
`Idle` đơn giản là không có method `read_body` — gọi nó sai thứ tự thì
không compile được, thay vì panic hoặc âm thầm sai lệch trong production.
Xem `03-rust/01-ownership.md` để hiểu vì sao việc consume `self` (chứ
không phải `&self`) mới là thứ khiến pattern này thực sự ép buộc các
transition một chiều.

### Chọn giữa hai cách
Đảm bảo compile-time của typestate tốn nhiều boilerplate hơn (một type
cho mỗi state, các phép chuyển đổi giữa chúng) và không compose tốt khi
state phải được lưu trong một field của struct hoặc truyền qua một ranh
giới `.await` bên trong cùng một future (type sẽ phải thay đổi giữa lúc
poll). Mặc định dùng enum+match; dùng typestate cụ thể cho một state
machine nơi một transition bất hợp lệ là một lớp bug bạn đã thực sự gặp
phải hoặc sẽ là một vấn đề bảo mật (ví dụ gửi response trước khi xác nhận
request đã được đọc đầy đủ).

## Practice
1. Mô hình hóa các state parse-request của `labs/01-http-parser` thành
   một enum tường minh *trước khi* viết logic parsing; dùng tính đầy đủ
   của `match` để xác nhận mọi state đều có transition được định nghĩa
   cho mọi lớp byte nó có thể thấy.
2. Viết lại vòng đời kết nối keep-alive từ
   `05-http-stack/04-keepalive.md` (idle → reading → responding →
   idle/closed) thành một chuỗi typestate; thử gọi một method sai thứ tự
   và xác nhận nó không compile được.
3. Chọn một transition mà parser dựa trên enum của bạn xử lý bằng
   `_ => unreachable!()` hoặc fallthrough âm thầm, và giải thích bằng văn
   bản vì sao typestate sẽ (hoặc sẽ không) bắt được một bug ở đó tại
   compile time thay vì tại runtime.
