# TLS

Handshake, SNI, ALPN, mTLS, session resumption. Nếu "asymmetric
encryption," "certificate chain," hay "digital signature" chưa phải các
thuật ngữ chính xác với bạn, đọc [`01-network/06-crypto-basics.md`](06-crypto-basics.md) trước —
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
[`labs/02-http-server`](../../labs/02-http-server) chọn đúng protocol mà không cần một port riêng cho
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
mạng riêng — gắn với [`07-security/01-auth.md`](../07-security/01-auth.md) để biết cái này kết hợp thế
nào với auth dựa trên JWT cho request của end-user.

### Session resumption
Session ticket (hoặc session ID) cho phép một client quay lại bỏ qua
handshake đầy đủ trên một kết nối mới, cắt bớt một round trip. Với một
proxy, điều này quan trọng nhất dưới connection churn cao — hỗ trợ
resumption (và việc xoay key của nó) ảnh hưởng trực tiếp tới tail latency
cho các client kết nối lại thường xuyên. Gotcha: resumption kiểu 0-RTT
tái tạo lại rủi ro replay tương tự 0-RTT của QUIC ([`13-http3.md`](13-http3.md)) — áp
dụng cùng sự thận trọng "chỉ cho request idempotent".

### Quản lý certificate
Một proxy production cần certificate được phát hành, gia hạn (thường qua
ACME/Let's Encrypt), và reload *mà không* làm rớt các kết nối hiện có
hoặc yêu cầu restart — đây là lý do [`proxy`](../../proxy) coi việc reload cert là một
mối quan tâm của config-reload, xem [`09-architecture/03-config.md`](../09-architecture/03-config.md).

### Một CA local để test
`curl --cacert` cần một certificate nối chuỗi tới một CA mà bạn tin, nên
hãy tạo hai thứ: một CA, và một certificate server ("leaf") do nó ký.
Phục vụ trực tiếp certificate của CA thì mong manh: curl có thể chấp nhận,
nhưng một client dựa trên rustls (như client `quinn` trong [`labs/09-http3`](../../labs/09-http3))
từ chối một CA certificate được đưa ra làm server certificate. Client hiện đại cũng bỏ qua
`CN` của subject và chỉ so hostname với danh sách **Subject Alternative
Name** (SAN), nên một leaf không có SAN sẽ verify thất bại dù nó "đúng tên".

```sh
# 1. CA: một certificate tự ký cùng key của nó (CA:TRUE được đặt mặc định)
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca.key -out ca.pem \
  -days 365 -subj "/CN=Lab CA"
# 2. key và signing request cho server
openssl req -newkey rsa:2048 -nodes -keyout a.key -out a.csr -subj "/CN=a.test"
# 3. CA ký nó, thêm danh sách SAN mà client sẽ so khớp
printf "subjectAltName=DNS:a.test,DNS:localhost,IP:127.0.0.1\n" > a.ext
openssl x509 -req -in a.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
  -out a.pem -days 30 -extfile a.ext
openssl verify -CAfile ca.pem a.pem        # phải in ra "a.pem: OK"
```

Lặp lại bước 2-3 với `b.test` cho hostname thứ hai. Server load `a.pem` +
`a.key`. Client nhận `ca.pem`: `curl --cacert ca.pem https://localhost:8443/`.
Để test một tên không resolve được, ghim nó lại:
`curl --cacert ca.pem --resolve a.test:8443:127.0.0.1 https://a.test:8443/`.
`openssl s_client -connect 127.0.0.1:8443 -servername a.test -CAfile ca.pem`
cho thấy certificate nào được trả về (`subject=`) và ALPN đã thương lượng
(`ALPN protocol:`).

### rustls và tokio-rustls trong thực tế
Các mảnh ghép, theo thứ tự một connection dùng tới chúng:

- **Một crypto provider.** rustls 0.23 không tự làm mật mã. Nó dùng một
  provider: `aws-lc-rs` (feature mặc định) hoặc `ring`. Nếu đúng một trong
  hai được bật trong toàn bộ dependency graph, `ServerConfig::builder()` tự
  chọn nó. Nếu cả hai cùng bị bật (một crate khác kéo cái còn lại vào),
  builder sẽ panic lúc chạy và đòi một `CryptoProvider` cấp process. Cách
  sửa là một lời gọi lúc khởi động, ví dụ
  `rustls::crypto::aws_lc_rs::default_provider().install_default()`.
  Repo này gặp đúng chuyện đó: `quinn` ([`labs/09-http3`](../../labs/09-http3)) bật `ring` còn
  `tokio-rustls` bật `aws-lc-rs`. `cargo run -p tls` chỉ build feature của
  lab 07 nên chạy được, nhưng `cargo build --workspace` gộp feature của mọi
  member, và binary `tls` sinh ra từ đó có cả hai.
- **Load file PEM.** Khi trait `rustls::pki_types::pem::PemObject` nằm trong
  scope, `CertificateDer::pem_file_iter(path)` trả ra từng certificate trong
  file và `PrivateKeyDer::from_pem_file(path)` load key (PKCS#8, PKCS#1 hoặc
  SEC1). File certificate bạn phục vụ chứa leaf trước, rồi các
  intermediate nếu có, không bao giờ chứa root.
- **Config.** `ServerConfig::builder().with_no_client_auth()` rồi hoặc
  `.with_single_cert(chain, key)` cho một certificate, hoặc
  `.with_cert_resolver(resolver)` để chọn theo tên SNI (xem dưới). Đặt
  `alpn_protocols` trên config nhận được, như ở phần ALPN phía trên.
- **Handshake.** `tokio_rustls::TlsAcceptor::from(Arc::new(config))`, rồi
  `acceptor.accept(tcp_stream)` là một future thực hiện toàn bộ handshake và
  trả ra một TLS stream. Future đó không có deadline riêng: một client kết
  nối rồi không gửi gì sẽ giữ nó mãi mãi. Bọc nó trong
  `tokio::time::timeout` ([`07-security/10-slowloris.md`](../07-security/10-slowloris.md)).
- **Sau handshake.** TLS stream đi vào `TokioIo::new(...)` rồi vào hyper y
  như một TCP stream ([`05-http-stack/02-hyper.md`](../05-http-stack/02-hyper.md)). `tls_stream.get_ref().1`
  là rustls connection. `alpn_protocol()` của nó trả về `Some(b"h2")` hoặc
  `Some(b"http/1.1")`, nên bạn có thể chọn builder chỉ-HTTP/2 hoặc
  chỉ-HTTP/1.1 thay vì để auto builder tự dò.

**Chọn certificate theo SNI.** `rustls::server::ResolvesServerCertUsingSni`
ánh xạ tên tới certificate: `.add("a.test", certified_key)`, trong đó một
`CertifiedKey` là chain cộng một signing key dựng từ private key
(`CertifiedKey::from_der` nhận chain, key và provider). Nó không có
fallback: client không gửi SNI, hoặc gửi một tên lạ, sẽ handshake thất bại.
Nếu bạn muốn một certificate mặc định, hãy tự implement trait
`ResolvesServerCert`. Method duy nhất của nó nhận `ClientHello`, và
`server_name()` của nó chính là giá trị SNI. Cùng cơ chế này ở quy mô lớn,
với nhiều tenant, nằm ở [`05-http-stack/12-vhost-routing.md`](../05-http-stack/12-vhost-routing.md).

**Phiên bản protocol.** rustls chỉ implement TLS 1.2 và TLS 1.3. TLS 1.0 và
1.1 không tồn tại trong nó, nên "từ chối TLS 1.1" đúng ngay từ cấu trúc.
Muốn đi xa hơn và chỉ cho phép 1.3, hãy dựng bằng
`ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])`.

Gotcha: khi test rằng một version cũ bị từ chối, hãy chắc chắn là *server*
của bạn từ chối. Chính client của OpenSSL 3 cũng từ chối TLS 1.0/1.1 ở mức
security level mặc định, nên `openssl s_client -tls1_1` thất bại với bất kỳ
server nào. Thêm `-cipher 'DEFAULT@SECLEVEL=0'` để client thực sự đề nghị
TLS 1.1. Khi đó một lần thất bại mới chứng minh server đã nói không.

## Practice

1. Dùng `openssl s_client -connect host:443 -servername example.com` và
   đọc output handshake để xác định cipher suite và TLS version đã được
   negotiate.
2. Làm [`labs/07-tls`](../../labs/07-tls) trước: chấm dứt TLS bằng `tokio-rustls` trên một
   listener trần, phục vụ một self-signed cert để test cục bộ. Một khi nó
   chạy được ở đó, port cùng setup đó sang [`proxy`](../../proxy).
3. Cấu hình ALPN sao cho cả `curl --http2` lẫn `curl --http1.1` đều hoạt
   động trên cùng một port, và xác nhận qua `curl -v` protocol nào đã
   được negotiate.
4. Thêm mTLS: yêu cầu và xác minh một client certificate, và từ chối các
   kết nối không trình ra cái nào hoặc trình ra một cái không đáng tin.
5. Mô phỏng một lần xoay cert (đổi file cert, kích hoạt reload theo
   [`09-architecture/03-config.md`](../09-architecture/03-config.md)) và xác nhận các kết nối hiện có không
   bị rớt giữa chừng request.
