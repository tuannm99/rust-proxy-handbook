# Input Normalization & Parser Differentials

Hai component parse cùng một byte sequence theo hai cách khác nhau, và kẻ tấn
công sống trong khoảng cách đó. Đây là điểm yếu cấu trúc đứng sau các cách
bypass WAF ([`07-security/06-waf.md`](06-waf.md)), bypass access-control dựa trên path
([`05-http-stack/04-router.md`](../05-http-stack/04-router.md)), và — ở dạng thuần túy nhất của nó — request
smuggling ([`07-security/05-request-smuggling.md`](05-request-smuggling.md)). Normalization là biện
pháp phòng thủ, và nó là một ranh giới bảo mật chứ không phải một chi tiết
tiền xử lý.

## What to learn
### Hình dạng của vấn đề
Bất cứ khi nào proxy của bạn diễn giải input để đưa ra quyết định, rồi sau
đó chuyển tiếp *chính các byte gốc* để một thứ khác diễn giải lại lần nữa,
bạn có hai parser phải đồng thuận với nhau. Chúng sẽ không đồng thuận, trừ
khi bạn ép chúng phải vậy:

```
client bytes ──► proxy parse ──► quyết định (allow/block/route)
             └─► chuyển tiếp ──► upstream parse ──► diễn giải khác
```

Mục tiêu của kẻ tấn công là bất kỳ input nào khiến hai cách diễn giải đó
khác nhau. Mục tiêu của bạn là loại bỏ cách diễn giải thứ hai: normalize
một lần, quyết định trên dạng đã normalize, và **chuyển tiếp chính dạng đã
normalize** để không còn gì để bất đồng nữa.

Gotcha: đây là lý do "chuyển tiếp request đúng y như nhận được" — nghe có
vẻ thận trọng và tôn trọng — lại là lựa chọn nguy hiểm cho một proxy có vai
trò thực thi bảo mật. Việc chuyển tiếp byte trung thực giữ nguyên sự mơ hồ
mà bạn vừa mới giải quyết xong.

### Decode depth
Bạn URL-decode một lần; kẻ tấn công gửi `%252e%252e%252f`, decode một lần
ra `%2e%2e%2f` (không khớp) và decode hai lần ra [`../`](../..). Nếu upstream
framework decode hai lần, nó thấy traversal còn bạn thì không.

Decode lặp đi lặp lại cho tới khi ổn định lại có thất bại ngược lại: giờ
bạn flag những input mà upstream xử lý theo nghĩa đen, đây là false
positive chứ không phải một lỗ hổng thực sự — thường là loại lỗi tốt hơn để
mắc phải. Dù theo cách nào, yêu cầu thực sự là phải biết upstream của bạn
decode bao nhiêu lần và khớp với nó, rồi ghi lại lựa chọn đó ở nơi rule set
sống.

Gotcha: decode depth phải nhất quán trên *mọi* component kiểm tra cùng một
input. Nếu router decode một lần còn WAF decode hai lần, chúng bất đồng với
nhau, và đó chính là con bug tương tự chỉ khác ở chỗ kẻ tấn công đã tiến
gần hơn một bước.

### Case và Unicode
Lowercase xử lý được `<ScRiPt>`. Nó không xử lý được:
- **Ký tự full-width và homoglyph** mà một framework có thể normalize
  thành tương đương ASCII sau khi bạn đã kiểm tra xong.
- **Overlong UTF-8 encoding**, nơi một ký tự được encode bằng nhiều byte
  hơn cần thiết — về mặt lịch sử là một cách đáng tin cậy để lén đưa `/`
  hoặc [`.`](..) qua các kiểm tra so sánh byte.
- **Khác biệt Unicode case folding**: chữ ı Thổ Nhĩ Kỳ không chấm, chữ ß
  Đức, và các ký tự mà dạng viết hoa là nhiều ký tự.

Áp dụng Unicode normalization (NFKC) trước khi match nếu bất kỳ upstream
nào trong hạ tầng của bạn làm vậy, và từ chối input không phải UTF-8 hợp lệ
thay vì lossy-convert nó — `String::from_utf8_lossy` thay thế các byte sequence không hợp lệ bằng U+FFFD, việc này thay đổi các byte và có thể khiến
một payload độc hại trông vô hại.

### Charset và content-type
Một body khai báo `charset=utf-16` mà WAF của bạn quét như UTF-8 là, theo
rule của bạn, nhiễu vô nghĩa — và với một framework tôn trọng charset đã
khai báo, một payload sạch sẽ. Điều tương tự áp dụng cho một body JSON gửi
dưới dạng `text/plain` mà upstream vẫn parse như JSON.

