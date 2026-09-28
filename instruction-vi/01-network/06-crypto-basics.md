# Crypto Basics: Encryption, Hashing, Signatures, PKI

Một phần của chuỗi fundamentals từ-con-số-0 — xem [`01-network/01-fundamentals.md`](01-fundamentals.md)
để có index đầy đủ. [`01-network/14-tls.md`](14-tls.md) mở đầu bằng "negotiate một
cipher suite, trao đổi (EC)DHE key share, derive session key" và
[`07-security/02-jwt.md`](../07-security/02-jwt.md)/[`07-security/03-mtls.md`](../07-security/03-mtls.md)/[`07-security/01-auth.md`](../07-security/01-auth.md)
đều dựa vào "signature," "public key," và "certificate chain" — không cái
nào đọc hiểu được nếu thiếu file này. File này không dạy cryptography như
một ngành học; nó dạy năm khối xây dựng đủ rõ để handshake của TLS và cơ
chế chữ ký của JWT không còn là phép màu.

## What to learn

### Mã hóa symmetric: một bí mật dùng chung, nhanh
Mã hóa **symmetric** dùng *cùng một* key để encrypt và decrypt. Nó nhanh —
đủ rẻ để chạy trên mọi byte của mọi traffic của một kết nối — và chính tốc
độ đó là lý do nó thực sự bảo vệ phần lớn dữ liệu trên một kết nối TLS
(AES và ChaCha20 là hai cái tên bạn sẽ thấy trong một TLS cipher suite).

Vấn đề là phân phối key: cả hai bên cần *cùng* một secret key *trước khi*
họ có thể nói chuyện an toàn, đây là bài toán con-gà-quả-trứng qua một
mạng mà hai người lạ vừa mới kết nối — làm sao bạn đồng ý về một bí mật mà
không để kẻ nghe lén trên đường truyền cũng thấy nó? Giải quyết chính xác
vấn đề này là việc của mục tiếp theo.

### Mã hóa asymmetric (public-key): hai key, chậm, không cần secret dùng chung
Mã hóa **asymmetric** dùng một *cặp* key liên quan với nhau về mặt toán
học: một **public key** (an toàn để đưa cho bất kỳ ai, kể cả attacker) và
một **private key** (không bao giờ chia sẻ, giữ bí mật bởi chủ sở hữu).
Dữ liệu được mã hóa bằng public key chỉ có thể giải mã bằng private key
tương ứng. Điều này giải quyết vấn đề phân phối ở trên — không cần bí mật
nào phải đi qua đường truyền — với cái giá là tốn kém tính toán hơn nhiều
so với symmetric encryption, quá chậm để dùng cho traffic khối lượng lớn.

Các thuật toán **key exchange** (Diffie-Hellman và biến thể đường cong
elliptic của nó, ECDHE — chính là "(EC)DHE" trong dòng mở đầu của
[`14-tls.md`](14-tls.md)) là một mẹo khéo léo liên quan tới asymmetric: cả hai bên trao
đổi các giá trị công khai qua đường truyền mở, và mỗi bên độc lập *tính
toán* ra cùng một secret dùng chung từ giá trị private của chính mình và
giá trị public của bên kia — một kẻ nghe lén thấy cả hai giá trị public
không thể suy ra được secret dùng chung nếu thiếu giá trị private của một
trong hai bên. Đây là cách TLS có được một symmetric key tươi mới cho mỗi
kết nối, mà không bao giờ truyền chính key đó.

### Cách tiếp cận hybrid mà mọi kết nối TLS thực sự dùng
Không khối xây dựng nào một mình vừa nhanh vừa dễ phân phối, nên TLS dùng
cả hai, theo trình tự: key exchange **asymmetric** (ECDHE) trong lúc
handshake để đồng ý về một secret dùng chung mà không truyền nó, rồi
encryption **symmetric** (AES/ChaCha20) dùng secret đã derive đó cho toàn
bộ traffic request/response thật sự. Đây là lý do tên cipher suite trong
[`14-tls.md`](14-tls.md) có nhiều phần (ví dụ
`TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256`) — nó đang gọi tên thuật toán key
exchange, thuật toán signature (mục tiếp theo), và cipher symmetric, cả ba
được negotiate cùng nhau.

### Hashing: dấu vân tay một chiều
Một **hash function** nhận input với kích thước bất kỳ và tạo ra một
output kích thước cố định (một "digest") với ba tính chất quan trọng ở
đây: cùng một input luôn cho ra cùng một output, một thay đổi nhỏ ở input
tạo ra một output hoàn toàn khác, và không khả thi về mặt tính toán để đi
*ngược* từ digest về thứ tạo ra nó (one-way). SHA-256 (tạo ra 256 bit, 32
byte) là cái bạn sẽ thấy được nhắc đến nhiều nhất trong handbook này.

Hashing đơn thuần cho bạn *kiểm tra tính toàn vẹn* (dữ liệu này có bị thay
đổi không?) nhưng không cho *tính xác thực* (nó có đến từ đúng người tôi
nghĩ không? — ai cũng có thể hash bất cứ thứ gì). Các hash function trong
[`13-algorithms/count-min-sketch.md`](../13-algorithms/count-min-sketch.md) và [`13-algorithms/hashmap.md`](../13-algorithms/hashmap.md) giải
quyết một vấn đề hoàn toàn khác (phân phối key vào các bucket) và rõ ràng
*không* bị ràng buộc bởi tính một-chiều kiểu cryptographic — đừng nhầm lẫn
một hash function dùng cho hash table với một hash function *cryptographic*
dùng cho bảo mật; loại dùng cho bảo mật chậm hơn nhiều và được chọn đặc
biệt để chống lại các tấn công mà một hash function nhanh cho hash table
không cần chống lại.

