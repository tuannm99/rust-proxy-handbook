# HTTP

File này nói về ngữ nghĩa HTTP theo RFC 9110: hình dạng của một message, method,
status code, header, các loại state mà HTTP mang theo (cookie, validator), và những quy
tắc mà proxy phải tuân thủ.

## What to learn

### Ngữ nghĩa theo RFC 9110, và cách chia theo version
RFC 9110 định nghĩa ngữ nghĩa HTTP, độc lập với version. (RFC 9112 mô tả wire format của
/1.1, 9113 của /2, 9114 của /3.) Đây là nơi định nghĩa "GET thực sự hứa điều gì" (safe,
idempotent, cacheable) và "một mã 3xx nghĩa là gì".

Hãy xem nó như bản hợp đồng mà proxy của bạn không được vi phạm khi rewrite hay forward
một request. Ví dụ: tự động retry một POST khi server chưa từng cho biết nó idempotent là
vi phạm hợp đồng.

Các version khác nhau ở *cách đóng gói byte* (framing), không khác ở ý nghĩa của một
request. Một proxy thường nhận HTTP/2 từ client và nói HTTP/1.1 với upstream. Nó dịch qua
lại giữa các kiểu framing, nhưng giữ nguyên ngữ nghĩa ([`17-http2.md`](17-http2.md)).

### Một message gồm start line, header, và body (tùy chọn)
```text
GET /search?q=rust HTTP/1.1\r\n             <- request line: method, target, version
Host: example.com\r\n                       <- header: Name: value
Accept: text/html\r\n
\r\n                                        <- dòng trống kết thúc phần header
(body, nếu có)
```
```text
HTTP/1.1 200 OK\r\n                         <- status line: version, code, reason
Content-Type: text/html\r\n
Content-Length: 1234\r\n
\r\n
<1234 byte>
```
Tên header không phân biệt hoa thường. Giá trị của header có thứ tự và có thể lặp lại.

Cách xác định ranh giới của body: dùng `Content-Length`, dùng chunked coding, hoặc (với
response) đóng connection. Quy tắc chính xác và thứ tự ưu tiên của chúng nằm ở
[`16-http1-wire-format.md`](16-http1-wire-format.md).

HTTP là **stateless**: mỗi request mang đủ mọi thứ cần thiết để được xử lý. Nhờ vậy proxy
có thể gửi các request liên tiếp tới những backend khác nhau. Những state có tồn tại
(session, đăng nhập) được mang trong các header như `Cookie` và `Authorization`, ở phần
bên dưới.

### Target, `Host` và virtual hosting
*Target* của request có bốn dạng:
- **origin-form** (`/search?q=rust`): dạng bình thường, gửi tới server.
- **absolute-form** (`GET http://example.com/x`): client gửi tới một **forward** proxy.
- **authority-form** (`CONNECT example.com:443`): chỉ dùng với `CONNECT`.
- **asterisk-form** (`OPTIONS *`): nhắm tới cả server.

Một URL gồm `scheme://host:port/path?query#fragment`. **Fragment không bao giờ được gửi
tới server.**

Nhiều site dùng chung một IP và port. Vì vậy HTTP/1.1 *bắt buộc* có header `Host` (HTTP/2
dùng `:authority`) để nói rõ request dành cho site nào. Đó là **virtual hosting**, và là
cơ sở của [`05-http-stack/12-vhost-routing.md`](../05-http-stack/12-vhost-routing.md).
Request không có `Host`, hoặc có hai `Host` mâu thuẫn nhau, phải bị từ chối.

Reverse proxy chọn route dựa vào `Host` + path. Nó cũng quyết định gửi `Host` nào tới
upstream: giữ nguyên `Host` của client, hay ghi đè bằng `Host` của backend. Backend
thường phụ thuộc vào lựa chọn này.

### Method, safety và idempotency
`GET`/`HEAD`/`OPTIONS` là **safe**: theo hợp đồng, chúng chỉ đọc, không thay đổi gì.
`GET`/`HEAD`/`PUT`/`DELETE`/`OPTIONS` là **idempotent**: gọi lặp lại cho kết quả giống
như gọi một lần. `POST`/`PATCH` thường thì không.

Đây là yếu tố quyết định logic retry của proxy
([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)) có được tự động áp dụng an toàn hay
không, hay phải yêu cầu opt-in tường minh/idempotency key.

`CONNECT` yêu cầu proxy mở một TCP tunnel thô tới `host:port`, rồi relay byte mà không
đọc nội dung. Đó là cách HTTPS đi qua forward proxy: proxy chỉ thấy đích đến, không thấy
nội dung.

`Upgrade` chuyển connection sang một protocol khác (WebSocket,
[`05-http-stack/10-websocket.md`](../05-http-stack/10-websocket.md)).

