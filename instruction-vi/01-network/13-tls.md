# TLS

Handshake, SNI, ALPN, mTLS, session resumption. Nếu "asymmetric
encryption," "certificate chain," hay "digital signature" chưa phải các
thuật ngữ chính xác với bạn, đọc `01-network/06-crypto-basics.md` trước —
file này giả định bạn đã biết chúng.

## What to learn

### Handshake TLS 1.3
Client và server negotiate một cipher suite, trao đổi key share
(EC)DHE, và derive session key — TLS 1.3 làm việc này trong một round trip
(so với hai của TLS 1.2), với chính certificate của server được mã hóa
một khi key đã được derive. Một proxy chấm dứt TLS đứng chính xác ở đây:
nó giữ private key, hoàn tất handshake với client, và (thường) nói
plaintext hoặc một phiên TLS *riêng* tới upstream.

### SNI (Server Name Indication)
Client gửi hostname đích dưới dạng cleartext trong lúc handshake (trước
khi server chọn một certificate), đây là cách một proxy trên một IP có
thể chấm dứt TLS cho nhiều domain/cert khác nhau — chọn cert dựa trên
SNI, không dựa trên IP mà request đã tới. Gotcha: SNI được gửi dưới dạng
cleartext theo mặc định (ECH/Encrypted Client Hello là cách sửa, chưa phổ
biến toàn diện), nên một proxy có thể route dựa trên nó nhưng không nên
giả định nó là riêng tư.

### ALPN (Application-Layer Protocol Negotiation)
Được negotiate bên trong cùng handshake đó, ALPN là cách client và server
đồng ý dùng HTTP/1.1 hay HTTP/2 (`h2`) trước khi bất kỳ byte HTTP nào
được trao đổi — đây là thứ cho phép auto server của `hyper-util` trong
`labs/02-http-server` chọn đúng protocol mà không cần một port riêng cho
mỗi version.

```rust
// tokio-rustls: quảng cáo HTTP/2 rồi HTTP/1.1 qua ALPN
let mut config = rustls::ServerConfig::builder()
    .with_no_client_auth()
    .with_single_cert(cert_chain, private_key)?;
config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
```

### mTLS (mutual TLS)
Server cũng yêu cầu và xác minh một certificate của client, xác thực
client ở tầng transport thay vì (hoặc thêm vào) một token ở tầng ứng
dụng. Phổ biến ở tầng proxy cho tin cậy service-to-service bên trong một
mạng riêng — gắn với `07-security/01-auth.md` để biết cái này kết hợp thế
nào với auth dựa trên JWT cho request của end-user.

### Session resumption
Session ticket (hoặc session ID) cho phép một client quay lại bỏ qua
handshake đầy đủ trên một kết nối mới, cắt bớt một round trip. Với một
proxy, điều này quan trọng nhất dưới connection churn cao — hỗ trợ
resumption (và việc xoay key của nó) ảnh hưởng trực tiếp tới tail latency
cho các client kết nối lại thường xuyên. Gotcha: resumption kiểu 0-RTT
tái tạo lại rủi ro replay tương tự 0-RTT của QUIC (`12-http3.md`) — áp
dụng cùng sự thận trọng "chỉ cho request idempotent".

### Quản lý certificate
Một proxy production cần certificate được phát hành, gia hạn (thường qua
ACME/Let's Encrypt), và reload *mà không* làm rớt các kết nối hiện có
hoặc yêu cầu restart — đây là lý do `proxy` coi việc reload cert là một
mối quan tâm của config-reload, xem `09-architecture/03-config.md`.

## Practice

1. Dùng `openssl s_client -connect host:443 -servername example.com` và
   đọc output handshake để xác định cipher suite và TLS version đã được
   negotiate.
2. Làm `labs/07-tls` trước: chấm dứt TLS bằng `tokio-rustls` trên một
   listener trần, phục vụ một self-signed cert để test cục bộ. Một khi nó
   chạy được ở đó, port cùng setup đó sang `proxy`.
3. Cấu hình ALPN sao cho cả `curl --http2` lẫn `curl --http1.1` đều hoạt
   động trên cùng một port, và xác nhận qua `curl -v` protocol nào đã
   được negotiate.
4. Thêm mTLS: yêu cầu và xác minh một client certificate, và từ chối các
   kết nối không trình ra cái nào hoặc trình ra một cái không đáng tin.
5. Mô phỏng một lần xoay cert (đổi file cert, kích hoạt reload theo
   `09-architecture/03-config.md`) và xác nhận các kết nối hiện có không
   bị rớt giữa chừng request.
