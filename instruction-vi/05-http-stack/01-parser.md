# HTTP Parser

## What to learn

### Request/status line
Một request line HTTP/1.1 là `METHOD SP request-target SP HTTP-version CRLF` (RFC 9112 §3). Một parser tự viết phải reject bất cứ thứ gì không khớp chính xác — thừa khoảng trắng, thiếu version, hay một `\n` trơ thay vì `\r\n` đều là attack surface có thật ngoài đời, không chỉ là input hỏng để lờ đi.

### Parse header
Header là các dòng `name: value CRLF` cho tới một dòng trống. Parser viết ẩu thường dính các lỗi: so khớp tên header phải case-insensitive, khoảng trắng đầu/cuối trong value, header trùng lặp (một số phải bị reject thẳng, ví dụ `Content-Length` trùng), và obsolete line folding (dòng tiếp nối bắt đầu bằng khoảng trắng) mà parser hiện đại nên đơn giản là reject.

```rust
// sketch: split a raw header line into (name, value), reject folding
fn parse_header_line(line: &str) -> Option<(&str, &str)> {
    let (name, value) = line.split_once(':')?;
    if name.is_empty() || name.contains(' ') {
        return None; // no whitespace before the colon, per RFC 9112 §5.1
    }
    Some((name, value.trim()))
}
```

### Body framing: phần thực sự quan trọng
Độ dài của body đến từ đúng một trong ba: `Content-Length`, `Transfer-Encoding: chunked`, hoặc "đọc tới khi connection đóng" (chỉ áp dụng cho response). Nếu một message có *cả* `Content-Length` lẫn `Transfer-Encoding`, hoặc nhiều giá trị `Content-Length` mâu thuẫn nhau, message đó là ambiguous — RFC 9112 §6.3 nói phải reject nó, không phải "chọn một cái". Làm sai chỗ này chính xác là cách request smuggling xảy ra (xem [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md)).

### Chunked transfer-encoding
Mỗi chunk có dạng `<hex-size>CRLF<data>CRLF`, kết thúc bằng một chunk kích thước `0` và trailer tùy chọn. Một parser đúng phải giới hạn số chữ số của chunk-size và tổng kích thước đã decode (attacker có thể khai một chunk-size khổng lồ để làm cạn memory), và không được coi trailer header tương đương với header gửi trước body.

### Incremental parsing: ràng buộc định hình mọi thứ
Một parser đọc từ socket **không** nhận được một request hoàn chỉnh. Nó
nhận được bất cứ gì `read()` trả về — nửa một dòng header, hai request rưỡi,
hay ba byte. Một parser viết kiểu "nhận `&str`, trả về `Request`" là không
thể test đúng với thực tế và sẽ phải viết lại ngay khi gặp một socket thật.

Cách viết đúng là trả về đã tiêu thụ bao nhiêu, hoặc xin thêm dữ liệu:

```rust
enum ParseStatus<T> {
    Complete { value: T, consumed: usize },
    Partial, // need more bytes; caller must preserve what it has
}
```

Đây là API của `httparse` và cũng là contract nội bộ của hyper, và nó buộc
bạn phải có hai tính chất cần thiết: parser không bao giờ block trên I/O, và
caller sở hữu buffer. Khi gặp `Partial`, caller đọc thêm vào *cùng* buffer
đó rồi parse lại từ đầu — không state nào được giữ lại giữa các lần gọi.

Gotcha: parse lại từ đầu ở mỗi lần đọc là O(n²) nếu attacker gửi từng byte
một (n lần đọc × n byte quét lại). Với một parser để học, điều này chấp
nhận được và đáng để đo đạc; parser production hoặc giới hạn header size đủ
chặt để n² bị chặn trên, hoặc giữ một state machine tường minh với một
resume offset. Lưu ý tương tác với [`07-security/09-ddos.md`](../07-security/09-ddos.md): gửi từng byte
một *chính là* tấn công Slowloris, nên ngưỡng tốc độ đọc dữ liệu và giới hạn
của parser này bảo vệ cùng một lỗ hổng từ hai phía.

### Quản lý buffer qua nhiều lần đọc
Vòng lặp của caller phải phân biệt bốn trường hợp: parse được một message
hoàn chỉnh (tiêu thụ những byte đó, giữ lại phần dư — nó có thể chứa request
*tiếp theo* trong pipelining), cần thêm dữ liệu (đọc tiếp), buffer đầy mà
chưa có message hoàn chỉnh (reject — đây là lúc giới hạn header size của bạn
phát huy tác dụng), và EOF giữa chừng message (reject; một request bị cắt
cụt không phải một request hợp lệ).

Trường hợp cuối rất quan trọng: EOF khi request còn dang dở là lỗi, nhưng
EOF ở ranh giới message sạch sẽ là một lần đóng connection bình thường. Gộp
lẫn hai cái này hoặc là làm rò rỉ nửa request vào handler của bạn, hoặc log
lỗi ở mọi lần client ngắt kết nối đàng hoàng.

