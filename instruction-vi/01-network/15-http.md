# HTTP

Ngữ nghĩa RFC 9110: hình dạng của một message, method, status code, header,
state mà HTTP mang (cookie, validator), và các quy tắc proxy phải giữ.

## What to learn

### Ngữ nghĩa RFC 9110, và sự phân chia theo version
RFC 9110 định nghĩa ngữ nghĩa HTTP độc lập với version (9112 cho wire format
/1.1, 9113 cho /2, 9114 cho /3). Đó là nơi "GET thực sự hứa gì" (safe,
idempotent, cacheable) và "một 3xx nghĩa là gì" nằm. Hãy đọc nó như bản hợp đồng
mà proxy của bạn không được vi phạm khi rewrite hay forward một request — ví dụ
forward retry của một POST khi server chưa bao giờ xác nhận idempotency là vi
phạm hợp đồng. Các version khác nhau ở *cách đóng khung byte*, không phải ở
nghĩa của một request; một proxy thường xuyên nhận HTTP/2 từ client và nói
HTTP/1.1 với upstream, dịch giữa các framing trong khi giữ nguyên ngữ nghĩa
([`17-http2.md`](17-http2.md)).

### Một message là start line, header, và body tùy chọn
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
Header là tên không phân biệt hoa thường với các value có thứ tự, có thể lặp.
Body được phân ranh bằng `Content-Length`, bằng chunked coding, hoặc — với
response — bằng việc đóng connection ([`16-http1-wire-format.md`](16-http1-wire-format.md) có quy tắc chính
xác và thứ tự của chúng). HTTP là **stateless**: mỗi request mang mọi thứ cần
để trả lời nó, đó là điều cho phép proxy gửi các request liên tiếp tới các
backend khác nhau. State có tồn tại (session, đăng nhập) được mang trong các
header như `Cookie` và `Authorization`, bên dưới.

### Target, `Host`, và virtual hosting
*Target* của request có bốn dạng. **origin-form** (`/search?q=rust`) là dạng bình
thường gửi tới server. **absolute-form** (`GET http://example.com/x`) là thứ
client gửi tới một **forward** proxy. **authority-form** (`CONNECT example.com:443`)
chỉ dùng bởi `CONNECT`, và **asterisk-form** (`OPTIONS *`) nhắm tới cả server.
Một URL tách thành `scheme://host:port/path?query#fragment`; **fragment không
bao giờ tới server**.

Vì nhiều site dùng chung một IP và port, HTTP/1.1 *bắt buộc* có header `Host`
(HTTP/2 dùng `:authority`) nêu rõ request dành cho site nào — đó là **virtual
hosting**, và là cơ sở của [`05-http-stack/12-vhost-routing.md`](../05-http-stack/12-vhost-routing.md). Request không có
`Host`, hoặc có hai cái mâu thuẫn, phải bị từ chối. Reverse proxy quyết định
routing từ `Host` + path, và quyết định `Host` nào gửi tới upstream (giữ của
client, hay ghi đè bằng của backend) — một lựa chọn mà backend thường phụ thuộc
vào.

### Method, safety, và idempotency
`GET`/`HEAD`/`OPTIONS` là **safe** (read-only theo hợp đồng).
`GET`/`HEAD`/`PUT`/`DELETE`/`OPTIONS` là **idempotent** (lặp lại có cùng hiệu
ứng với làm một lần); `POST`/`PATCH` thường thì không. Đây là yếu tố quyết định
liệu logic retry của proxy ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)) có an toàn để áp dụng tự động
hay cần opt-in tường minh/idempotency key. `CONNECT` yêu cầu proxy mở một TCP
tunnel thô tới `host:port` rồi relay byte một cách mù quáng (cách HTTPS đi qua
forward proxy; proxy chỉ thấy đích, không thấy nội dung). `Upgrade` chuyển
connection sang protocol khác (WebSocket, [`05-http-stack/10-websocket.md`](../05-http-stack/10-websocket.md)).

### Status code: các lớp, và những cái dễ cắn
`1xx` thông tin (`100 Continue`, `101 Switching Protocols`), `2xx` thành công,
`3xx` redirect, `4xx` client sai, `5xx` server sai. Đáng biết chính xác:

- `200` OK; `201` Created; `204` No Content (không bao giờ có body); `206`
  Partial Content (trả lời một request `Range`).
