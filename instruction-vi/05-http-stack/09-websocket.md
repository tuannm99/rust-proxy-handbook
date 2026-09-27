# WebSocket Proxying

## What to learn

### Upgrade handshake
Một connection WebSocket bắt đầu như một request GET HTTP/1.1 bình thường với `Upgrade: websocket`, `Connection: Upgrade`, và một `Sec-WebSocket-Key`. Server trả về `101 Switching Protocols` với `Sec-WebSocket-Accept` tính từ key đó (SHA-1 + một GUID cố định, theo RFC 6455 §1.3). Sau `101`, connection ngừng hoàn toàn là HTTP — giờ nó là một luồng byte đã framing thô trên cùng connection TCP (hoặc TLS) đó.

```rust
// sketch: computing Sec-WebSocket-Accept
use sha1::{Sha1, Digest};
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
fn accept_key(client_key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(client_key.as_bytes());
    hasher.update(WS_GUID.as_bytes());
    base64::encode(hasher.finalize())
}
```

Lưu ý handshake này *không phải* là gì: `Sec-WebSocket-Key` không phải một
bí mật và phép tính accept không phải authentication. Nó tồn tại để một
cache hay một intermediary không biết về WebSocket không bị lừa coi
response này như một HTTP response bình thường — chỉ vậy thôi. Đừng bao
giờ đọc yếu tố bảo mật vào nó.

Gotcha: `Connection` và `Upgrade` là header hop-by-hop
(`05-http-stack/04-keepalive.md`). Một proxy không được forward chúng mù
quáng — nó kết thúc một lần upgrade và khởi tạo một lần khác, sinh lại cả
hai header cho leg upstream. Một proxy strip header hop-by-hop đúng đắn
rồi *sau đó* quên thêm lại chúng cho request upgrade làm hỏng WebSocket
hoàn toàn; đây là cách phổ biến nhất một proxy đang chạy tốt mất hỗ trợ
WebSocket trong một lần refactor.

Gotcha: validate `Sec-WebSocket-Version: 13` và negotiate
`Sec-WebSocket-Protocol` trung thực — nếu client đề nghị các subprotocol,
response phải nêu tên đúng một trong số đó hoặc không cái nào cả. Echo
lại toàn bộ danh sách là một vi phạm spec mà một số client chấp nhận còn
một số khác bỏ connection.

### Kiểm tra Origin: lỗ hổng mà WebSocket trao cho bạn
Browser gửi cookie kèm handshake WebSocket, và **same-origin policy không
áp dụng** — không có CORS preflight cho một lần upgrade WebSocket. Nên
bất kỳ website nào nạn nhân ghé thăm đều có thể mở một WebSocket tới
service của bạn, đã authenticate như nạn nhân, và đọc mọi thứ trên đó.
Đây là Cross-Site WebSocket Hijacking, và nó là hành vi mặc định trừ khi
bạn ngăn nó.

Handshake mang một header `Origin` (browser tự set và không cho script
giả mạo). Validate nó với một allowlist và reject các trường hợp không
khớp ngay tại lúc upgrade. Client không phải browser không gửi `Origin`
chút nào, nên quyết định tường minh việc thiếu `Origin` nghĩa là gì cho
service của bạn thay vì để nó mặc định là được phép.

Gotcha: auth dựa trên token ngay trên chính WebSocket (một token trong
URL hoặc trong message đầu tiên) né hoàn toàn vấn đề cookie, và là thiết
kế vững chắc hơn — nhưng một token trong query string sẽ lọt vào access
log (`08-observability/01-logging.md`), nên hãy xóa nó ở đó.

### Vì sao một proxy không thể coi đây là request/response sau khi upgrade
Một khi `101` được gửi, cả router lẫn bất kỳ middleware theo-từng-request nào (auth, compression, caching) được xây quanh mô hình "một request vào, một response ra" không còn áp dụng — không có request tiếp theo trên connection này, chỉ có một ống byte hai chiều. Một reverse proxy phải xử lý Upgrade như một trường hợp đặc biệt: sau khi forward handshake, nó chuyển sang relay byte thô cả hai chiều cho tới khi một trong hai bên đóng.

Điều này có hệ quả cho mọi timeout và giới hạn bạn cấu hình dựa trên mô
hình tinh thần request/response. Read timeout dùng để bắt một request bị
treo (`06-proxy/01-upstream.md`) giờ nổ trên một WebSocket rảnh hoàn toàn
khỏe mạnh. Giới hạn "tổng thời gian request" giết một connection đáng lẽ
sống nhiều giờ. **Áp request timeout lên connection đã upgrade là bug
WebSocket-qua-proxy phổ biến nhất**, và nó biểu hiện như "app của chúng
tôi ngắt kết nối mỗi 60 giây" — điều người dùng nhận ra và log hiếm khi
giải thích được.

Một khi đã upgrade, các giới hạn áp dụng là loại khác: một idle timeout đo
theo *độ sống của ping/pong* thay vì request, một giới hạn tuổi thọ
connection, và giới hạn memory theo từng connection.

### Cơ bản về framing
Frame WebSocket có một opcode (text, binary, close, ping, pong, continuation), một độ dài payload (với một encoding độ dài mở rộng cho payload lớn), và một masking key cho frame client→server (masking bắt buộc phía client, cấm phía server — một biện pháp chống cache-poisoning từ RFC). Một proxy chỉ forward byte không cần parse frame đầy đủ trừ khi nó cần kiểm tra/lọc message.

Gotcha: nếu bạn *thực sự* parse frame (để kiểm tra, lọc, hoặc áp giới hạn
kích thước message), trường độ dài mở rộng 64-bit nằm dưới quyền kiểm
soát của attacker. Một header frame khai một payload 2^63 byte phải bị
reject theo một giới hạn cấu hình *trước* bất kỳ lần cấp phát nào — không
bao giờ `Vec::with_capacity(declared_len)`. Đây là cùng loại bug với một
decompression bomb (`07-security/09-ddos.md`): tin vào một trường độ dài
do peer chọn.

