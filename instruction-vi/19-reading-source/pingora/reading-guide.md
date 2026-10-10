# Reading guide: pingora

Một lộ trình đi qua pingora của Cloudflare, kèm các câu hỏi cần trả lời
trên đường đi. Đọc cái này sau khi [`proxy/`](../../../proxy) đã chạy được, như
[`19-reading-source/00-README.md`](../00-README.md) giải thích: mọi câu hỏi bên dưới thực chất là
"cái này so với thứ bạn đã build thế nào?", và nó không có câu trả lời cho
tới khi bạn đã build một cái. Đây là guide, không phải notes — các file
theo project trong [`00-README.md`](00-README.md) là để bạn tự viết.

Repository: `github.com/cloudflare/pingora`, một workspace gồm nhiều crate.
Path khớp với version tại thời điểm viết; nếu một path đã chuyển
chỗ, hãy search tên type hoặc trait được nêu.

## Các crate bạn sẽ ghé qua

- `pingora-core` — server, service, listener, xử lý protocol (`src/protocols/`), và connector ra ngoài (`src/connectors/`).
- `pingora-proxy` — logic HTTP proxy và trait `ProxyHttp` mà bạn implement để build một proxy.
- `pingora-load-balancing` — các giải thuật chọn upstream, health check, service discovery.
- `pingora-pool` — connection pool tới upstream.
- `pingora-cache`, `pingora-limits`, `pingora-timeout`, `pingora-error` — caching, ước lượng rate, timer, kiểu lỗi.

## Lộ trình

### Điểm dừng 1: điểm mở rộng: `ProxyHttp`
Bắt đầu trong `pingora-proxy` với trait `ProxyHttp` (file riêng của nó,
`proxy_trait.rs`) và đọc doc comment của từng method theo thứ tự. Chúng
được gọi ở những điểm cố định trong vòng đời một request — chọn upstream,
filter request, filter response, logging, xử lý lỗi connection.
- Vẽ thứ tự các callback này được gọi cho một request thành công. Middleware trong proxy của bạn ([`labs/14-plugin`](../../../labs/14-plugin)) lẽ ra chạy ở đâu trong thứ tự đó?
- Callback nào có thể kết thúc request sớm, và bằng cách nào?
- So với [`09-architecture/02-plugin.md`](../../09-architecture/02-plugin.md) và [`03-rust/18-async-traits.md`](../../03-rust/18-async-traits.md): pingora làm cho callback async hoạt động trên một trait thế nào, và cái giá là gì?

### Điểm dừng 2: một request từ đầu tới cuối
Theo một request HTTP/1 từ downstream, từ session phía server của
`pingora-core` (`src/protocols/http/v1/server.rs`) vào `pingora-proxy`
(đường HTTP/1, `proxy_h1.rs`), đi ra qua một connector tới upstream
(`src/protocols/http/v1/client.rs`), rồi quay lại.
- Body request và response được chuyển giữa hai connection thế nào — nguyên khối, hay stream từng chunk? Backpressure từ một client chậm sẽ block việc đọc từ upstream ở đâu?
- Connection tới upstream được trả về pool ở đâu, và trong điều kiện nào nó bị bỏ đi thay vì trả về?

### Điểm dừng 3: connection pool: `pingora-pool`
- Một connection trong pool được đánh key thế nào — chỉ theo địa chỉ, hay nhiều hơn? Vì sao TLS/SNI quan trọng với key?
- Pool phát hiện một connection rảnh trong pool đã bị upstream đóng trước khi dùng lại thế nào?
- So với thiết kế của bạn từ [`06-proxy/01-upstream.md`](../../06-proxy/01-upstream.md). Lựa chọn nào của pool lẽ ra đã cứu bạn khỏi một bug?

### Điểm dừng 4: load balancing và health check: `pingora-load-balancing`
- Có những giải thuật chọn upstream nào, và giải thuật nào từ [`labs/06-load-balancer`](../../../labs/06-load-balancer) bị thiếu?
- Implementation consistent hashing so với [`13-algorithms/consistent-hash.md`](../../13-algorithms/consistent-hash.md) thế nào?
- Kết quả health check quay lại ảnh hưởng việc chọn upstream thế nào, và so với [`06-proxy/03-healthcheck.md`](../../06-proxy/03-healthcheck.md) ra sao?

### Điểm dừng 5: chạy như một server
Trong `pingora-core`, tìm phần bootstrap server và đường graceful upgrade
(search việc chuyển listening socket giữa process cũ và process mới).
- Một process pingora mới tiếp quản listening socket từ process cũ mà không làm rớt connection thế nào?
- So với cách bạn làm [`09-architecture/04-graceful-shutdown.md`](../../09-architecture/04-graceful-shutdown.md) và [`09-architecture/05-rolling-restart.md`](../../09-architecture/05-rolling-restart.md). Cách của pingora cho bạn thứ gì mà drain-rồi-restart không cho?

### Điểm dừng 6: ước lượng rate: `pingora-limits`
- Bộ ước lượng rate của nó dùng cấu trúc dữ liệu gì, và nó liên quan tới [`13-algorithms/count-min-sketch.md`](../../13-algorithms/count-min-sketch.md) thế nào?
- Nó đánh đổi mất gì so với implementation trong [`labs/11-rate-limit`](../../../labs/11-rate-limit) của bạn?

## Viết gì vào notes
`architecture.md` nên có sơ đồ của riêng bạn về các crate và cách một
request đi qua chúng. `interesting-code.md` nên ghi lại, theo từng điểm
dừng, một lựa chọn thiết kế khác bạn nhất và bây giờ bạn có áp dụng nó
không. File này là phần thưởng của cả handbook — nó ghi càng trung thực
chỗ proxy của bạn còn thua kém, nó càng có giá trị.
