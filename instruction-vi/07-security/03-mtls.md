# mTLS (Mutual TLS)

Xác thực *kết nối* bằng một client certificate, trước khi bất kỳ HTTP
request nào được parse. `01-network/13-tls.md` nói về cơ chế handshake;
`07-security/01-auth.md` nói về việc này kết hợp thế nào với identity ở
mức request.

## What to learn
### mTLS ở tầng proxy
Với mutual TLS, TLS server của proxy (xem `01-network/13-tls.md`) yêu cầu
và xác minh một client certificate trong lúc handshake, trước khi bất kỳ
HTTP request nào được parse. Proxy kiểm tra cert có chain tới một CA tin
cậy và tùy chọn kiểm tra các trường cụ thể (CN/SAN) so với một allowlist.
Điều này xác thực *kết nối*, không nhất thiết là end user — thường được
kết hợp với JWT (`07-security/02-jwt.md`) cho identity ở mức user nằm trên
mTLS ở mức service.

Gotcha: việc xác minh client cert xảy ra ở tầng TLS (rustls
`WebPkiClientVerifier` hoặc tương tự) — nếu bạn chỉ kiểm tra cert sau khi
đã accept kết nối ở tầng HTTP, bạn đã tiêu tốn tài nguyên cho một peer chưa
xác thực, và bản thân điều đó là một vector DoS.

### "Chain tới một CA tin cậy" yếu hơn nó nghe có vẻ
Nếu CA tin cậy phát hành cert cho bất kỳ ai — một CA công cộng, hoặc một CA
nội bộ dùng chung toàn công ty — thì *bất kỳ* cert hợp lệ nào cũng xác thực
được, và bạn chỉ mới xác minh rằng client tồn tại chứ chưa xác minh rằng nó
được ủy quyền. Ghim vào các giá trị SAN/CN cụ thể, hoặc dùng một CA riêng
chỉ ký cho đúng trust domain này.

Gotcha: match theo CN đã lỗi thời và mơ hồ; hãy match theo SAN. Và match
*toàn bộ* giá trị thay vì một chuỗi con — một kiểm tra "chứa `payments`" sẽ
bị thỏa mãn bởi `payments.evil.example.com`.

### Hết hạn là một outage bạn có thể lên lịch trước
Certificate hết hạn, và một client cert đã hết hạn fail ngay tại thời điểm
handshake với một lỗi không bao giờ tới được HTTP-layer logging của bạn —
vì không có request nào để log. Traffic tổng thể trông vẫn ổn trong khi
một client bị khóa hoàn toàn.

Theo dõi thời gian hết hạn của certificate như một metric kèm cảnh báo từ
sớm trước ngày đó (`08-observability/06-alerting.md`); "client cert hết hạn
qua đêm" là một nguyên nhân hàng đầu của các outage buổi sáng, và tín hiệu
này vô hình trừ khi bạn chủ động đi tìm nó. Điều tương tự áp dụng cho từng
tenant ở phía server (`05-http-stack/11-vhost-routing.md`), nơi một cert
hết hạn chỉ làm fail handshake của đúng tenant đó.

### Revocation gặp vấn đề giống JWT, với ergonomics tệ hơn
CRL lớn và cũ; OCSP thêm một network dependency vào đường handshake, và một
lần OCSP responder sập trở thành một lần handshake sập trừ khi bạn
soft-fail — mà tại điểm đó revocation coi như không được thực thi nữa.

Client cert sống ngắn (vài giờ, tự động renew, như SPIFFE và hầu hết
service mesh làm) né tránh vấn đề này giống hệt cách `exp` ngắn làm với
JWT: cửa sổ mà một credential bị lộ còn hoạt động được giới hạn bởi vòng
đời của nó thay vì bởi một cơ chế revocation mà bạn phải vận hành.

### Proxy làm gì với identity đã xác thực
Subject của certificate là một identity mà upstream thường muốn có. Nó đi
theo cùng cách các claim JWT đi — một header do proxy set, sau khi loại bỏ
bất kỳ phiên bản nào do client tự cung cấp. Xem phần identity-propagation ở
`07-security/01-auth.md`; quy tắc loại bỏ là vô điều kiện và áp dụng y hệt
ở đây.

Gotcha: khi proxy terminate mTLS và mở kết nối *của chính nó* lên upstream,
upstream thấy identity của proxy, không phải của client. Nếu upstream cần
đưa ra quyết định authorization dựa trên client gốc, proxy phải chuyển tiếp
identity đó một cách tường minh — và upstream chỉ nên tin duy nhất proxy để
set nó.

## Practice
Làm lần lượt theo thứ tự sau.

1. Cấu hình TLS listener của `proxy` (`01-network/13-tls.md`) để yêu cầu và
   xác minh client certificate cho một route, dùng `tokio-rustls`. **Xong
   khi** một client không có cert bị từ chối ngay tại handshake, trước khi
   bất kỳ request nào được parse.
2. Ghim vào một SAN cụ thể. **Xong khi** một certificate do đúng CA phát
   hành nhưng mang sai SAN bị từ chối — bài test chứng minh rằng chỉ tin
   CA thôi là chưa bao giờ đủ.
3. Export thời gian còn lại tới khi hết hạn của từng certificate như một
   metric. **Xong khi** một cert còn 7 ngày là hết hạn hiện rõ trên
   dashboard mà không cần ai kiểm tra thủ công.
4. Set identity của client cert thành một header sau khi loại bỏ bất kỳ
   bản sao inbound nào. **Xong khi** một client gửi một identity header giả
   mạo kèm theo một cert hợp lệ chỉ khiến upstream nhận được giá trị lấy từ
   cert.
5. (Mở rộng) Phát hành cert sống ngắn (1 giờ) với renew tự động và xác nhận
   việc xoay vòng diễn ra mà không có handshake nào thất bại.