- **Redirect**: `301`/`302` mơ hồ về việc `POST` có biến thành `GET` không; các
  mã chính xác là `307` (tạm thời) và `308` (vĩnh viễn), **giữ nguyên method và
  body**. `304 Not Modified` không phải redirect — nó nghĩa là "bản cache của
  bạn vẫn dùng được."
- `400` request sai định dạng; `401` chưa xác thực (gửi credential); `403` đã xác
  thực nhưng bị cấm; `404`; `408` request tới quá chậm; `413` body quá lớn; `414`
  URI quá dài; `431` header quá lớn; `429` quá nhiều request.
- `500` bug server; `501` chưa implement; `502`/`503`/`504` bên dưới.

Hầu hết status code mà proxy *sinh ra* là về chính lớp proxy, không phải
backend: `502 Bad Gateway` (upstream không tới được/response không hợp lệ),
`503 Service Unavailable` (không có upstream healthy, hoặc chủ động load-shedding),
`504 Gateway Timeout` (upstream quá chậm), `429 Too Many Requests` (rate limit,
xem [`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)). Trả `500` cho những trường hợp này là lỗi
người mới phổ biến — nó giấu việc lỗi là của proxy hay của backend. Status `204`,
`304` và mọi response cho `HEAD` không mang body dù có `Content-Length` — một proxy
chờ một body sẽ không bao giờ tới sẽ làm treo connection.

### Validator, conditional request, và range
Một response có thể mang **validator**: `ETag` (một version tag mờ đục) và/hoặc
`Last-Modified`. Client revalidate gửi `If-None-Match: "<etag>"` hoặc
`If-Modified-Since`; nếu không đổi, server trả `304` không có body, tiết kiệm
việc truyền. Cơ chế tương tự bảo vệ việc ghi (`If-Match` chặn lost update).
`Cache-Control` (`max-age`, `no-store`, `private`, ...) nói response được dùng lại
bao lâu và bởi ai — toàn bộ luật chơi của một proxy cache
([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)). `Range: bytes=0-999` xin một phần của resource;
server reply `206` kèm `Content-Range`, cho phép tải tiếp và tua video
([`05-http-stack/06-static.md`](../05-http-stack/06-static.md)). `Vary` liệt kê các request header đã làm thay đổi
response (ví dụ `Accept-Encoding`), và phải được tôn trọng nếu không cache sẽ
phục vụ sai biến thể.

### Content negotiation và encoding
`Accept`/`Accept-Language`/`Accept-Encoding` cho client nêu ưu tiên;
`Content-Type` (kèm `charset`) và `Content-Encoding: gzip|br|zstd` mô tả body
([`05-http-stack/07-compression.md`](../05-http-stack/07-compression.md)). **`Content-Encoding` là end-to-end** (body được
nén nguyên khối và được client cuối giải nén), trong khi **`Transfer-Encoding` là
hop-by-hop** (chỉ áp dụng cho connection này). Nhầm lẫn chúng là cách một proxy
nén hai lần hoặc làm hỏng body.

### State trên một protocol stateless: cookie và credential
Server set `Set-Cookie: sid=abc; HttpOnly; Secure; SameSite=Lax; Max-Age=3600`;
client trả `Cookie: sid=abc` ở các request khớp. Proxy phải forward nhiều header
`Set-Cookie` *riêng rẽ* (không thể nối bằng dấu phẩy — header duy nhất phá luật
"header lặp gộp thành danh sách phân cách bằng phẩy"). `Authorization: Bearer
<token>` hoặc `Basic <base64>` mang credential ([`07-security/01-auth.md`](../07-security/01-auth.md));
`WWW-Authenticate` là challenge của `401`. Một caching proxy không bao giờ được
phục vụ response cá nhân hóa theo `Authorization` hay `Cookie` cho user khác
([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)).

### Header: hop-by-hop vs end-to-end
`Connection`, `Keep-Alive`, `Transfer-Encoding`, `TE`, `Upgrade`,
`Proxy-Authenticate` và `Proxy-Authorization` là hop-by-hop — proxy phải
strip/tạo lại chúng theo từng hop, không bao giờ forward mù quáng, và cũng phải
strip mọi header được *nêu tên* trong `Connection:`. Mọi thứ khác là end-to-end và
nên đi qua gần như nguyên vẹn (ngoài việc thêm `X-Forwarded-*`/`Forwarded`/`Via`).
Forward nguyên văn một hop-by-hop header tới hop kế là bug proxy kinh điển
([`05-http-stack/03-hop-by-hop-headers.md`](../05-http-stack/03-hop-by-hop-headers.md)).

```rust
const HOP_BY_HOP: &[&str] = &[
    "connection", "keep-alive", "proxy-authenticate",
    "proxy-authorization", "te", "trailers",
    "transfer-encoding", "upgrade",
];
```

### Báo cho backend biết client là ai
Khi proxy kết thúc connection, backend thấy IP của *proxy*.
`X-Forwarded-For: client, proxy1` (de-facto) và `Forwarded: for=...;proto=https`
(chuẩn) chuyển nó đi; `X-Forwarded-Proto`/`-Host` mang scheme và host gốc. Đây là
các header thường mà **ai cũng giả mạo được**: proxy nên *nối thêm* vào một chuỗi
tin cậy, và **ghi đè hoặc bỏ** header khi nó đến từ client không tin cậy, nếu
không client có thể khai bất kỳ IP nào để vượt IP filtering hay rate limit
([`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md)). Ở mức TCP, PROXY protocol giải cùng bài toán
([`20-proxy-protocol.md`](20-proxy-protocol.md)).

