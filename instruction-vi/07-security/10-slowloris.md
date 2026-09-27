# Tấn công Slow-Client

Kiểu denial-of-service rẻ nhất: giữ kết nối mở bằng cách chậm, không phải
bằng cách ồn ào. [`07-security/09-ddos.md`](09-ddos.md) bao quát phía volumetric và
connection-flood; file này bao quát nhóm tấn công tốn gần như không băng
thông của attacker.

## What to learn
### Vì sao sự chậm chạp là một cuộc tấn công
Mỗi kết nối mở tốn proxy một file descriptor, kernel socket buffer, và
state riêng của kết nối (phép tính trần tài nguyên của
[`07-security/09-ddos.md`](09-ddos.md)). Một attacker mở kết nối và giữ chúng *về mặt kỹ
thuật vẫn sống* — gửi vừa đủ để tránh mọi timeout — tiêu tốn các tài
nguyên đó với chi phí gần như bằng 0 cho chính họ. Vài ngàn kết nối từ một
host có thể làm cạn một proxy vốn thừa sức chịu cả triệu request mỗi giây.

Sự bất cân xứng chi phí ở đây tệ nhất trong cả phần này: một socket
attacker gửi một byte mỗi phút, đổi lấy một kết nối của bạn, vô hạn.

### Ba biến thể, và vì sao chỉ phòng thủ một cái là không đủ
- **Slow header (Slowloris kinh điển).** Attacker mở một kết nối và nhỏ
  từng byte header một, không bao giờ gửi dòng trắng kết thúc header. Request
  không bao giờ hoàn thành, nên một "request timeout" đo từ lúc request
  *hoàn thành* không bao giờ bắt đầu.
- **Slow body (R-U-Dead-Yet).** Header hoàn thành bình thường và khai báo
  một `Content-Length` lớn; body sau đó đến từng byte một mỗi khoảng thời
  gian. Một deadline ở phase header không còn áp dụng nữa — header đã ổn.
  Cái này còn giữ chặt bất kỳ body buffering nào bạn làm cho WAF inspection
  ([`07-security/06-waf.md`](06-waf.md)) hoặc retry replay ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md))
  trong suốt cả lần upload chậm.
- **Slow read.** Attacker gửi một request hoàn toàn bình thường cho một
  response lớn, rồi đọc response ở tốc độ một byte mỗi khoảng thời gian,
  giữ chặt send buffer của bạn và bất kỳ response bạn đã buffer. Cái này
  vô hình với mọi timeout phía *request* bạn có, vì request hoàn hảo.

Chỉ phòng thủ cái đầu tiên là tình trạng phổ biến, và đó là vì sao những
cái này vẫn còn hiệu quả.

### Một deadline là không đủ; bạn cần một rate floor
Cách sửa hiển nhiên là một timeout tổng cho request. Một attacker tính
toán khéo tránh nó bằng cách hoàn thành ngay dưới mức đó, rồi mở kết nối
kế tiếp. Và tăng timeout tổng lên là đi ngược hoàn toàn: nó làm cuộc tấn
công rẻ hơn.

Điều thực sự đặc trưng cho cuộc tấn công là **throughput**, không phải
duration. Vậy hãy enforce một rate dữ liệu tối thiểu ở mỗi phase — "phải
có ít nhất N byte đến trong T giây, và lại như vậy sau mỗi lần đọc" — cùng
với một deadline theo phase. Một client truyền dữ liệu có ý nghĩa thì tiếp
tục; một client giữ socket mở với một dòng nhỏ giọt bị ngắt kết nối bất kể
nó chờ bao kiên nhẫn.

```rust
// về mặt khái niệm, mỗi phase: byte phải tiếp tục đến đủ nhanh
// để có thể là một client thật, không chỉ đủ nhanh để tránh một deadline
struct RateFloor {
    min_bytes: usize,
    window: std::time::Duration,
}
```

