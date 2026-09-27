# gzip, brotli, zstd

## What to learn

### Đánh đổi giữa các thuật toán
gzip (DEFLATE) được hỗ trợ phổ biến và decode rẻ nhưng có tỉ lệ nén yếu nhất trong ba cái. Brotli nhìn chung nén text/HTML tốt nhất và được browser hỗ trợ tốt nhưng encode chậm hơn ở các mức chất lượng cao. zstd nén và giải nén rất nhanh với tỉ lệ hợp lý và ngày càng được dùng cho traffic nội bộ/east-west nơi CPU quan trọng hơn vài phần trăm kích thước cuối cùng; hỗ trợ `Content-Encoding: zstd` trên browser vẫn chưa phổ biến, nên nó hữu ích cho proxy-tới-upstream hơn là proxy-tới-browser.

Gotcha: mức chất lượng chi phối toàn bộ so sánh này, và các mức cao nhất
là cái bẫy cho nội dung động. Brotli quality 11 có thể chậm hơn một bậc
độ lớn so với quality 4-5 để đổi lấy vài phần trăm tỉ lệ tốt hơn — ổn khi
bạn nén một lần lúc build rồi serve mãi mãi (asset content-hashed của
[`05-http-stack/05-static.md`](05-static.md)), không bao giờ đáng khi nén một response
theo từng request. Dùng chất lượng cao cho file tĩnh nén sẵn, thấp-tới-vừa
cho bất cứ gì động.

### Negotiation qua Accept-Encoding
Client liệt kê những gì nó có thể decode, tùy chọn kèm trọng số chất lượng: `Accept-Encoding: gzip, br;q=0.8`. Server (hoặc proxy) chọn một cái nó hỗ trợ, set `Content-Encoding` trên response, và phải thêm `Vary: Accept-Encoding` để bất kỳ cache nào phía trước nó (xem [`05-http-stack/07-cache.md`](07-cache.md)) không serve một response gzip cho một client chỉ yêu cầu brotli.

Gotcha: xử lý các trường hợp biên của q-value, vì đó là chỗ negotiation
tinh vi sai lệch. `q=0` nghĩa là *không* chấp nhận tường minh, không phải
"ưu tiên thấp nhất" — `gzip;q=0` là một sự từ chối. `identity;q=0` nghĩa
là client từ chối không nén, và `*;q=0` từ chối mọi thứ chưa được liệt kê
ở nơi khác; nếu bạn không thể đáp ứng request đó, bạn được kỳ vọng trả về
`406`, dù trong thực tế cứ serve `identity` là lựa chọn thực dụng phổ biến.

Gotcha: quên `Vary: Accept-Encoding` là một bug về đúng đắn chỉ xuất hiện
khi có một cache tham gia — và khi đó nó serve byte brotli cho một client
chỉ decode được gzip, biểu hiện như một sự hỏng dữ liệu bí ẩn thay vì một
lỗi rõ ràng.

### Những gì không nên nén
Compression có một sàn và một trần, cả hai đều đáng để áp dụng:
- **Định dạng đã nén sẵn** — JPEG, PNG, WebP, MP4, zip, và bất cứ gì
  upstream đã encode sẵn — gần như không lợi gì và tốn CPU đầy đủ. Gate
  theo `Content-Type`, không chỉ theo size.
- **Body quá nhỏ.** Dưới khoảng 1 KB, overhead framing của gzip có thể
  làm response *lớn hơn*, và CPU là lãng phí thuần túy.
- **Dữ liệu đã mã hóa hoặc ngẫu nhiên** nén ra kích thước hơi lớn hơn ban
  đầu, vì cùng một lý do.

### Streaming so với buffer-rồi-nén
Nén toàn bộ response trong memory trước khi gửi thêm latency (client chờ toàn bộ body) và áp lực memory với body lớn. Streaming compression (API `Compress` của `flate2`, các wrapper stream của `async-compression`) nén từng chunk khi chúng được sinh ra/forward, điều rất quan trọng trong một proxy đang relay một body lớn từ upstream.

```rust
// sketch: streaming gzip over an async body using async-compression
use async_compression::tokio::bufread::GzipEncoder;
let compressed = GzipEncoder::new(upstream_body_reader);
// compressed implements AsyncRead; forward it straight to the client socket
```

Gotcha: một khi bạn nén trong lúc streaming, **`Content-Length` của
upstream trở nên sai** — nó mô tả body chưa nén, và bạn không biết độ dài
đã nén cho tới khi xong. Bạn phải bỏ `Content-Length` và chuyển sang
chunked transfer encoding (HTTP/1.1) hoặc dựa vào độ dài frame (HTTP/2).
Forward một `Content-Length` đã cũ cùng với một body đã nén chính xác là
sự bất đồng về framing mà [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md) nói tới —
đây là một trong những cách phổ biến nhất một proxy vô tình tự tạo ra nó.

Gotcha: flush là một đánh đổi latency/tỉ lệ. Một compressor không bao giờ
flush đệm dữ liệu để có tỉ lệ tốt hơn, điều này làm đứng các response
streaming (SSE, long-poll, các pattern gần với
[`05-http-stack/09-websocket.md`](09-websocket.md)) — client chờ output đang nằm trong
compressor. Flush ở các ranh giới có ý nghĩa cho các content type
streaming; đừng làm vậy với download hàng loạt.

