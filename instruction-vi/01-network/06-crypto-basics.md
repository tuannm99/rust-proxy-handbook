# Crypto Basics: Mã hóa, Hashing, Chữ ký, PKI

File này thuộc nhóm fundamentals. Xem [`01-network/01-fundamentals.md`](01-fundamentals.md) để có mục lục đầy đủ.
[`01-network/19-tls.md`](19-tls.md) mở đầu bằng "thương lượng một cipher suite, trao đổi (EC)DHE key share, derive
session key". [`07-security/02-jwt.md`](../07-security/02-jwt.md), [`07-security/03-mtls.md`](../07-security/03-mtls.md)
và [`07-security/01-auth.md`](../07-security/01-auth.md) đều dựa vào "signature", "public key" và "certificate chain".
Không đọc được các file đó nếu thiếu file này.

File này không dạy cryptography như một môn học. Nó dạy năm khối nền, đủ rõ để handshake của TLS và cơ chế chữ ký của
JWT không còn là chuyện thần bí.
Về phần toán thực sự bên dưới các khối nền này (cấu trúc round của AES, Diffie-Hellman, RSA), xem
[`22-theory/07-crypto-math.md`](../22-theory/07-crypto-math.md).

## What to learn

### Mã hóa symmetric: một bí mật dùng chung, nhanh
Mã hóa **symmetric** dùng *cùng một* key để encrypt và decrypt. Nó nhanh, đủ rẻ để chạy trên mọi byte của mọi
connection. Chính tốc độ đó là lý do nó bảo vệ phần lớn dữ liệu trên một TLS connection (AES và ChaCha20 là hai cái tên
bạn sẽ thấy trong một TLS cipher suite).

Vấn đề là phân phối key. Cả hai bên cần *cùng* một secret key *trước khi* có thể nói chuyện an toàn. Đây là bài toán
con gà và quả trứng trên một mạng mà hai người lạ vừa mới kết nối: làm sao thống nhất một bí mật mà kẻ nghe lén trên
đường truyền không thấy được? Mục tiếp theo giải quyết đúng vấn đề này.

### Mã hóa asymmetric (public-key): hai key, chậm, không cần secret dùng chung
Mã hóa **asymmetric** dùng một *cặp* key có quan hệ toán học với nhau:
- **public key**: đưa cho ai cũng an toàn, kể cả kẻ tấn công.
- **private key**: không bao giờ chia sẻ, chủ sở hữu giữ bí mật.

Dữ liệu được mã hóa bằng public key chỉ giải mã được bằng private key tương ứng. Cách này giải quyết vấn đề phân phối ở
trên: không có bí mật nào phải đi qua đường truyền. Cái giá là tốn tính toán hơn symmetric rất nhiều, quá chậm cho
traffic khối lượng lớn.

Các thuật toán **key exchange** (Diffie-Hellman và biến thể đường cong elliptic của nó, ECDHE, chính là "(EC)DHE" trong
dòng mở đầu của [`19-tls.md`](19-tls.md)) là một mẹo khéo dùng asymmetric. Hai bên trao đổi các giá trị công khai qua
đường truyền mở. Mỗi bên độc lập *tính ra* cùng một secret dùng chung, từ giá trị private của mình và giá trị public
của bên kia. Kẻ nghe lén thấy cả hai giá trị public nhưng không thể tính ra secret chung, nếu không có giá trị private
của một trong hai bên. Nhờ vậy TLS có một symmetric key mới cho mỗi connection mà không bao giờ truyền chính key đó.

### Cách tiếp cận hybrid mà mọi TLS connection thực sự dùng
Không khối nền nào vừa nhanh vừa dễ phân phối, nên TLS dùng cả hai, theo thứ tự:
1. Key exchange **asymmetric** (ECDHE) trong handshake, để thống nhất một secret chung mà không truyền nó.
2. Mã hóa **symmetric** (AES/ChaCha20) dùng secret đã derive đó, cho toàn bộ traffic request/response thật.

Vì vậy tên cipher suite trong [`19-tls.md`](19-tls.md) gồm nhiều phần (ví dụ `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256`).
Nó gọi tên thuật toán key exchange, thuật toán signature (mục sau), và cipher symmetric. Cả ba được thương lượng cùng
lúc.

### Hashing: dấu vân tay một chiều
Một **hash function** nhận input có kích thước bất kỳ và tạo ra output kích thước cố định (một "digest"). Nó có ba tính
chất quan trọng:
- Cùng input luôn cho cùng output.
- Input thay đổi chút ít thì output thay đổi hoàn toàn.
- Không thể tính ngược từ digest về input (một chiều).

SHA-256 (output 256 bit, 32 byte) là cái được nhắc đến nhiều nhất trong handbook này.

Hashing đơn thuần cho bạn *kiểm tra tính toàn vẹn* (dữ liệu này có bị thay đổi không?). Nhưng nó không cho *tính xác
thực* (dữ liệu này có đến từ đúng người tôi nghĩ không?), vì ai cũng hash được bất cứ thứ gì.

Các hash function trong [`13-algorithms/count-min-sketch.md`](../13-algorithms/count-min-sketch.md) và
[`13-algorithms/hashmap.md`](../13-algorithms/hashmap.md) giải quyết vấn đề hoàn toàn khác (phân phối key vào các
bucket), và không cần tính một chiều kiểu cryptographic. Đừng nhầm hash function dùng cho hash table với hash function
*cryptographic* dùng cho bảo mật. Loại bảo mật chậm hơn nhiều, và được chọn để chống các cuộc tấn công mà hash nhanh
của hash table không cần chống.

