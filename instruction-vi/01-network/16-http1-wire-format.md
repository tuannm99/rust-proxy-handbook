# HTTP/1.1 Wire Format

Grammar chính xác ở mức byte của một message HTTP/1.1 (RFC 9112, cùng các
lớp ký tự lấy từ RFC 9110): cái gì hợp lệ, cái gì phải reject, và chỗ nào
spec để bạn tự chọn. [`01-network/15-http.md`](15-http.md) nói một message *mang ý
nghĩa gì*; file này nói nó *trông thế nào trên dây*, và đây là spec mà
[`labs/01-http-parser`](../../labs/01-http-parser) implement. Bạn không cần mở RFC mới làm xong lab
đó. Số section được ghi kèm để bạn đối chiếu bất kỳ luật nào tại nguồn.

## What to learn

### Hình dạng của một message
Mọi message HTTP/1.1 đều gồm đúng bốn phần (RFC 9112 §2.1):

```text
HTTP-message = start-line CRLF
               *( field-line CRLF )
               CRLF
               [ message-body ]
```

Một start line, không hoặc nhiều dòng header, một dòng trống, rồi một body
tùy chọn mà độ dài được quyết định bởi header (xem "Độ dài message body"
bên dưới), không bao giờ bằng cách đi tìm ký tự kết thúc. Đây là một
request thật, từng byte một (`\r\n` là CRLF, hai byte `0x0D 0x0A`):

```text
POST /upload?id=7 HTTP/1.1\r\n        <- request-line
Host: example.com\r\n                  <- field-line
Content-Length: 5\r\n                  <- field-line
\r\n                                   <- dòng trống: hết phần header
hello                                  <- body: đúng 5 byte, không có CRLF theo sau
```

Dòng trống là cách duy nhất để biết phần header đã hết. Trong buffer byte,
đó là chuỗi `\r\n\r\n`: CRLF đóng dòng header cuối cùng, ngay sau đó là
CRLF của dòng trống. Body không có ký tự kết thúc riêng. Sau 5 byte của
nó, byte *tiếp theo* trên một connection keep-alive là byte đầu tiên của
request kế tiếp.

### Lớp ký tự: bảng chữ cái của grammar
Grammar được viết bằng ABNF dựa trên vài lớp byte. Mọi phép kiểm tra hợp
lệ trong parser của bạn đều quy về một trong số này:

| Tên | Byte | Dùng cho |
|---|---|---|
| `CRLF` | `0x0D 0x0A` | kết thúc dòng |
| `SP` / `HTAB` | `0x20` / `0x09` | phân cách |
| `OWS` (optional whitespace) | không hoặc nhiều `SP`/`HTAB` | quanh value của header |
| `BWS` ("bad" whitespace) | giống `OWS` | chỉ được phép để dễ dãi, quanh `;` và `=` trong chunk extension |
| `DIGIT` | `0`-`9` | `Content-Length`, version |
| `HEXDIG` | `0`-`9`, `A`-`F`, `a`-`f` | chunk size |
| `VCHAR` | `0x21`-`0x7E` (ASCII nhìn thấy được) | value của header |
| `obs-text` | `0x80`-`0xFF` | value của header (hợp lệ, nhưng không đảm bảo là UTF-8) |
| `tchar` | ALPHA, DIGIT, và ``! # $ % & ' * + - . ^ _ ` \| ~`` | token |
| `token` | một hoặc nhiều `tchar` | method, tên header, tên coding |

Một `token` không chứa khoảng trắng, không có `:`, không có `"`, không có
`(),/;<=>?@[\]{}`, và không có byte điều khiển. Nên câu hỏi "đây có phải
tên header hợp lệ không?" chính xác là "nó có phải một hoặc nhiều `tchar`
không?" (RFC 9110 §5.6.2). Chỉ riêng phép kiểm tra đó đã reject
`Content-Length ` (thừa dấu cách ở cuối), `Content Length`, và một tên có
byte NUL bên trong.

Gotcha: `tchar` chỉ gồm ASCII. Một parser kiểm tra "không phải khoảng trắng
và không phải `:`" thay vì "là `tchar`" sẽ chấp nhận những byte mà parser
khác reject, và việc hai parser bất đồng về cái gì được tính là header
chính là nguyên liệu thô của [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md).

### Request line
```text
request-line = method SP request-target SP HTTP-version
method       = token
HTTP-version = "HTTP" "/" DIGIT "." DIGIT
```

