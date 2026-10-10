# Phần toán đằng sau Cryptography của TLS

## What to learn

### Symmetric encryption như một permutation có key
Một block cipher như AES, về mặt toán học, là một họ permutation của một
block kích thước cố định, đánh index theo key — với mỗi key, encryption
là một bijection trên tập các block khả dĩ, và decryption là nghịch đảo
của nó. AES cụ thể chạy block qua nhiều round (10/12/14 tùy kích thước
key) của substitution (một lookup S-box cố định, cho confusion) và
permutation (xáo byte qua block, cho diffusion) — "symmetric encryption"
của [`01-network/06-crypto-basics.md`](../01-network/06-crypto-basics.md) đã gọi tên cái này; đây là cấu trúc
round thật khiến nó an toàn.

```text
AES round (đơn giản hóa): SubBytes -> ShiftRows -> MixColumns -> AddRoundKey
lặp lại N round, mỗi round trộn thêm key material vào block
```
Gotcha: *mode* của block cipher quan trọng như chính cipher — encrypt mỗi
block độc lập (mode ECB) lộ pattern (các block plaintext giống nhau ra
block ciphertext giống nhau), đó là lý do TLS dùng các mode như GCM —
encrypt một counter theo từng block (CTR mode) dưới một nonce duy nhất nên
các block plaintext giống nhau không bao giờ ra ciphertext giống nhau — và
thêm cả authentication (AEAD — authenticated encryption with associated
data).

### Diffie-Hellman: shared secret qua một kênh công khai, từ một bài toán khó
Hai bên mỗi bên chọn một số ngẫu nhiên private, trao đổi một giá trị công
khai suy ra từ nó (`g^a mod p` và `g^b mod p`), và mỗi bên có thể tính
cùng một shared secret (`g^(ab) mod p`) từ giá trị công khai của bên kia
và giá trị private của chính mình — mà không bao giờ truyền secret đó.
An toàn dựa trên bài toán discrete logarithm khó về tính toán: lấy lại
`a` từ `g^a mod p` (với `p` đủ lớn) là bất khả thi với các giải thuật đã
biết, dù tính `g^a mod p` từ `a` là dễ.

```text
Alice: chọn a, gửi A = g^a mod p
Bob:   chọn b, gửi B = g^b mod p
Alice tính: B^a mod p = g^(ba) mod p
Bob tính:   A^b mod p = g^(ab) mod p   -- cùng giá trị, không bao giờ truyền trực tiếp
```
Đây chính xác là chuyện xảy ra bên trong mỗi handshake TLS 1.3
([`01-network/19-tls.md`](../01-network/19-tls.md)) qua elliptic-curve Diffie-Hellman (ECDHE) — cùng
hình dạng toán học, trên điểm elliptic curve thay vì lũy thừa modular,
cho an toàn tương đương với key nhỏ hơn nhiều.

### RSA: hàm trapdoor từ độ khó của phân tích thừa số
An toàn của RSA dựa trên một bài toán khó khác: cho `n = p * q` với hai
số nguyên tố lớn, phân tích `n` ngược lại thành `p` và `q` là bất khả thi
về tính toán, dù nhân `p * q` để ra `n` là tầm thường. Sinh key chọn `p`,
`q`, tính `n = pq` và một cặp exponent public/private `(e, d)` thỏa
`e*d ≡ 1 mod φ(n)`; encryption là `c = m^e mod n`, decryption là
`m = c^d mod n` — trapdoor là việc tính `d` từ `e` cần biết `φ(n)`, cần
biết `p` và `q`, cần phân tích `n`.

Gotcha: TLS 1.3 đã loại bỏ hoàn toàn RSA key exchange — mọi handshake TLS
1.3 đều dùng (EC)DHE, vì nó cho forward secrecy (lộ một key dài hạn sau
này không lộ session key trong quá khứ, thứ static RSA key exchange của
TLS 1.2 không cung cấp). RSA chỉ còn tồn tại trong TLS 1.3 như một giải
thuật *chữ ký* certificate, không bao giờ dùng để suy ra session key.

### Hashing: một chiều, và vì sao đó là cả điểm mấu chốt
Một hàm hash cryptographic phải gần như không thể đảo (cho `H(x)`, tìm
bất kỳ `x` nào là bất khả thi) và chống collision (tìm hai input khác
nhau có cùng output là bất khả thi). HMAC (vốn từ vựng của
[`01-network/06-crypto-basics.md`](../01-network/06-crypto-basics.md)) xây một hash *có key* từ một hash không
key chính xác để việc tính một HMAC hợp lệ cần secret key, không chỉ cần
biết giải thuật hash — đây là thứ khiến HMAC dùng được như một
message-authentication code, và một hash thường không dùng được như vậy.

### Vì sao handbook này chỉ dừng ở "basics," và vì sao file này tồn tại
[`proxy`](../../proxy) không nên tự implement bất kỳ primitive cryptographic nào bằng
tay — [`01-network/06-crypto-basics.md`](../01-network/06-crypto-basics.md) và [`01-network/19-tls.md`](../01-network/19-tls.md) đều nói
rõ đây là việc của `rustls`, và crypto viết tay là một nguồn lỗ hổng thảm
khốc, âm thầm đã biết rõ. Giá trị của file này thuần túy là khả năng đọc
một trace handshake TLS hoặc một security advisory và hiểu *vì sao* một
primitive hoặc mode cụ thể an toàn hay không an toàn — không bao giờ để
tự viết một cái.

## Practice
1. Bằng tay, hoặc dùng máy tính cho số nhỏ, chạy qua một trao đổi
   Diffie-Hellman đồ chơi với `p`, `g`, và giá trị private nhỏ, và xác
   nhận cả hai bên tự tính ra cùng shared secret độc lập.
2. Bằng tay, sinh một cặp key RSA đồ chơi với hai số nguyên tố nhỏ (ví dụ
   `p=61, q=53`, ví dụ textbook), tính `n`, `φ(n)`, và một cặp `(e, d)`
   hợp lệ, rồi encrypt và decrypt một số message nhỏ.
3. Capture một handshake TLS 1.3 thật (`openssl s_client -connect ... -tls1_3`
   hoặc Wireshark) và xác định group key-exchange đang dùng (ví dụ
   `x25519`) — xác nhận nó là elliptic-curve Diffie-Hellman, không phải
   RSA key exchange.
4. Encrypt cùng một pattern block lặp lại bằng AES ở mode ECB và lại bằng
   mode GCM (dùng một crypto library, không viết tay AES) và so sánh
   pattern ciphertext bằng mắt — xác nhận ECB lộ sự lặp lại còn GCM không.
5. Đọc danh sách cipher suite mặc định của `rustls` và, cho mỗi entry,
   xác định giải thuật key-exchange, bulk cipher, và mode — nối mỗi cái
   lại với một phần trong file này.
