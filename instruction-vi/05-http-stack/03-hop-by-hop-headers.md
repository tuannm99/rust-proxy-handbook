# Hop-by-Hop Headers

Một số header mô tả connection giữa hai hop liền kề, không phải message
end-to-end. Một proxy forward chúng đang forward một phát biểu về một
connection không còn tồn tại — đó là một bug đúng nghĩa, và với các header
framing, còn là một lỗ hổng bảo mật.

## What to learn
### Tập header đó, và vì sao mỗi cái là per-hop
`Connection`, `Keep-Alive`, `Transfer-Encoding`, `TE`, `Trailer`,
`Upgrade`, `Proxy-Authorization`, `Proxy-Authenticate`.

Mỗi header mô tả điều gì đó về *link này*: connection này được framing như
thế nào, nó có giữ mở hay không, hop này được phép gửi gì, ai đã
authenticate với proxy này. Connection của bạn tới upstream có framing
riêng, trạng thái keep-alive riêng, trạng thái upgrade riêng — nên các
header này phải được sinh lại cho connection đó, không bao giờ được kế
thừa từ connection inbound.

### `Connection` còn nêu tên các header khác
`Connection` đặc biệt gấp đôi: ngoài việc tự nó là hop-by-hop, nó còn *nêu
tên* các header khác để coi là hop-by-hop. Một request mang
`Connection: X-Foo, Keep-Alive` đang nói "`X-Foo` chỉ dành cho bạn" — nên
bạn phải bỏ cả `X-Foo`, rồi mới bỏ chính `Connection`.

```rust
// strip the named headers first, then the standard hop-by-hop set
if let Some(conn) = req.headers().get(header::CONNECTION).cloned() {
    for name in conn.to_str().unwrap_or("").split(',') {
        req.headers_mut().remove(name.trim());
    }
}
for name in HOP_BY_HOP {          // Connection, Keep-Alive, TE, Trailer,
    req.headers_mut().remove(name); // Transfer-Encoding, Upgrade, Proxy-*
}
```

Gotcha: cơ chế đó nằm dưới quyền kiểm soát của attacker. Một client gửi
`Connection: Authorization` thuyết phục một proxy ngây thơ strip một header
mà một component downstream đang phụ thuộc vào — hoặc, theo chiều ngược
lại, strip một security header mà hạ tầng của chính bạn thêm vào. Cách
phòng thủ là thứ tự: strip header hop-by-hop trước, rồi mới áp header đáng
tin của chính bạn (phần identity injection của [`07-security/01-auth.md`](../07-security/01-auth.md)),
để không gì client nói ra có thể xóa chúng.

Gotcha: giá trị `Connection` là một danh sách phân tách bằng dấu phẩy do
peer viết — giới hạn số entry bạn xử lý và bỏ qua những cái dị dạng, thay
vì lặp qua một danh sách kích thước tùy attacker.

### Forward `Transfer-Encoding` là cách bạn tự tạo ra bug smuggling
Trong toàn bộ tập header đó, các header framing là những cái nguy hiểm.
Framing cho connection tới upstream của bạn phải **do chính bạn sinh ra**,
từ body bạn thực sự sắp gửi — không copy từ những gì đã đến.

Một proxy forward `Transfer-Encoding: chunked` nguyên vẹn trong khi viết ra
một body được framing khác đi đã tự tạo ra một mâu thuẫn giữa framing của
chính nó và những gì nó nói với upstream để mong đợi. Đó chính xác là
desync trong [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md), do tự mình gây ra.

Điều tương tự áp dụng khi bạn thay đổi body: nén response trong lúc
streaming ([`05-http-stack/07-compression.md`](07-compression.md)) làm `Content-Length` inbound
không còn đúng, và forward nó tiếp tục tạo ra cùng loại bug từ chiều ngược
lại.

### `Upgrade` là hop-by-hop, và WebSocket vẫn cần nó
`Connection: Upgrade` và `Upgrade: websocket` là per-hop theo định nghĩa —
client đang upgrade connection *của bạn*, và bạn riêng biệt upgrade
connection của mình tới upstream. Nên một implementation đúng strip chúng
đi rồi cố tình thêm lại cho leg upstream.

Gotcha: đây là cách phổ biến nhất khiến một proxy đang chạy tốt mất hỗ trợ
WebSocket trong một lần refactor — ai đó thêm việc strip hop-by-hop đúng
đắn và đường upgrade lặng lẽ ngừng hoạt động, vì không gì thêm lại các
header đó. [`05-http-stack/10-websocket.md`](10-websocket.md) bao quát luồng upgrade; việc xử
lý header là một trường hợp đặc biệt của file này.

### Các header end-to-end mà bạn vẫn không nên forward mù quáng
Hop-by-hop là một khái niệm trong spec; còn có một tập thứ hai, lớn hơn,
theo spec là end-to-end nhưng không nên vượt qua trust boundary của bạn:
- **Header identity** mà hạ tầng của bạn tin tưởng (`X-User-Id`,
  `X-Auth-*`) — [`07-security/01-auth.md`](../07-security/01-auth.md).
- **`X-Forwarded-For` / `Forwarded`** từ các peer không đáng tin —
  [`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md).
- **Bất cứ thứ gì bạn tự set** ở phía sau điểm này, mà nếu client set
  trước thì sẽ thắng.

Cả ba đều có chung một hình dạng: một header được tin tưởng vì "proxy set
nó", nên một client có thể set nó sẽ mạo danh proxy. Strip chúng ở cùng
một điểm sớm như tập hop-by-hop — một chỗ duy nhất, trước khi bất kỳ module
nào đọc bất cứ gì.

### Làm một lần, trước khi bất cứ gì đọc header
Đặt cả hai việc strip vào một stage duy nhất ở đầu pipeline
([`09-architecture/01-components.md`](../09-architecture/01-components.md)), trước routing, auth, WAF, hay
logging. Một lần strip xảy ra bên trong auth module không bảo vệ được các
route cấu hình `Public`; một lần strip xảy ra sau logging nghĩa là header
giả mạo xuất hiện trong log của bạn như thể chúng là thật.

## Practice
Làm theo thứ tự này.

1. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), implement việc strip hop-by-hop bao gồm
   cả các header được `Connection` nêu tên, như một stage ở đầu pipeline.
   **Xong khi** một request với `Connection: X-Secret` và một header
   `X-Secret` tới upstream mà không còn cả hai.
2. Sinh lại framing khi forward. **Xong khi** request mà proxy của bạn
   phát ra mang `Content-Length`/`Transfer-Encoding` mà nó tự tính từ body
   nó thực sự đang gửi — xác minh bằng cách kiểm tra raw byte, không phải
   đọc code.
3. Thử cuộc tấn công strip. **Xong khi** `Connection: Authorization` từ
   một client không xóa được header identity mà proxy của bạn thêm vào,
   vì header của bạn được áp sau khi strip.
4. Thêm việc strip identity và forwarded-for vào cùng một stage. **Xong
   khi** một `X-User-Id` giả mạo biến mất trên mọi đường đi — bao gồm cả
   route `Public` và đường 404.
5. Thêm lại header upgrade cho leg upstream. **Xong khi** một WebSocket
   vẫn hoạt động end-to-end sau bước 1 — đây là regression mà việc strip
   hop-by-hop kinh điển gây ra.
6. Fuzz header `Connection` ([`12-testing/02-fuzzing.md`](../12-testing/02-fuzzing.md)) với nhiều entry,
   entry rỗng, và tên dị dạng. **Xong khi** không có cái nào panic hay gây
   ra khối lượng công việc không giới hạn.