- **Method** là một `token` và **phân biệt hoa thường** (RFC 9110 §9.1):
  `get` không phải `GET`, nó là một method lạ. Server không implement một
  method thì trả `501 Not Implemented`.
- **Phân cách** là đúng một `SP` mỗi chỗ. RFC 9112 §3 cho phép bên nhận
  *tùy chọn* tách theo bất kỳ chuỗi khoảng trắng nào (SP, HTAB, VT, FF, CR
  trơ), và ngay trong đoạn đó cảnh báo rằng sự dễ dãi này gây ra
  smuggling. Làm chặt chỗ này không làm bạn mất gì với client thật.
- **Version** đúng nguyên văn là `HTTP/`, một chữ số, `.`, một chữ số, và
  `HTTP` phân biệt hoa thường. `HTTP/1.1` và `HTTP/1.0` là thứ bạn sẽ gặp.
  Major version không hỗ trợ thì trả `505 HTTP Version Not Supported`.
- **Độ dài**: RFC khuyến nghị hỗ trợ request line dài ít nhất 8000 byte.
  Dài hơn giới hạn của bạn → `414 URI Too Long`.
- Mọi request line sai định dạng khác → `400 Bad Request`.

### Request target: bốn dạng
Request target không bao giờ chứa khoảng trắng, và có bốn dạng (RFC 9112
§3.2). Dạng nào hợp lệ phụ thuộc vào method:

| Dạng | Ví dụ | Khi nào |
|---|---|---|
| origin-form | `/where?q=now` | request bình thường tới origin server. Phải bắt đầu bằng `/` |
| absolute-form | `http://example.com/where?q=now` | request gửi *tới một proxy*. Server cũng phải chấp nhận nó |
| authority-form | `example.com:443` | chỉ cho `CONNECT` |
| asterisk-form | `*` | chỉ cho `OPTIONS *` (cả server, không phải một resource) |

Target là byte, không phải một path đã decode. Percent-decoding (`%2F` →
`/`) và loại bỏ dot-segment (`/a/../b`) là một bước riêng, diễn ra sau, có
luật bảo mật riêng ([`07-security/04-normalization.md`](../07-security/04-normalization.md)). Parser nên chuyển
nguyên target thô cho tầng tiếp theo, đừng "dọn dẹp" nó.

### Header `Host` là một phần của framing
Một request HTTP/1.1 phải mang đúng một field `Host` (RFC 9112 §3.2).
Server phải trả `400` cho request HTTP/1.1 không có `Host`, có nhiều hơn
một dòng `Host`, hoặc có giá trị `Host` không hợp lệ. Khi target ở
absolute-form, authority bên trong target thắng và header `Host` bị bỏ
qua. Điều này quan trọng với proxy vì `Host` chọn virtual host
([`05-http-stack/12-vhost-routing.md`](../05-http-stack/12-vhost-routing.md)). Hai giá trị `Host` mà hai tầng hiểu khác
nhau là một bug nhầm lẫn routing.

### Field line (header)
```text
field-line  = field-name ":" OWS field-value OWS
field-name  = token
field-value = *field-content      ; VCHAR, obs-text, và SP/HTAB ở giữa chúng
```

Các luật, tất cả từ RFC 9112 §5 và RFC 9110 §5:

- **Không có gì giữa tên và dấu hai chấm.** `Host : x` phải bị reject với
  `400` (§5.1). Đây là MUST, không phải lựa chọn. Nó tồn tại vì từng có
  parser coi `Transfer-Encoding : chunked` là một header còn parser khác
  thì không.
- **Khoảng trắng quanh value không thuộc về value.** Bỏ `SP`/`HTAB` ở đầu
  và cuối. Khoảng trắng *bên trong* value được giữ nguyên.
- **Value rỗng là hợp lệ.** `X-Empty:` rồi CRLF là một field hợp lệ với
  value `""`.
- **CR, LF, hoặc NUL bên trong value** làm message không hợp lệ. Bên nhận
  phải reject hoặc thay từng byte đó bằng `SP` (RFC 9110 §5.5). Reject là
  lựa chọn an toàn: một CR lạc chỗ là đường cho response splitting và
  header injection lọt vào.
- **Một CR trơ** ở bất kỳ đâu ngoài body (CR không có LF theo sau) là
  không hợp lệ, cùng luật reject-hoặc-thay (§2.2).