### Chi phí CPU dưới tải
Compression tốn CPU; ở tốc độ request cao, encode mỗi request có thể trở thành bottleneck trước cả network. Hai cách giảm nhẹ phổ biến: cache byte đã nén cho các response cacheable (nén một lần, serve nhiều lần) và bỏ qua compression dưới một kích thước body tối thiểu (nén một response JSON 50 byte thường không đáng CPU).

Gotcha: compression cũng là cách kinh điển vô tình chặn một async
runtime. Nén một buffer lớn đồng bộ bên trong một task giữ worker thread
cho toàn bộ phép toán (cooperative scheduling của
[`03-rust/05-async.md`](../03-rust/05-async.md)), làm đứng mọi connection khác trên nó. Hoặc dùng
một encoder streaming yield giữa các chunk, hoặc đẩy các lần nén lớn vào
`spawn_blocking`.

### BREACH: compression cộng bí mật là một kênh phụ
Nếu một response body chứa cả một **bí mật** (một CSRF token, một API key) và **input phản chiếu do attacker kiểm soát**, nén nó làm rò rỉ bí mật. Attacker gửi các phỏng đoán làm input; khi một phỏng đoán khớp một phần bí mật, hai chuỗi nén cùng nhau và response trở nên *nhỏ hơn* một cách đo được. Lặp lại từng byte và bí mật lộ ra — qua cả HTTPS, vì độ dài response vẫn thấy được bất kể mã hóa. Đây là tấn công BREACH, và nó là lý do compression response không phải một chiến thắng vô điều kiện.

Các biện pháp giảm nhẹ, theo thứ tự thực tế: đừng phản chiếu input người
dùng vào response chứa bí mật; tách bí mật khỏi nội dung có thể nén; tắt
compression cho response đã authenticate mà phản chiếu input; hoặc thêm
padding độ dài ngẫu nhiên để độ dài không còn là tín hiệu sạch (che giấu
chứ không sửa). Với một proxy, phiên bản khả thi là làm compression cấu
hình được theo từng route để một team có thể tắt nó ở các endpoint chỗ
này áp dụng.

Gotcha: đây là một cân nhắc có thật, không phải lý do để tắt compression
toàn cục — đại đa số response không chứa bí mật, và tắt compression ở
mọi nơi là một chi phí lớn, chắc chắn để đổi lấy một rủi ro hẹp, có điều
kiện.

### Đừng nén hai lần
Nếu upstream đã nén body rồi (nó gửi `Content-Encoding: gzip`), proxy không được nén nó lần nữa — hoặc pass-through nguyên vẹn nếu client chấp nhận encoding đó, hoặc giải-nén-rồi-nén-lại chỉ khi client cần một encoding khác với cái upstream cung cấp.

Gotcha: khi bạn *thực sự* giải nén một response upstream, bạn đã nhận lấy
rủi ro decompression-bomb từ [`07-security/09-ddos.md`](../07-security/09-ddos.md) — giới hạn kích
thước đã giải nén và tỉ lệ giãn nở, và streaming thay vì vật chất hóa toàn
bộ. Điều tương tự áp dụng cho body *request* đã nén mà bạn giải nén để
kiểm tra WAF ([`07-security/06-waf.md`](../07-security/06-waf.md)).

## Practice
Làm theo thứ tự này.

1. Trong [`labs/02-http-server`](../../labs/02-http-server), thêm nén response gzip có điều kiện theo
   `Accept-Encoding`, set `Content-Encoding` và `Vary`. **Xong khi** một
   client gửi `Accept-Encoding: gzip` nhận một body đã nén mà
   `curl --compressed` decode đúng, và một client gửi không có gì nhận
   plaintext.
2. Làm nó streaming và bỏ `Content-Length` để chuyển sang chunked. **Xong
   khi** memory phẳng khi serve một body 1 GB, và response không mang
   `Content-Length` đã cũ — kiểm tra raw byte bằng `curl --raw` để xác
   nhận, vì đây là nơi smuggling vô tình bắt nguồn.
3. Thêm gate theo content-type và kích thước tối thiểu. **Xong khi** một
   response JSON 50 byte và một JPEG đều trả về không nén trong khi HTML
   thì có, và bạn đã đo được rằng nén JPEG chỉ lợi dưới 1%.
4. Thêm brotli và zstd với negotiation q-value đầy đủ, bao gồm `q=0` và
   `*`. **Xong khi** `gzip;q=0, br` chọn brotli, `*;q=0` được xử lý có chủ
   đích, và logic chọn của bạn có một test cho mỗi trường hợp biên.
5. Đo chi phí CPU. **Xong khi** bạn có số throughput ở brotli quality 4 so
   với 11 dưới tải ([`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)) và có thể nói rõ tỉ
   lệ đạt được so với CPU bỏ ra.
6. Cache byte đã nén cho response cacheable
   ([`05-http-stack/07-cache.md`](07-cache.md)). **Xong khi** request lặp lại cho cùng
   resource nén zero lần.
7. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), xử lý một response upstream đã nén sẵn.
   **Xong khi** một body upstream gzip được pass qua nguyên vẹn cho một
   client chấp nhận gzip, và được transcode đúng một lần cho một client
   chỉ chấp nhận brotli — với một bước giải nén có giới hạn.
8. (Stretch) Chứng minh BREACH trên một endpoint đồ chơi phản chiếu một
   query parameter cạnh một bí mật, đã nén. **Xong khi** bạn có thể khôi
   phục bí mật chỉ từ độ dài response — rồi thêm việc tắt compression
   theo route và xác nhận tín hiệu biến mất.