### Chunked transfer encoding và message framing
Khi `Content-Length` không biết trước, `Transfer-Encoding: chunked` đóng khung body
thành chuỗi các chunk `<hex-size>\r\n<data>\r\n` kết thúc bằng một chunk kích thước
0. Một message không được chỉ định đồng thời `Content-Length` và
`Transfer-Encoding: chunked` — RFC 9112 nói bên nhận phải từ chối hoặc normalize
sự mơ hồ đó (grammar chính xác và thứ tự quy tắc nằm ở
[`16-http1-wire-format.md`](16-http1-wire-format.md)). Đây chính là sự mơ hồ mà các cuộc tấn công request
smuggling khai thác khi parser front-end và back-end bất đồng về header nào thắng
— xem [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md).

### `Expect: 100-continue` và response sớm
Một client sắp gửi body lớn có thể gửi `Expect: 100-continue` và chờ một interim
`100 Continue` trước khi truyền, để server có thể từ chối chỉ dựa trên header
(`401`, `413`) mà không phải nhận hàng gigabyte. Proxy phải forward handshake
`Expect` đúng cách (hoặc tự trả `100`) và sẵn sàng cho việc server phản hồi
**trước khi request body được đọc hết** — không được giả định "đọc request, rồi
mới ghi response".

## Practice

1. Dùng `curl -v` vào một server thật và xác định, từ output wire thô, header
   response nào là hop-by-hop vs end-to-end, cùng request/status line, khối
   header và ranh giới body. Sau đó `printf` cùng request đó vào `nc` bằng tay và
   so sánh.
2. Gửi một request có cả `Content-Length` và `Transfer-Encoding: chunked` tới một
   test server bạn kiểm soát và quan sát cách nó bị từ chối (hoặc không — thử
   nhiều hơn một HTTP library).
3. Với một static-file server bất kỳ hỗ trợ validator và range (nginx, hoặc một
   file phục vụ từ CDN công cộng), tải file bằng `curl -sv -o /dev/null`, copy
   `ETag` của nó, rồi lặp lại với `-H 'If-None-Match: "<etag>"'` để nhận `304`, và
   với `curl -r 0-99` để nhận `206`; đọc `Content-Range` và ghi lại kích thước
   body.
4. Chạy `curl -v --http1.1 http://example.com/` và một `HEAD` cho cùng URL; so
   sánh header, và xác nhận response `HEAD` có `Content-Length` nhưng không có
   body.
5. Trong [`labs/02-http-server`](../../labs/02-http-server), implement việc strip hop-by-hop header đúng cho
   cả chiều request và response, và từ chối request không có `Host` hoặc có các
   header `Host` trùng lặp mâu thuẫn.
6. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), trả `502`/`503`/`504` phân biệt cho
   "upstream từ chối connection", "không có upstream healthy", và "upstream
   timeout" tương ứng, nối thêm `X-Forwarded-For` trong khi bỏ cái giả mạo đến từ
   ngoài, và đọc [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) cùng
   [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md) để nối phần thảo luận về framing với
   cách một parser phải enforce nó.