- **Tên không phân biệt hoa thường, và chỉ là ASCII.** `content-length`,
  `Content-Length` và `CONTENT-LENGTH` là cùng một field.
- **Value là byte.** `obs-text` (`0x80`-`0xFF`) hợp lệ, nên value không
  đảm bảo là UTF-8. Đừng làm kiểu header của bạn là `String`.

### Field trùng lặp
Sender chỉ được lặp lại một tên field nếu value của field đó là một danh
sách phân cách bằng dấu phẩy (RFC 9110 §5.3), ví dụ `Accept`, `Via`,
`Transfer-Encoding`, `Cache-Control`. Với những field này, hai dạng sau
mang cùng nghĩa, và thứ tự phải được giữ:

```text
Cache-Control: no-cache\r\n            Cache-Control: no-cache, no-store\r\n
Cache-Control: no-store\r\n
```

Field *không* phải danh sách thì chỉ được xuất hiện một lần, và những field
quyết định framing hay routing chính là những field bạn phải làm chặt:
`Host` (reject trùng lặp, xem trên) và `Content-Length` (xem dưới).
`Set-Cookie` là ngoại lệ nổi tiếng: nó lặp lại, nhưng không nối bằng dấu
phẩy được, vì ngày tháng trong cookie có chứa dấu phẩy. Field này chỉ xuất
hiện trong response.

### Obsolete line folding và LF trơ: hai quyết định của bạn
Hai dạng cũ vẫn còn trong grammar vì lý do tương thích, và lab yêu cầu bạn
quyết định cho từng cái và ghi lại lý do:

- **obs-fold** (§5.2): một dòng header bắt đầu bằng SP hoặc HTAB là phần
  nối tiếp value của header trước (`X-Long: a\r\n  b\r\n`). Nó bị loại bỏ
  vì các parser bất đồng về nó. Server nhận nó trong một request phải hoặc
  reject với `400`, hoặc thay phần fold (CRLF cộng khoảng trắng đầu dòng)
  bằng SP trước khi diễn giải value.
- **LF trơ** (§2.2): bên nhận *được phép* chấp nhận một `\n` đơn lẻ làm
  ký tự kết thúc dòng. Hai parser mà một cái chấp nhận `\n` còn cái kia chỉ
  nhận `\r\n` sẽ thấy ranh giới header khác nhau trên cùng một byte sequence.

Một MUST liên quan từ §2.2: một dòng khoảng trắng nằm giữa request line và
header đầu tiên phải bị reject, hoặc toàn bộ dòng bắt đầu bằng khoảng
trắng đó bị bỏ qua. Và để chịu lỗi tốt hơn, server *nên* bỏ qua ít nhất một
dòng trống (một CRLF thừa còn sót từ request trước) nằm *trước* request
line.

### Độ dài message body: các luật có thứ tự
Đây là phần quyết định tính đúng và bảo mật. RFC 9112 §6.3 đưa ra các luật
được áp dụng **theo thứ tự**, luật đầu tiên khớp sẽ thắng. Với server đang
đọc một request:

1. **Có cả `Transfer-Encoding` và `Content-Length`.** TE đè CL, và message
   "ought to be handled as an error" vì đây là kịch bản smuggling kinh
   điển. Server *được phép* reject nó, hoặc xử lý chỉ theo TE, nhưng dù
   cách nào cũng *phải* đóng connection sau khi trả lời (§6.1). Proxy nào
   chuyển tiếp nó thì phải bỏ `Content-Length` trước. Reject là một policy
   chặt hơn mức RFC yêu cầu, và là một lựa chọn bảo vệ được.
2. **Có `Transfer-Encoding`.** Nếu `chunked` là coding **cuối cùng** trong
   danh sách, body là chunked (xem dưới). Nếu không, không xác định được độ
   dài của request: trả `400` và đóng.
3. **Có `Content-Length` nhưng không hợp lệ.** Không hợp lệ nghĩa là:
   không phải `1*DIGIT`, hoặc nhiều giá trị mâu thuẫn nhau. Trả `400` và
   đóng. Lỗi này không cứu được, vì bạn không biết request tiếp theo bắt
   đầu ở đâu.
4. **`Content-Length` hợp lệ.** Body dài đúng bằng số byte đó. Nếu
   connection đóng trước, message chưa hoàn chỉnh, và đó là lỗi, không phải
   một body ngắn.
5. **Không có cái nào.** Request **không có body** (độ dài 0).

