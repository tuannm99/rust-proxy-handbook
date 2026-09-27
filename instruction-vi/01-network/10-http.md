# HTTP

RFC 9110, method, status code, header, chunked encoding.

## What to learn

### Ngữ nghĩa RFC 9110
RFC 9110 định nghĩa ngữ nghĩa HTTP độc lập với version (9112 cho wire
format của /1.1, 9113 cho /2, 9114 cho /3). Đây là nơi "GET thực sự hứa
hẹn điều gì" (safe, idempotent, cacheable) và "3xx nghĩa là gì" được định
nghĩa. Đọc nó như một hợp đồng mà proxy của bạn không được vi phạm khi nó
rewrite hoặc forward một request — ví dụ forward một retry của POST khi
server chưa bao giờ xác nhận idempotency là vi phạm hợp đồng.

### Method và idempotency
GET/HEAD/PUT/DELETE là idempotent (lặp lại có cùng hiệu ứng như làm một
lần); POST/PATCH nhìn chung thì không. Đây là yếu tố quyết định liệu logic
retry của proxy bạn ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)) có an toàn để áp dụng tự động
hay cần một cơ chế opt-in tường minh/idempotency key.

### Status code mà một proxy thực sự tạo ra
Phần lớn status code một proxy trả về là về chính tầng proxy, không phải
về backend: `502 Bad Gateway` (upstream không thể tiếp cận/response
không hợp lệ), `503 Service Unavailable` (không có upstream khỏe mạnh,
hoặc load-shedding có chủ đích), `504 Gateway Timeout` (upstream quá
chậm), `429 Too Many Requests` (rate limit, xem
[`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)). Trả về `500` cho những trường hợp này là
một lỗi phổ biến của người mới — nó che giấu việc lỗi là từ proxy của bạn
hay từ backend.

### Header: hop-by-hop vs end-to-end
`Connection`, `Keep-Alive`, `Transfer-Encoding`, `TE`, `Upgrade` là
hop-by-hop — một proxy phải bóc/tạo lại chúng ở mỗi hop, không bao giờ
forward mù quáng. Mọi thứ khác là end-to-end và nên đi qua gần như nguyên
vẹn (trừ việc thêm `X-Forwarded-*`/`Forwarded`). Forward một header
hop-by-hop nguyên văn tới hop tiếp theo là một bug proxy kinh điển.

```rust
const HOP_BY_HOP: &[&str] = &[
    "connection", "keep-alive", "proxy-authenticate",
    "proxy-authorization", "te", "trailers",
    "transfer-encoding", "upgrade",
];
```

### Chunked transfer encoding và framing của message
Khi `Content-Length` không được biết trước, `Transfer-Encoding: chunked`
frame body thành một chuỗi các chunk `<hex-size>\r\n<data>\r\n` kết thúc
bằng một chunk kích thước 0. Một message không được vừa chỉ định
`Content-Length` vừa `Transfer-Encoding: chunked` — RFC 9112 nói rằng bên
nhận phải từ chối hoặc chuẩn hóa sự mập mờ đó. Đây chính xác là sự mập mờ
mà các tấn công request-smuggling khai thác khi một parser front-end và
back-end bất đồng về header nào thắng — xem
[`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md).

## Practice

1. Dùng `curl -v` nhắm vào một server thật và xác định header response
   nào là hop-by-hop vs end-to-end từ output thô trên đường truyền.
2. Gửi một request có cả `Content-Length` và
   `Transfer-Encoding: chunked` tới một test server bạn kiểm soát và quan
   sát nó bị từ chối như thế nào (hoặc không — thử nhiều hơn một HTTP
   library).
3. Trong [`labs/02-http-server`](../../labs/02-http-server), implement việc bóc header hop-by-hop
   đúng cách cho cả request lẫn response.
4. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), trả về `502`/`503`/`504` khác nhau
   tương ứng cho "upstream từ chối kết nối", "không có upstream khỏe
   mạnh", và "upstream timeout".
5. Đọc [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) và [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md)
   để nối phần thảo luận về framing của file này với việc một parser phải
   thực thi nó ra sao.
