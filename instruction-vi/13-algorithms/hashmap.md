# Hash Map: Chaining so với Open Addressing

Được dùng ở khắp nơi trong handbook này — route table, upstream pool,
rate-limit bucket, cache key. File này nói về những gì thực sự nằm bên
trong `HashMap` mà bạn dùng mặc định, và một gotcha production áp dụng
riêng cho trường hợp key của map trong một proxy thường đến từ attacker.

## What to learn

### Hai họ
**Chaining**: mỗi bucket giữ một danh sách (hoặc `Vec` nhỏ) chứa mọi entry
đã hash vào đó; một va chạm chỉ đơn giản làm danh sách dài thêm. Đơn giản,
suy giảm nhẹ nhàng, nhưng mỗi lần lookup sau chỉ số bucket đầu tiên là một
lần đuổi theo con trỏ — không thân thiện với cache.

**Open addressing**: mọi entry nằm trực tiếp trong mảng backing; khi va
chạm, dò tới một slot khác (linear, quadratic, hoặc qua một hash thứ hai)
cho tới khi tìm được slot trống. Không đuổi theo con trỏ, hành vi cache
tốt hơn nhiều — toàn bộ bảng là một vùng cấp phát liền mạch — nhưng cần
tombstone hoặc dịch chuyển ngược (backward-shifting) để xử lý việc xóa mà
không phá vỡ chuỗi dò của các entry được insert sau một va chạm.

### `HashMap` chuẩn của Rust thực chất là gì
Từ Rust 1.36, `std::collections::HashMap` được backing bởi `hashbrown`,
một bản port Rust của thiết kế SwissTable của Google: open addressing với
việc dò được tăng tốc bằng SIMD (một control byte cho mỗi slot cho phép
việc dò kiểm tra tới 16 slot mỗi lệnh SIMD) và dịch chuyển kiểu
robin-hood để giữ chuỗi dò ngắn. Đây là lý do "cứ dùng `HashMap`" thực sự
là lời khuyên tốt trong Rust cụ thể — nó không phải map chaining ngây thơ
mà một số ngôn ngữ khác dùng mặc định.

### Hash flooding: một không gian key do attacker chọn
Một `HashMap<IpAddr, TokenBucket>` (`13-algorithms/token-bucket.md`) hay
bất kỳ map nào có key đến từ dữ liệu client kiểm soát (header, query
param) có phân phối key được chọn bởi bất kỳ ai gửi request. Với một hàm
hash **không có key** (FxHash, một FNV thô, bất cứ thứ gì không có seed
ngẫu nhiên theo từng process), một attacker biết thuật toán hash có thể
chọn input khiến tất cả va chạm, làm suy giảm mọi thao tác về phía độ dài
chuỗi va chạm — O(n) cho mỗi lookup thay vì O(1), biến một hash map thành
chính một vector từ chối dịch vụ (`07-security/09-ddos.md`).

Hasher mặc định của Rust (SipHash, có key với một seed ngẫu nhiên sinh ra
mỗi process khi khởi động) được thiết kế riêng để chống DoS kiểu này: nếu
không biết key ngẫu nhiên của process, attacker không thể đoán trước input
nào sẽ va chạm. **Đây chính xác là lý do vì sao việc chuyển sang một
hasher nhanh hơn (`ahash`, `FxHash` — cả hai đều là lời khuyên hiệu năng
phổ biến) chỉ an toàn cho các key mà bản thân proxy kiểm soát** (một route
table nội bộ xây từ config tĩnh) và không an toàn cho key bắt nguồn từ
input client (source IP, giá trị header) trừ khi hasher nhanh hơn đó cũng
có key và được seed một cách không thể đoán trước — kiểm tra trước khi
đổi, đừng cho rằng "hash map nhanh hơn" là một chiến thắng miễn phí một
khi có liên quan tới key do client kiểm soát.

## Practice
1. Implement một hash map chaining và một hash map open-addressing đồ chơi
   (linear probing) trên cùng một kiểu key; so sánh thời gian lookup khi
   load factor tăng từ rỗng tới gần đầy.
2. Tái tạo hash flooding: chọn một hàm hash không key, xây một tập input
   mà tất cả đều map vào cùng một bucket, và đo việc lookup của map
   chaining suy giảm về phía O(n). Xác nhận `HashMap` mặc định của Rust
   với cùng các input được chế tạo đó không suy giảm theo cách tương tự.
3. Trong `labs/11-rate-limit`, benchmark map bucket theo từng IP với hasher
   mặc định của `HashMap` so với `ahash`/`FxHash`; rồi lặp lại nỗ lực
   flooding ở bước 2 nhắm cụ thể vào phiên bản hasher nhanh hơn để xem nó
   có còn kháng được hay không.
4. Kiểm toán các map trong `proxy/` (hoặc thiết kế chúng, nếu chưa viết)
   và phân loại mỗi cái theo việc key của nó được tin cậy (config nội bộ)
   hay không tin cậy (bắt nguồn từ client) — quyết định hasher cho mỗi cái
   dựa trên đó, không chỉ dựa trên tốc độ benchmark.