### Status code: các nhóm, và những mã dễ gây lỗi
`1xx` là thông tin (`100 Continue`, `101 Switching Protocols`), `2xx` là thành công, `3xx`
là redirect, `4xx` là lỗi do client, `5xx` là lỗi do server. Các mã cần biết chính xác:

- `200` OK. `201` Created. `204` No Content (không bao giờ có body). `206` Partial
  Content (trả lời một request có `Range`).
- **Redirect**: `301`/`302` không rõ ràng về việc `POST` có bị đổi thành `GET` hay không.
  Hai mã chính xác là `307` (tạm thời) và `308` (vĩnh viễn), cả hai **giữ nguyên method
  và body**. `304 Not Modified` không phải redirect. Nó nghĩa là "bản cache của bạn vẫn
  dùng được".
- `400`: request sai định dạng. `401`: chưa xác thực (cần gửi credential). `403`: đã xác
  thực nhưng bị cấm. `404`: không tìm thấy. `408`: request gửi tới quá chậm. `413`: body
  quá lớn. `414`: URI quá dài. `431`: header quá lớn. `429`: quá nhiều request.
- `500`: bug của server. `501`: chưa implement. `502`/`503`/`504`: xem bên dưới.

Phần lớn status code mà proxy *tự sinh ra* nói về chính lớp proxy, không phải về
backend:

- `502 Bad Gateway`: không kết nối được upstream, hoặc upstream trả response không hợp
  lệ.
- `503 Service Unavailable`: không có upstream nào healthy, hoặc proxy chủ động
  load-shedding.
- `504 Gateway Timeout`: upstream quá chậm.
- `429 Too Many Requests`: bị rate limit (xem
  [`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)).

Trả `500` cho các trường hợp này là lỗi người mới hay mắc. Nó che mất việc lỗi nằm ở
proxy hay ở backend.

Status `204`, `304` và mọi response cho `HEAD` đều không có body, dù có `Content-Length`.
Nếu proxy cứ chờ một body sẽ không bao giờ đến, connection sẽ bị treo.

### Validator, conditional request và range
Một response có thể mang **validator**: `ETag` (một tag phiên bản, nội dung không có ý
nghĩa với client) và/hoặc `Last-Modified`. Client muốn kiểm tra lại (revalidate) thì gửi
`If-None-Match: "<etag>"` hoặc `If-Modified-Since`. Nếu nội dung không đổi, server trả
`304` không kèm body, tiết kiệm được dữ liệu truyền. Cơ chế tương tự cũng bảo vệ việc
ghi: `If-Match` ngăn lost update.

`Cache-Control` (`max-age`, `no-store`, `private`, ...) cho biết response được dùng lại
trong bao lâu và bởi ai. Nó là toàn bộ luật chơi của một proxy cache
([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)).

`Range: bytes=0-999` yêu cầu một phần của resource. Server trả `206` kèm `Content-Range`.
Nhờ đó có thể tải tiếp file đang dở và tua video
([`05-http-stack/06-static.md`](../05-http-stack/06-static.md)).

`Vary` liệt kê những request header làm response thay đổi (ví dụ `Accept-Encoding`). Phải
tôn trọng nó, nếu không cache sẽ trả nhầm biến thể.

### Content negotiation và encoding
`Accept`/`Accept-Language`/`Accept-Encoding` cho client nêu ưu tiên của mình.
`Content-Type` (kèm `charset`) và `Content-Encoding: gzip|br|zstd` mô tả body
([`05-http-stack/07-compression.md`](../05-http-stack/07-compression.md)).

**`Content-Encoding` là end-to-end**: body được nén nguyên khối và chỉ client cuối giải
nén. **`Transfer-Encoding` là hop-by-hop**: chỉ áp dụng cho connection hiện tại. Nhầm hai
loại này là cách để proxy nén body hai lần hoặc làm hỏng body.

### State trên một protocol stateless: cookie và credential
Server set `Set-Cookie: sid=abc; HttpOnly; Secure; SameSite=Lax; Max-Age=3600`. Client
gửi lại `Cookie: sid=abc` ở các request khớp.

Proxy phải forward nhiều header `Set-Cookie` *riêng rẽ*. Không được nối chúng bằng dấu
phẩy. Đây là header duy nhất phá quy tắc "các header lặp lại gộp được thành một danh sách
phân cách bằng dấu phẩy".

`Authorization: Bearer <token>` hoặc `Basic <base64>` mang credential
([`07-security/01-auth.md`](../07-security/01-auth.md)). `WWW-Authenticate` là challenge đi
kèm mã `401`.

Một caching proxy không bao giờ được trả response đã cá nhân hóa theo `Authorization` hay
`Cookie` cho user khác ([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)).

### Header hop-by-hop và end-to-end
`Connection`, `Keep-Alive`, `Transfer-Encoding`, `TE`, `Upgrade`, `Proxy-Authenticate` và
`Proxy-Authorization` là hop-by-hop. Proxy phải bỏ chúng đi và tạo lại cho từng hop, không
được forward nguyên xi. Proxy cũng phải bỏ mọi header được *nêu tên* trong `Connection:`.

Mọi header khác là end-to-end, và nên đi qua gần như nguyên vẹn (ngoài việc thêm
`X-Forwarded-*`/`Forwarded`/`Via`). Forward nguyên một hop-by-hop header tới hop kế tiếp là
bug proxy kinh điển
([`05-http-stack/03-hop-by-hop-headers.md`](../05-http-stack/03-hop-by-hop-headers.md)).

```rust
const HOP_BY_HOP: &[&str] = &[
    "connection", "keep-alive", "proxy-authenticate",
    "proxy-authorization", "te", "trailers",
    "transfer-encoding", "upgrade",
];
```

### Cho backend biết client là ai
Khi proxy kết thúc connection của client, backend chỉ thấy IP của *proxy*. Các header sau
chuyển thông tin về client đi tiếp:
- `X-Forwarded-For: client, proxy1` (de-facto) và `Forwarded: for=...;proto=https`
  (chuẩn).
- `X-Forwarded-Proto`/`-Host` mang scheme và host gốc.

Đây là các header thông thường nên **ai cũng giả mạo được**. Proxy nên *nối thêm* vào một
chuỗi đáng tin. Khi header đến từ client không đáng tin, proxy phải **ghi đè hoặc bỏ nó**.
Nếu không, client có thể khai bất kỳ IP nào để qua mặt IP filtering hay rate limit
([`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md)).