### HMAC: một hash cộng một secret, để có tính xác thực
Một **HMAC** (hash-based message authentication code) kết hợp một
cryptographic hash function với một secret key, tạo ra một digest mà chỉ
ai đang giữ cùng secret đó mới có thể tạo ra được — đây là thứ cho bạn
tính xác thực chồng lên trên tính toàn vẹn của hashing đơn thuần. HMAC là
thứ "HS256" nghĩa là trong tên thuật toán của một JWT
([`07-security/02-jwt.md`](../07-security/02-jwt.md)): HMAC dùng SHA-256, với secret dùng chung là bất
kỳ key nào mà service của bạn và bên phát hành token đã thống nhất ngoài
băng tần (out of band).

### Digital signature: tính xác thực mà không cần secret dùng chung
Một **digital signature** làm cho asymmetric key điều mà HMAC làm cho một
secret dùng chung: chủ sở hữu *private* key ký dữ liệu (một phép tính liên
quan tới dữ liệu và private key), và *bất kỳ ai* giữ public key tương ứng
đều có thể xác minh chữ ký là hợp lệ — mà không bao giờ cần tiếp cận
private key. Đây là ý nghĩa của "RS256" và "ES256" trong tên thuật toán
của một JWT: chữ ký RSA hoặc ECDSA, được xác minh bằng một public key thay
vì một secret dùng chung.

Khác biệt thực tế so với HMAC quan trọng cho tấn công algorithm-confusion
của [`07-security/02-jwt.md`](../07-security/02-jwt.md): một HMAC secret phải được giữ bí mật ngang
nhau ở cả phía ký lẫn phía xác minh (ai xác minh được thì cũng giả mạo
được), trong khi public key của một signature *có chủ đích* là công khai
— ai cũng xác minh được, chỉ chủ sở hữu private key mới ký được. Nhầm lẫn
hai thứ này (đối xử với một public key như thể nó là một HMAC secret)
chính xác là lỗ hổng mà tấn công đó khai thác.

### Certificate và PKI: gắn một public key với một identity
Asymmetric crypto giải quyết "làm sao có một secret dùng chung mà không
truyền nó", nhưng để lại một khoảng trống: khi trình duyệt của bạn nhận
một public key từ một server, làm sao nó biết key đó thực sự thuộc về
`example.com` chứ không phải một attacker đứng giữa? Một **certificate**
trả lời câu này: nó là một public key cộng một identity (một hostname,
trong trường hợp khớp SNI của [`14-tls.md`](14-tls.md)) cộng một **digital signature**
— được ký không phải bởi chính server, mà bởi một **Certificate Authority
(CA)**, một bên thứ ba mà trình duyệt/OS của bạn đã tin tưởng sẵn.

Điều này tạo thành một **chain of trust**: OS của bạn đi kèm sẵn một danh
sách public key của **root CA** đáng tin cậy. Một root CA ký các
certificate **intermediate CA**, các certificate này lại ký các
certificate **leaf** (server) thật sự mà proxy của bạn trình ra. Xác minh
một certificate nghĩa là đi ngược chuỗi này — leaf được ký bởi
intermediate, intermediate được ký bởi một root bạn đã tin tưởng sẵn —
dùng cơ chế xác minh signature từ mục trước ở mỗi bước. Gotcha "chain tới
một CA đáng tin cậy yếu hơn nghe có vẻ" của [`07-security/03-mtls.md`](../07-security/03-mtls.md) hoàn
toàn nói về điều này: CA chỉ vouch cho *một identity*, không phải cho
*sự ủy quyền* — bất kỳ ai CA đó chịu ký cho đều có được một chain hợp lệ.

**PKI** (public key infrastructure) chỉ là thuật ngữ bao trùm cho toàn bộ
hệ thống này: các CA, các certificate, việc xác minh chain-of-trust, và
tooling (ACME/Let's Encrypt, được nhắc trong phần quản lý certificate của
[`14-tls.md`](14-tls.md)) phát hành và gia hạn chúng.

## Practice
1. Chạy `openssl s_client -connect example.com:443 -servername
   example.com` và đọc certificate chain nó in ra — xác định subject của
   leaf certificate (hostname), rồi lần ngược lên qua mọi intermediate
   tới root, và ghi chú field nào mang identity của bên phát hành vs
   identity của subject được ký.
2. Chạy `openssl x509 -in <a cert file> -noout -text` trên một
   certificate thật (export một cái từ bước 1 bằng `openssl s_client ... |
   openssl x509`) và xác định public key, signature, và các field
   issuer/subject đã nói ở trên.
3. Tính `sha256sum` trên một file, đổi một byte, rồi tính lại — xác nhận
   digest thay đổi hoàn toàn thay vì chỉ thay đổi một chút (tính chất
   avalanche của một hash function tốt).
4. Đọc mục "Algorithm confusion, concretely" trong
   [`07-security/02-jwt.md`](../07-security/02-jwt.md) khi HMAC vs signature còn mới trong đầu, và
   giải thích bằng lời của bạn vì sao đối xử với một RSA public key như
   một HMAC secret cho phép attacker giả mạo một token — gắn nó lại với
   "ai cũng xác minh được một signature; chỉ chủ sở hữu secret mới tính
   được HMAC" từ file này.
5. Tạo một cặp key RSA hoặc Ed25519 (`openssl genpkey` hoặc `ssh-keygen`),
   ký một file nhỏ bằng private key, và xác minh signature bằng public
   key — rồi thử xác minh bằng public key của một cặp key *khác* và xác
   nhận nó thất bại.