Gotcha: quyết định xem bạn tin content-type đã khai báo hay tự sniff nó, và
biết upstream làm cái nào. Nếu chúng khác nhau, sự khác nhau đó chính là
lỗ hổng bypass.

### Ngữ nghĩa tham số khác nhau giữa các framework
`?id=1&id=2' OR '1'='1` là một request với hai giá trị `id`, và "giá trị
của `id`" nghĩa là gì phụ thuộc hoàn toàn vào ai đang hỏi:

| Nhóm framework | Lấy |
| --- | --- |
| PHP, Rails | cái cuối |
| ASP.NET | tất cả, nối bằng dấu phẩy |
| Nhiều router Go/Rust | cái đầu |
| Một số parser | mảng gồm cả hai |

Một WAF chỉ kiểm tra giá trị đầu tiên bỏ lỡ một payload mà upstream sẽ thực
thi; một WAF chỉ kiểm tra giá trị cuối cùng bỏ lỡ trường hợp ngược lại.

Quy tắc: kiểm tra **mọi lần xuất hiện của mọi tham số**, và kiểm tra cả
raw query string. Điều tương tự áp dụng cho header trùng lặp, và cho JSON
body có key trùng lặp ([`15-parser/`](../15-parser) — hành vi parser ở đó cũng khác nhau).

Gotcha: các framing header là trường hợp cực đoan của vấn đề này, nơi sự
bất đồng cho bạn nguyên một request bị smuggle thay vì chỉ một giá trị
tham số sai. [`07-security/05-request-smuggling.md`](05-request-smuggling.md) nói về nó; lý luận y
hệt, chỉ ở một tầng sâu hơn.

### Path là vấn đề normalization của riêng nó
Dot segment, separator đã encode, dấu gạch chéo trùng lặp, dấu gạch chéo ở
cuối, và filesystem không phân biệt hoa thường đều khiến path bạn dùng để
route khác với path mà upstream resolve. [`05-http-stack/04-router.md`](../05-http-stack/04-router.md) nói
về các biến thể và quy tắc canonical-form; đó chính là kỷ luật tương tự áp
dụng cho input duy nhất mà một proxy luôn luôn phải parse.

### Normalize một lần, sớm, ở một nơi duy nhất
Kiểu thất bại sống sót qua tất cả những điều trên là normalize ở nhiều nơi
với các rule hơi khác nhau. Đặt normalization tại một điểm duy nhất trong
pipeline ([`09-architecture/01-components.md`](../09-architecture/01-components.md), trước routing), lưu dạng
canonical trên request, và để mọi component sau đó — router, WAF, logger,
bộ chuyển tiếp upstream — đọc *chính dạng đó* thay vì tự tính lại của riêng
mình.

Gotcha: giữ lại các byte gốc để log
([`08-observability/01-logging.md`](../08-observability/01-logging.md)) để một cuộc điều tra có thể thấy những
gì thực sự đã được gửi, nhưng chỉ để dạng canonical là thứ duy nhất mà bất
kỳ *quyết định* nào được đọc. Có hai dạng biểu diễn thì ổn; có hai nguồn
input cho quyết định thì không.

## Practice
Làm lần lượt theo thứ tự sau.

1. Trong [`labs/12-waf`](../../labs/12-waf), viết các bài test bypass trước khi có bất kỳ
   normalization nào: mixed case, double URL-encoding, một overlong UTF-8
   encoding của `/`, một payload nằm ở tham số *thứ hai* trong hai tham số
   trùng tên, và một body có charset khai báo khác với encoding thực tế.
   **Xong khi** tất cả đều lọt qua — đây là baseline.
2. Thêm một tầng normalization duy nhất: URL-decode tới một độ sâu đã ghi
   lại, lowercase, NFKC, từ chối UTF-8 không hợp lệ. **Xong khi** ba bài
   test đầu bị bắt và bạn có thể nói rõ decode depth của mình và vì sao.
3. Kiểm tra mọi lần xuất hiện của mọi tham số cộng với raw query string.
   **Xong khi** bài test tham số trùng lặp bị bắt bất kể payload nằm ở vị
   trí nào.
4. Xác định bằng thực nghiệm upstream của bạn làm gì. **Xong khi** bạn đã
   test — không phải giả định — nó decode bao nhiêu lần và lấy tham số
   trùng lặp nào, và normalization của bạn khớp với điều đó.
5. Làm cho normalization có nguồn duy nhất. **Xong khi** router, WAF, và bộ
   chuyển tiếp upstream đều đọc cùng một dạng canonical, và một bài test
   chứng minh request được chuyển tiếp mang path đã normalize thay vì byte
   gốc.
6. Chỉ giữ bản gốc cho log. **Xong khi** dòng log của một request bị block
   cho thấy input thô như đã gửi, trong khi không có decision path nào có
   thể đọc nó.
