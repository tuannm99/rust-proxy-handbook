# Networking Fundamentals

## Trạng thái: ngoại lệ duy nhất của "không phải tutorial"

Mọi file khác trong handbook này giả định bạn đã có baseline và chỉ dạy
góc nhìn liên quan tới proxy — giống cách `03-rust/` giả định bạn đã biết
cú pháp Rust và chỉ đào sâu ownership/async/unsafe. File này và năm file
anh em bên dưới thì khác, một cách có chủ đích: kiến thức nền về mạng
không phải thứ hầu hết người đọc đã sẵn có theo cách cú pháp Rust thường
là vậy, và `07-socket.md`/`08-tcp.md`/`09-dns.md` không thể đọc hiểu nếu
thiếu nó. Bắt đầu từ đây nếu các thuật ngữ như "port," "packet,"
"handshake," hay "NAT" chưa có nghĩa chính xác với bạn — bỏ qua cả nhóm
này nếu chúng đã quen thuộc.

## What to learn

### Hai process, nói chuyện qua byte
Lột bỏ mọi từ viết tắt thì networking chỉ là: hai chương trình, có thể
nằm trên các máy khác nhau, trao đổi byte qua dây (hoặc sóng radio). Một
bên lắng nghe, một bên kết nối tới. Mọi thứ khác trong thư mục này — TCP,
TLS, HTTP, DNS — là một tập luật đặt chồng lên "gửi byte, nhận byte" để
hai chương trình được viết độc lập, trên hai máy tính khác nhau, đồng ý
với nhau về ý nghĩa của những byte đó.

Về mặt cấu trúc, `proxy/` chỉ là một chương trình đứng ở giữa: nó là
"server" đối với bất kỳ ai kết nối tới nó, và là "client" đối với bất kỳ
thứ gì nó kết nối tới tiếp theo. Mỗi file protocol trong thư mục này mô tả
hành vi từ một hoặc cả hai vai trò đó.

### Sáu mảnh ghép, và mỗi mảnh nằm ở đâu
Lớp giới thiệu này được chia thành sáu file ngắn thay vì một file dài,
vì mỗi mảnh thực sự là một ý tưởng tách biệt và sau này bạn sẽ muốn quay
lại từng phần riêng lẻ thay vì đọc lại cả một bức tường chữ:

- **`02-addressing.md`** — cách một host và một process trên đó được định
  danh: IP address, port, CIDR notation, và NAT (vì sao địa chỉ mà một
  packet đến với thường không phải địa chỉ nó được gửi từ).
- **`03-byte-streams.md`** — thứ TCP thực sự đưa cho chương trình của bạn
  (một stream, không phải các message), TCP vs UDP, và handshake như một
  pattern lặp lại.
- **`04-latency-throughput.md`** — bốn con số người ta hay lẫn lộn:
  latency, bandwidth, throughput, RTT — và vì sao một kết nối "nhanh" vẫn
  có thể cảm giác chậm.
- **`05-proxy-taxonomy.md`** — forward proxy vs reverse proxy vs NAT
  gateway vs load balancer vs L4 vs L7. Repo này xây một điểm cụ thể trong
  không gian đó, và file này trả lời thẳng "cái nào, và vì sao".
- **`06-crypto-basics.md`** — mã hóa symmetric vs asymmetric, hashing,
  HMAC, digital signature, certificate/PKI. Không phải cryptography như
  một ngành học — chỉ đủ để handshake trong `13-tls.md` và chữ ký trong
  `07-security/02-jwt.md` không còn là phép màu.

Đọc chúng theo thứ tự đó một lần; sau đó, coi mỗi file là một điểm tra cứu
độc lập.

## Practice
1. Chạy `ss -tlnp` (hoặc `netstat -tlnp`) trên máy của bạn và xác định mọi
   listening port cùng process nào sở hữu nó — xác nhận bạn giải thích
   được, với ít nhất ba trong số đó, vì sao chương trình đó chọn port đó.
2. Chạy `curl -v http://example.com` và xác định, trong output, chỗ nào
   TCP handshake diễn ra, chỗ nào HTTP request được gửi, và chỗ nào là
   response header vs body — curl gắn nhãn từng giai đoạn.
3. Đọc năm file anh em theo thứ tự, rồi quay lại đây và giải thích, mỗi
   ý một câu: một socket là gì, vì sao TCP cảm giác giống một file, NAT
   làm gì với source address, và `labs/05-reverse-proxy` thuộc loại proxy
   nào.