### Keepalive qua ping/pong
Các connection WebSocket rảnh sống lâu trông giống hệt một connection chết dưới góc nhìn của network (một NAT box hay LB có thể âm thầm drop chúng). Bên nào cũng có thể gửi một frame `ping`; bên kia phải trả lời `pong` với cùng payload. Một proxy relay traffic WebSocket nên hoặc pass các cái này qua trong suốt, hoặc, nếu terminate và thiết lập lại hai leg WebSocket riêng, tự sinh ping keepalive của riêng nó trên mỗi leg độc lập.

Gotcha: ping phải chạy theo một đồng hồ *và* có một deadline cho pong.
Gửi ping mà không theo dõi pong có quay lại không thì không phát hiện
được gì — connection chết dù sao đi nữa, bạn chỉ cảm thấy yên tâm hơn một
chút. Đóng connection sau một pong bị bỏ lỡ, và đếm các lần đóng đó như
một metric (`08-observability/02-metrics.md`): tỉ lệ tăng thường nghĩa là
một intermediary đang drop connection rảnh, điều có thể hành động được.

### Backpressure
Không như request/response bình thường nơi `Content-Length` giới hạn công việc, một connection WebSocket có thể có một bên sinh message nhanh hơn bên kia tiêu thụ — proxy đứng giữa hai luồng có nhịp độ độc lập và cần buffer có giới hạn (không phải queue vô hạn) để một client chậm không thể gây tăng trưởng memory vô hạn trên proxy.

`tokio::io::copy_bidirectional` cho bạn điều này miễn phí ở tầng byte: nó
đọc vào một buffer cố định và không đọc thêm cho tới khi bên viết đã
drain, nên một reader chậm tự nhiên chặn một writer nhanh. Ngay khi bạn
đưa vào một channel riêng giữa hai leg (để kiểm tra hay biến đổi message),
bạn sở hữu bài toán backpressure — dùng một channel có giới hạn, và hiểu
rằng "có giới hạn" nghĩa là một client chậm cuối cùng sẽ chặn lần đọc
upstream, điều đó là đúng đắn.

Gotcha: một connection WebSocket là một tài nguyên *sống lâu*, nên cách
tính toán từ `07-security/09-ddos.md` thay đổi hình dạng. Mười nghìn
WebSocket rảnh tốn mười nghìn fd, socket, và cặp buffer, vô thời hạn,
trong khi không sinh request nào cả — nên giới hạn tốc độ request không
ràng buộc được chúng. Giới hạn số connection đã upgrade đồng thời tường
minh, cả theo từng client lẫn toàn cục.

### WebSocket qua HTTP/2 và HTTP/3
RFC 8441 mang WebSocket qua HTTP/2 dùng một `CONNECT` mở rộng với một pseudo-header `:protocol`, thay vì cơ chế `Upgrade` của HTTP/1.1 (HTTP/3 làm tương đương). Một proxy terminate HTTP/2 từ client và nói HTTP/1.1 với upstream — hay ngược lại — phải dịch giữa hai dạng, và một proxy chỉ biết đường `Upgrade` sẽ reject thẳng client WebSocket HTTP/2. Biết stack của bạn hỗ trợ dạng nào trước khi hứa hẹn hỗ trợ WebSocket qua HTTP/2.

## Practice
Làm theo thứ tự này.

1. Trong `labs/02-http-server`, implement upgrade handshake bằng tay —
   validate `Sec-WebSocket-Version`, tính `Sec-WebSocket-Accept`, không
   dùng crate WebSocket nào. **Xong khi** một browser thật hoặc client
   `websocat` hoàn thành handshake với nó.
2. Thêm validate `Origin` với một allowlist, cùng một chính sách tường
   minh cho `Origin` bị thiếu. **Xong khi** một handshake từ một origin
   không được phép bị reject ngay lúc `101` — viết một trang HTML ở một
   origin khác mở một socket tới server của bạn và xác nhận nó fail.
3. Chiếm lấy luồng đã upgrade thô và echo lại frame text, parse vừa đủ
   để unmask và re-frame. **Xong khi** một client round-trip được cả
   message text lẫn binary, và một frame khai một payload length phi lý
   bị reject mà không cấp phát.
4. Trong `labs/05-reverse-proxy`, thêm pass-through proxying: forward
   handshake (sinh lại header hop-by-hop), rồi relay bằng
   `tokio::io::copy_bidirectional`. **Xong khi** một WebSocket end-to-end
   hoạt động qua proxy.
5. Audit timeout của bạn. **Xong khi** một WebSocket rảnh sống lâu hơn
   hẳn read timeout của request và tổng request timeout — set cả hai
   thành 10 giây có chủ đích và xác nhận một socket rảnh 5 phút vẫn sống,
   sau khi trước đó đã xác nhận nó *chết* khi chưa sửa.
6. Thêm ping/pong với một deadline cho pong và một metric pong-bị-lỡ.
   **Xong khi** một peer ngừng trả lời ping bị đóng trong deadline và lần
   đóng đó được đếm.
7. Thêm một test backpressure: writer upstream nhanh, reader client cố
   tình chậm. **Xong khi** memory của proxy phẳng suốt thời gian đó thay
   vì tăng theo backlog.
8. Giới hạn số connection đã upgrade đồng thời, cả toàn cục lẫn theo từng
   client. **Xong khi** một client mở connection trong vòng lặp bị từ
   chối khi vượt giới hạn và traffic HTTP bình thường không bị ảnh hưởng.