Những chi tiết mà các luật trên phụ thuộc vào:

- `Content-Length = 1*DIGIT`: chỉ chữ số thập phân. Không dấu (nên không có
  `-1` và không có `+5`), không hex, không khoảng trắng bên trong, không
  rỗng. Coi chừng overflow: `99999999999999999999999` toàn chữ số nhưng vẫn
  không hợp lệ với bạn. RFC dặn bạn phải lường trước những số khổng lồ.
- Giá trị lặp lại giống hệt nhau (`Content-Length: 5, 5`, hoặc hai dòng đều
  là `5`) *có thể* được chấp nhận như một `5` duy nhất, hoặc bị reject (RFC
  9110 §8.6). Giá trị khác nhau thì luôn không hợp lệ.
- `Transfer-Encoding` là danh sách tên coding phân cách bằng dấu phẩy,
  không phân biệt hoa thường (`Chunked` là `chunked`). Nhiều dòng TE gộp
  lại thành một danh sách. `chunked` phải đứng cuối và không được xuất hiện
  hai lần. Một coding lạ trong request → `501`.
- `Transfer-Encoding` trong một message **HTTP/1.0** nghĩa là framing bị
  lỗi, kể cả khi cũng có `Content-Length`. Xử lý rồi đóng (§6.1). TE không
  tồn tại trong 1.0.

Phía response (thứ [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) cần khi đọc reply từ upstream)
thêm ba thứ đứng *trước* các luật trên: response cho `HEAD`, hoặc có status
`1xx`, `204` hay `304`, không bao giờ có body dù header nói gì. Một `2xx`
cho `CONNECT` biến connection thành tunnel. Và response không có cả TE lẫn
CL thì kéo dài tới khi connection đóng. Luật cuối chỉ dành cho response,
nên "đọc tới khi đóng" không bao giờ áp dụng cho request.

### Chunked transfer coding
Khi sender không biết trước độ dài, body là một chuỗi chunk (RFC 9112
§7.1):

```text
chunked-body    = *chunk last-chunk trailer-section CRLF
chunk           = chunk-size [ chunk-ext ] CRLF chunk-data CRLF
chunk-size      = 1*HEXDIG
last-chunk      = 1*("0") [ chunk-ext ] CRLF
chunk-ext       = *( BWS ";" BWS ext-name [ BWS "=" BWS ext-val ] )
ext-name        = token
ext-val         = token / quoted-string
trailer-section = *( field-line CRLF )
```

Cùng body `hello world`, từng byte một:

```text
5\r\n                  <- chunk-size (hex 5 = 5 byte)
hello\r\n              <- 5 byte data, rồi một CRLF bắt buộc
6;note=x\r\n           <- size 6 kèm một chunk extension
 world\r\n             <- 6 byte: " world", rồi CRLF
0\r\n                  <- last-chunk: size bằng 0
Checksum: abc\r\n      <- trailer-section: không hoặc nhiều field line
\r\n                   <- dòng trống: chunked body kết thúc ở đây
```

Mỗi phần yêu cầu gì:

- **chunk-size** là hex, không phân biệt hoa thường, và được có số 0 ở đầu
  (`000a` là 10). Không có tiền tố `0x`, không dấu, không khoảng trắng phía
  trước. Trong grammar nó có thể dài tùy ý, nên hãy giới hạn số chữ số
  trước khi chuyển đổi, nếu không `ffffffffffffffffff` sẽ overflow.
- **CRLF sau chunk-data là bắt buộc**, và phải đúng là CRLF. Một parser đếm
  đủ `size` byte rồi bỏ qua "kiểu kết thúc dòng nào cũng được" sẽ bất đồng
  với một parser chặt về chỗ chunk-size tiếp theo bắt đầu. Đó là một biến
  thể smuggling đã được ghi nhận.