Ở mức TCP, PROXY protocol giải quyết cùng bài toán này
([`20-proxy-protocol.md`](20-proxy-protocol.md)).

### Chunked transfer encoding và message framing
Khi chưa biết trước `Content-Length`, `Transfer-Encoding: chunked` chia body thành các
chunk dạng `<hex-size>\r\n<data>\r\n`, kết thúc bằng một chunk có kích thước 0.

Một message không được vừa có `Content-Length` vừa có `Transfer-Encoding: chunked`. RFC
9112 quy định bên nhận phải từ chối hoặc normalize sự mơ hồ này (grammar chính xác và thứ
tự các quy tắc nằm ở [`16-http1-wire-format.md`](16-http1-wire-format.md)).

Các cuộc tấn công request smuggling khai thác đúng sự mơ hồ này, khi parser ở front-end và
back-end không thống nhất header nào được ưu tiên. Xem
[`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md).

### `Expect: 100-continue` và response sớm
Client sắp gửi một body lớn có thể gửi `Expect: 100-continue` rồi chờ một interim
`100 Continue` trước khi truyền. Nhờ vậy server có thể từ chối ngay dựa trên header
(`401`, `413`), mà không phải nhận hàng gigabyte dữ liệu.

Proxy phải forward đúng handshake `Expect` (hoặc tự trả `100`). Proxy cũng phải sẵn sàng
cho trường hợp server phản hồi **trước khi đọc xong request body**. Đừng giả định "đọc
xong request rồi mới ghi response".

## Practice

1. Dùng `curl -v` vào một server thật. Từ output wire thô, xác định: header response nào
   là hop-by-hop, header nào là end-to-end, đâu là request line/status line, khối header
   và ranh giới body. Sau đó `printf` cùng request đó vào `nc` bằng tay và so sánh.
2. Gửi một request có cả `Content-Length` lẫn `Transfer-Encoding: chunked` tới một test
   server do bạn kiểm soát. Quan sát xem nó bị từ chối hay không. Thử với nhiều hơn một
   HTTP library.
3. Chọn một static-file server bất kỳ có hỗ trợ validator và range (nginx, hoặc một file
   phục vụ từ CDN công cộng). Tải file bằng `curl -sv -o /dev/null` và copy `ETag` của nó.
   Lặp lại với `-H 'If-None-Match: "<etag>"'` để nhận `304`. Dùng `curl -r 0-99` để nhận
   `206`. Đọc `Content-Range` và ghi lại kích thước body.
4. Chạy `curl -v --http1.1 http://example.com/` và một request `HEAD` cho cùng URL. So sánh
   header, và xác nhận response của `HEAD` có `Content-Length` nhưng không có body.
5. Trong [`labs/02-http-server`](../../labs/02-http-server), implement việc bỏ hop-by-hop
   header đúng cho cả chiều request lẫn response. Từ chối các request không có `Host`, hoặc
   có nhiều header `Host` mâu thuẫn nhau.
6. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), trả `502`/`503`/`504` phân
   biệt cho ba trường hợp: "upstream từ chối connection", "không có upstream healthy", và
   "upstream timeout". Nối thêm `X-Forwarded-For`, đồng thời bỏ header giả mạo đến từ bên
   ngoài. Đọc [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) và
   [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md) để nối
   phần thảo luận về framing với cách một parser phải thực thi nó.