Gotcha: một rate floor calibrate trong datacenter sẽ ngắt kết nối user
thật. Mạng di động, đường truyền tắc nghẽn, và client ở nửa vòng trái đất
thực sự chậm. Đặt floor đủ thấp để rõ ràng là bất thường (vài chục byte
mỗi giây, không phải kilobyte), và dựa vào deadline theo phase để bắt phần
còn lại — hai thứ kết hợp phân biệt tốt hơn nhiều so với dùng riêng lẻ.

Gotcha: áp floor cho cả phía *viết*, hoặc slow read vẫn chưa được xử lý.
Nghĩa là một deadline tiến độ trên response write: nếu socket không nhận
được byte nào trong T giây dù đang có data chờ gửi, client không đọc.

### Các giới hạn này nằm ở đâu
Đây là vấn đề vòng đời kết nối, không phải xử lý request, nên nó thuộc về
connection manager ([`09-architecture/01-components.md`](../09-architecture/01-components.md)) — phải áp dụng
trước và độc lập với bất cứ thứ gì giả định một request đã hoàn chỉnh.
[`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) và [`labs/01-http-parser`](../../labs/01-http-parser) là nơi một request
thực sự được nạp từng byte một, và nơi floor của phase header phải được
enforce.

Gotcha: miễn trừ cho các kết nối upgraded và streaming
([`05-http-stack/09-websocket.md`](../05-http-stack/09-websocket.md), [`05-http-stack/10-grpc.md`](../05-http-stack/10-grpc.md)) khỏi các
deadline theo hình dạng request nhưng *không* khỏi liveness checking — một
WebSocket idle là hợp lệ, một cái không phản hồi thì không, đó là mục đích
của ping/pong deadline.

### HTTP/2 có phiên bản riêng của nó
Multiplexing thay đổi hình dạng nhưng không thay đổi nguyên lý. Một
attacker có thể mở nhiều stream trên một kết nối và bỏ chúng chưa hoàn
thành, hoặc điều khiển flow-control window để buộc server giữ data mà nó
không thể gửi. `SETTINGS_MAX_CONCURRENT_STREAMS` của HTTP/2 chặn cái đầu;
per-connection memory accounting chặn cái sau. Xem [`01-network/11-http2.md`](../01-network/11-http2.md),
cũng bao quát Rapid Reset — cuộc tấn công nghịch, nơi stream được mở và
hủy nhanh nhất có thể.

## Practice
Làm theo thứ tự này.

1. Viết ba attacker dưới dạng test client nhắm vào [`proxy`](../../proxy): slow header,
   slow body, slow read. **Xong khi** cả ba đều giữ được một kết nối mở
   nhiều phút chống lại config hiện tại của bạn — bạn cần cuộc tấn công
   hoạt động trước khi biện pháp phòng thủ có ý nghĩa.
2. Thêm một deadline phase-header cùng một rate floor theo byte. **Xong
   khi** client slow-headers bị ngắt kết nối trong deadline và một client
   bình thường không bị ảnh hưởng.
3. Mở rộng floor sang phase body. **Xong khi** client slow-body bị ngắt
   kết nối, và một upload lớn hợp lệ ở vài trăm KB/s hoàn thành được.
4. Thêm một deadline tiến độ write. **Xong khi** client slow-read bị ngắt
   kết nối và một download streaming lớn tới một client bình thường vẫn
   hoạt động.
5. Xác minh với một client chậm-nhưng-hợp-lệ giả lập (throttle xuống vài
   KB/s bằng `tc` hoặc một proxy rate-limit). **Xong khi** nó *không* bị
   ngắt kết nối — nếu bị, floor của bạn đang đặt theo giả định datacenter.
6. Chạy cả ba cuộc tấn công đồng thời với một load test bình thường
   ([`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)). **Xong khi** p99 latency hợp lệ
   không đổi và số kết nối vẫn bị chặn giới hạn.
7. Xác nhận các miễn trừ. **Xong khi** một WebSocket idle sống sót vô hạn
   trong khi một cái không phản hồi (không pong) bị đóng.