- **Chunk extension** (`;name=value` sau size) không mang ý nghĩa gì bạn
  cần: bên nhận phải bỏ qua những cái nó không nhận ra. Bạn vẫn phải
  *parse* chúng để tìm ra CRLF, và giới hạn tổng độ dài của chúng, vì nếu
  không chúng là một buffer không giới hạn do attacker điều khiển (§7.1.1).
  `ext-val` có thể là một `quoted-string`: `"` ... `"`, trong đó `\` escape
  byte tiếp theo, và nội dung có thể chứa `;` và `=`.
- **Chunk cuối** là một hoặc nhiều chữ số `0` (có thể kèm extension) rồi
  CRLF. Nó không mang data và không có CRLF-sau-data của riêng nó.
- **Trailer section** là không hoặc nhiều field line, cùng grammar và cùng
  luật như header, kết thúc bằng một dòng trống. Không có trailer thì body
  kết thúc bằng `0\r\n\r\n`. Trailer không được gộp vào phần header trừ khi
  định nghĩa của field cho phép (§7.1.2). Hãy giữ chúng tách riêng: một
  `Content-Length` hay `Host` đến dưới dạng trailer không bao giờ được thay
  đổi framing hay routing.
- **Độ dài sau decode** là tổng các chunk size. Giới hạn body tối đa của
  bạn áp lên tổng đó, không phải lên từng chunk.

Hãy đọc nó như một state machine nhỏ: đọc dòng size → đọc đủ số byte data
đó → chờ CRLF → lặp lại, cho tới khi một size bằng 0 chuyển bạn sang đọc
các dòng trailer cho tới một dòng trống. Ở state nào cũng có thể hết input
giữa chừng, đó là lý do phần incremental parsing của
[`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) cũng áp dụng cho body.

### Trả gì khi reject
Status code là một phần của việc reject cho đúng:

| Tình huống | Status |
|---|---|
| request line sai, cú pháp header sai, vi phạm luật `Host`, framing CL/TE | `400 Bad Request` |
| request line/target dài hơn giới hạn của bạn | `414 URI Too Long` |
| phần header quá lớn hoặc quá nhiều field | `431 Request Header Fields Too Large` |
| body lớn hơn giới hạn của bạn | `413 Content Too Large` |
| method lạ, hoặc transfer coding lạ | `501 Not Implemented` |
| major version HTTP không hỗ trợ | `505 HTTP Version Not Supported` |

Sau mọi lỗi framing (`400` vì framing, và mọi xung đột CL/TE), hãy đóng
connection. Bạn không còn biết message tiếp theo bắt đầu ở đâu, nên dùng
lại connection nghĩa là parse byte của attacker như một request.

### Response line (cho sau này)
Response bắt đầu bằng `status-line = HTTP-version SP status-code SP
[ reason-phrase ]`, trong đó status-code là đúng ba chữ số và reason
phrase (`OK`, `Not Found`) là văn bản tự do mà client nên bỏ qua. Mọi thứ
sau start line (field, dòng trống, độ dài body) dùng các luật ở trên.

## Practice
1. Gửi `printf 'GET / HTTP/1.1\r\nHost: localhost\r\n\r\n' | nc localhost 80` (hoặc tới bất kỳ server local nào) và xem cả request lẫn reply qua `| xxd`. Chỉ ra từng CRLF và chuỗi `\r\n\r\n` kết thúc mỗi phần header.
2. Với mỗi request sau, chỉ dựa vào file này, ghi ra nó có hợp lệ không, và nếu không thì vi phạm luật nào và đáng nhận status gì: `get / HTTP/1.1`, `GET  / HTTP/1.1` (hai dấu cách), `GET / HTTP/1.1` với `Host : a`, không có `Host`, hai dòng `Host`, `Content-Length: +5`, `Content-Length: 5, 6`, `Transfer-Encoding: gzip`, `Transfer-Encoding: chunked, gzip`.
3. Tự decode bằng tay `4\r\nWiki\r\n5;x="a;b"\r\npedia\r\n0\r\nX-T: 1\r\n\r\n`: liệt kê size, extension, data của từng chunk, và trailer. Sau đó tìm lỗi trong `4\r\nWikiXX\r\n0\r\n\r\n`.
4. Gửi các request sai ở bài 2 tới hai server thật khác nhau (ví dụ nginx và một server dựa trên hyper) bằng `printf ... | nc`, và ghi lại chỗ câu trả lời của chúng khác nhau. Mỗi khác biệt là một chỗ mà proxy đứng trước một trong hai server có thể bị smuggle qua.
5. Trong [`labs/01-http-parser`](../../labs/01-http-parser), biến bài 2 và 3 thành một bảng test: byte đầu vào → kết quả parse mong đợi hoặc status code mong đợi. Viết bảng này *trước* khi viết parser.
6. Ghi quyết định của bạn về obs-fold và LF trơ vào README của [`labs/01-http-parser`](../../labs/01-http-parser), mỗi cái kèm luật từ file này và rủi ro smuggling mà nó tránh được hoặc chấp nhận.