Gotcha: sau `Complete { consumed }`, các byte còn dư phải được dịch về đầu
buffer (hoặc theo dõi bằng một read cursor) trước lần đọc kế tiếp. Quên điều
này là bug pipelining kinh điển — request thứ hai trên một connection
keep-alive ([`05-http-stack/04-keepalive.md`](04-keepalive.md)) bị parse từ một buffer vẫn còn
đuôi của request thứ nhất.

### Giới hạn là một phần của parser, không phải một wrapper quanh nó
Mọi đại lượng không giới hạn đều là một vector làm cạn memory, và parser là
nơi duy nhất có đủ ngữ cảnh để chặn chúng. Tối thiểu, hãy áp: giới hạn độ
dài request line, số lượng header tối đa, kích thước một header tối đa,
tổng kích thước khối header tối đa, kích thước chunk tối đa, tổng kích
thước body tối đa. Áp chúng *trong lúc parse* — kiểm tra sau khi đã tích
lũy dữ liệu là kiểm tra sau khi attacker đã thắng.

Con số thật để tham khảo: nginx cho phép request line 8k và header buffer
8k; hyper mặc định giới hạn header ở 100. Chọn con số, ghi lại chúng, và
biến việc vượt giới hạn thành một reject sạch với đúng status
(`431 Request Header Fields Too Large`, `413 Content Too Large`) thay vì
panic hay cắt xén.

### Bytes, không phải `String`
HTTP là một byte protocol. Giá trị header có thể hợp pháp chứa byte không
phải UTF-8 hợp lệ, và một URI là byte cho tới khi bạn quyết định khác đi —
nên một parser xây trên `&str` hoặc reject traffic hợp lệ, hoặc giấu một
phép convert không kiểm tra. Làm việc trên `&[u8]`, validate tường minh
theo character class của RFC (token, VCHAR, obs-text), và chỉ convert khi
đã xác định byte đó là ASCII.

Gotcha: tên header case-insensitive, nên so sánh cũng phải vậy, nhưng gọi
`to_lowercase()` cho mỗi header mỗi request là allocate trên hot path. So
sánh bằng `eq_ignore_ascii_case` với một hằng static, và lưu ý
`str::to_lowercase` làm full Unicode case folding — vừa sai vừa chậm cho
thứ vốn được định nghĩa là ASCII token. Đây là chỗ kỷ luật
borrow-thay-vì-clone của [`03-rust/01-ownership.md`](../03-rust/01-ownership.md) phát huy tác dụng: một
request đã parse nên borrow slice từ buffer input, không sở hữu bản copy
của từng field.

### Vì sao code production dùng hyper thay vì tự viết parser
Codec HTTP/1 (`h1`) của `hyper` đã hấp thụ nhiều năm sửa lỗi tương thích và bảo mật cho chính xác những ambiguity ở trên. Tự viết một parser có giá trị để học những ambiguity đó *là gì*, nhưng ship một cái tự viết trong [`labs/02-http-server`](../../labs/02-http-server) hay sau này đồng nghĩa với việc tự khám phá lại từng CVE smuggling mà hyper đã fix.

## Practice
1. Trong [`labs/01-http-parser`](../../labs/01-http-parser), parse request line và header từ một buffer `&[u8]` thô vào một struct *borrow* từ nó; reject input hỏng thay vì cố phục hồi best-effort.
2. Làm cho entry point trả về `ParseStatus` (complete + số byte đã tiêu thụ, hoặc partial). Test bằng cách feed một request từng byte một và assert nó trả về `Partial` cho tới byte cuối cùng.
3. Thêm body framing: hỗ trợ `Content-Length`, rồi `Transfer-Encoding: chunked`, và reject tường minh một request khai cả hai.
4. Viết vòng lặp buffer xung quanh nó: xử lý byte dư sau một message hoàn chỉnh, và chứng minh pipelining hoạt động bằng cách parse hai request từ một buffer trong một lần đọc. Sau đó assert EOF giữa request là lỗi còn EOF ở ranh giới message thì không.
5. Áp các giới hạn ở trên (số header, kích thước, chunk size) và viết một test reject cho mỗi giới hạn, assert đúng status code chứ không chỉ "có lỗi".
6. Feed parser của bạn các input mang tính đối kháng (`Content-Length` trùng với giá trị khác nhau, header bị fold, một `\n` trơ, chunk size `0x` theo sau bởi rác, tên header có khoảng trắng thừa cuối) và xác nhận nó reject từng cái thay vì parse "ra được gì đó".
7. Fuzz [`labs/01-http-parser`](../../labs/01-http-parser) (xem [`12-testing/02-fuzzing.md`](../12-testing/02-fuzzing.md)) với một corpus các bản capture traffic HTTP/1.1 thật và fix mọi panic.
8. Sau khi tự tay làm xong, đọc API của `httparse` và codec `h1` của `hyper` cho cùng các trường hợp, và ghi lại chúng làm khác implementation của bạn ở điểm nào.