### HMAC: hash cộng một secret, để có tính xác thực
Một **HMAC** (hash-based message authentication code) kết hợp một cryptographic hash function với một secret key. Digest
tạo ra chỉ người giữ cùng secret đó mới tính được. Nhờ vậy bạn có *tính xác thực* bên cạnh tính toàn vẹn của hashing
đơn thuần.

HMAC là ý nghĩa của "HS256" trong tên thuật toán của một JWT ([`07-security/02-jwt.md`](../07-security/02-jwt.md)): HMAC
dùng SHA-256, với secret dùng chung là key mà service của bạn và bên phát hành token đã thống nhất với nhau từ trước
(out of band).

### Digital signature: tính xác thực mà không cần secret dùng chung
**Digital signature** làm với cặp asymmetric key điều mà HMAC làm với secret dùng chung. Chủ sở hữu *private* key ký dữ
liệu (một phép tính dùng dữ liệu và private key). *Bất kỳ ai* có public key tương ứng đều xác minh được chữ ký hợp lệ,
mà không cần chạm vào private key.

Đó là ý nghĩa của "RS256" và "ES256" trong tên thuật toán của một JWT: chữ ký RSA hoặc ECDSA, được xác minh bằng
public key thay vì secret dùng chung.

Khác biệt thực tế so với HMAC rất quan trọng với cuộc tấn công algorithm-confusion trong
[`07-security/02-jwt.md`](../07-security/02-jwt.md):
- HMAC secret phải được giữ bí mật như nhau ở cả phía ký lẫn phía xác minh (ai xác minh được thì cũng giả mạo được).
- Public key của một signature thì *cố ý* công khai: ai cũng xác minh được, chỉ chủ private key mới ký được.

Nếu nhầm hai thứ này (coi public key như HMAC secret), bạn tạo ra chính lỗ hổng mà cuộc tấn công đó khai thác.

### Certificate và PKI: gắn một public key với một identity
Asymmetric crypto giải quyết bài toán "có secret chung mà không truyền nó". Nhưng nó để lại một khoảng trống: khi trình
duyệt nhận một public key từ server, làm sao biết key đó thật sự thuộc về `example.com` chứ không phải một kẻ tấn công
đứng giữa?

**Certificate** trả lời câu hỏi này. Nó gồm: một public key, một identity (một hostname, trong trường hợp khớp SNI ở
[`19-tls.md`](19-tls.md)), và một **digital signature**. Chữ ký này không phải của chính server, mà của một
**Certificate Authority (CA)**, một bên thứ ba mà trình duyệt/OS của bạn đã tin sẵn.

Từ đó có một **chain of trust**:
1. OS của bạn đi kèm danh sách public key của các **root CA** đáng tin.
2. Root CA ký các certificate của **intermediate CA**.
3. Intermediate CA ký certificate **leaf** (của server), thứ mà proxy của bạn trình ra.

Xác minh một certificate nghĩa là đi ngược chuỗi này: leaf được ký bởi intermediate, intermediate được ký bởi một root
mà bạn đã tin sẵn. Ở mỗi bước, bạn dùng cơ chế xác minh signature đã nói ở mục trước.

Gotcha "chain tới một CA đáng tin yếu hơn nghe có vẻ" trong [`07-security/03-mtls.md`](../07-security/03-mtls.md) nói
đúng về điều này. CA chỉ xác nhận cho *một identity*, không xác nhận *quyền hạn*. Bất kỳ ai mà CA đó chịu ký cho đều có
một chain hợp lệ.

**PKI** (public key infrastructure) là thuật ngữ chung cho cả hệ thống: các CA, các certificate, việc xác minh chain of
trust, và công cụ phát hành, gia hạn chúng (ACME/Let's Encrypt, được nhắc trong phần quản lý certificate của
[`19-tls.md`](19-tls.md)).

## Practice
1. Chạy `openssl s_client -connect example.com:443 -servername example.com` và đọc certificate chain nó in ra. Xác định
   subject của leaf certificate (hostname), rồi lần ngược qua các intermediate tới root. Ghi chú field nào mang identity
   của bên phát hành và field nào mang identity của subject được ký.
2. Chạy `openssl x509 -in <file cert> -noout -text` trên một certificate thật (export một cái từ bước 1 bằng
   `openssl s_client ... | openssl x509`). Xác định public key, signature, và các field issuer/subject đã nói ở trên.
3. Tính `sha256sum` của một file, đổi một byte, rồi tính lại. Xác nhận digest thay đổi hoàn toàn thay vì chỉ một chút
   (tính chất avalanche của một hash function tốt).
4. Đọc mục "Algorithm confusion, concretely" trong [`07-security/02-jwt.md`](../07-security/02-jwt.md) khi sự khác biệt
   giữa HMAC và signature còn mới trong đầu. Giải thích bằng lời của bạn vì sao coi một RSA public key như HMAC secret
   cho phép kẻ tấn công giả mạo token. Gắn nó với ý "ai cũng xác minh được một signature; chỉ chủ sở hữu secret mới tính
   được HMAC" trong file này.
5. Tạo một cặp key RSA hoặc Ed25519 (`openssl genpkey` hoặc `ssh-keygen`), ký một file nhỏ bằng private key, và xác minh
   chữ ký bằng public key. Sau đó thử xác minh bằng public key của một cặp key *khác* và xác nhận nó thất bại.
