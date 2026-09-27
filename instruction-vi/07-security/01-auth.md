# Authentication

Auth nằm ở đâu trong pipeline của proxy, và proxy làm gì với một identity
một khi đã có nó. Hai cơ chế có file riêng của mình:
`07-security/02-jwt.md` (bearer token) và `07-security/03-mtls.md` (client
certificate). Chúng thường được kết hợp — mTLS xác thực *service* đang gọi,
JWT xác thực *user* nằm trên đó.

## What to learn
### Auth nên nằm ở đâu trong pipeline
Auth nên chạy càng sớm càng tốt trong pipeline component (xem
`09-architecture/01-components.md`: ngay sau khi routing xác định route nào
áp dụng auth policy nào, trước bất kỳ upstream call hay xử lý tốn kém nào
như WAF body inspection). Từ chối request chưa xác thực/không hợp lệ trước
khi chúng tiêu tốn capacity của upstream.

"Sau routing" không phải ngẫu nhiên — policy là theo từng route, nên bạn
phải biết route trước khi biết policy nào áp dụng. Thứ tự đó có một hệ quả
đáng lên kế hoạch trước: bất cứ thứ gì chạy *trước* routing (IP filtering,
connection limit) không thể phụ thuộc vào identity, và bất cứ thứ gì cần
identity buộc phải chạy sau khi router đã làm xong việc của nó.

### Fail closed với route chưa cấu hình
Một request không khớp route nào, hoặc khớp một route mà auth policy chưa
bao giờ được cấu hình, không được rơi xuống trạng thái "không cần auth" —
điều đó biến một lỗi gõ config thành một cánh cửa mở.

Làm cho kiểu policy là non-optional để compiler buộc bạn phải quyết định:

```rust
enum AuthPolicy {
    Public,             // một variant bạn viết có chủ đích
    Jwt { audience: String },
    MutualTls { allowed_sans: Vec<String> },
    Both { .. },
}

struct Route {
    // ...
    auth: AuthPolicy,   // không phải Option<AuthPolicy>
}
```
Gotcha: đây là sự khác biệt giữa việc "chúng ta quên cấu hình auth" là một
compile error hay là một sự cố production. `Option<AuthPolicy>` với một
nhánh `None => allow` là chính con bug đó, chỉ viết theo cách trông có vẻ
có chủ đích.

### Truyền identity lên upstream — và loại bỏ nó trước
Đây là nửa đặc thù-proxy của auth, và cũng là phần hay bị làm sai nhất. Sau
khi proxy xác thực identity, upstream cần biết caller là ai, theo quy ước
qua một header (`X-User-Id`, `X-Auth-Subject`). Upstream sau đó tin tưởng
header đó, vì "proxy đã set nó".

Nghĩa là: **nếu client có thể gửi header đó và proxy chuyển tiếp nó
nguyên vẹn, client được xác thực như bất kỳ ai họ muốn.** Kẻ tấn công không
cần phá vỡ JWT validation của bạn — họ chỉ cần bỏ qua nó, gửi
`X-User-Id: admin` mà không kèm token nào cả, và proxy của bạn ngoan ngoãn
chuyển tiếp nó.

Quy tắc là vô điều kiện: loại bỏ mọi identity header khỏi request inbound
*trước khi* auth chạy, sau đó tự set nó từ các claim đã xác thực. Loại bỏ
theo kiểu allowlist (xóa bất cứ thứ gì thuộc namespace identity của bạn),
không phải denylist các header bạn nhớ được.

```rust
// trước auth, vô điều kiện:
for name in IDENTITY_HEADERS {          // X-User-Id, X-Auth-*, v.v.
    req.headers_mut().remove(name);
}
// sau khi xác thực thành công:
req.headers_mut().insert("x-user-id", claims.sub.parse()?);
```
Gotcha: đây là cùng một lỗi trust-boundary như `X-Forwarded-For` trong
`07-security/08-ip-filtering.md`. Bất kỳ header nào hạ tầng của bạn coi là
tin cậy đều phải bị loại bỏ ở edge, mỗi lần, trên mọi path — kể cả error
path và bất kỳ route nào bỏ qua auth.

Gotcha: việc loại bỏ phải xảy ra tại một điểm duy nhất, sớm, cùng chỗ với
việc loại bỏ hop-by-hop header (`05-http-stack/02-hop-by-hop-headers.md`),
không phải bên trong module auth. Một route cấu hình `Public` bỏ qua hoàn
toàn module auth — và nếu việc loại bỏ nằm ở đó, route này chuyển tiếp
thẳng identity header giả mạo.

### So sánh secret phải mất thời gian không đổi
Bất kỳ path nào so sánh một API key tĩnh, một HMAC digest, hay một shared
secret đều phải dùng constant-time comparison (`ConstantTimeEq` của
`subtle`), không phải `==`. Một phép so sánh short-circuit trả về nhanh hơn
khi tìm thấy sai lệch càng sớm, điều này để lộ tiền tố đúng qua timing —
có thể khôi phục từng byte một.

Gotcha: một crate JWT tốt đã làm điều này bên trong signature verification
rồi. Đoạn code dễ bị tấn công gần như luôn là fallback path tự viết tay —
kiểu "API key đơn giản" mà ai đó thêm vào cho một tích hợp nội bộ.

### Authentication không phải là authorization
Việc proxy trả lời "đây là ai?" không trả lời "họ có được làm việc này
không?". Authorization thô (route này yêu cầu scope này hay SAN này) nên
nằm ở edge vì nó cho phép từ chối với chi phí thấp; authorization chi tiết
(user này có được sửa *chính document này* không) cần dữ liệu mà chỉ
upstream mới có.

Gotcha: một proxy thực thi các rule thô có thể khiến upstream chủ quan bỏ
qua kiểm tra của chính nó — rồi một caller nội bộ vượt qua proxy sẽ không
có sự thực thi nào cả. Coi authorization ở edge là defense in depth, và nói
rõ điều đó với các team đứng sau nó.

## Practice
Làm lần lượt theo thứ tự sau.

1. Biến route auth policy thành một enum non-optional. **Xong khi** thêm
   một route mới mà không chỉ định policy khiến compile lỗi thay vì mặc
   định thành public.
2. Thêm việc loại bỏ identity header ở cùng điểm sớm với loại bỏ
   hop-by-hop, và inject sau khi xác thực thành công. **Xong khi** một
   request mang `X-User-Id: admin` và không có token bị từ chối với 401 và
   upstream không nhận được `X-User-Id` nào cả — viết phiên bản passthrough
   trước và xác nhận upstream thấy `admin`, để bạn đã tận mắt thấy lỗ hổng.
3. Xác minh việc loại bỏ trên các path bỏ qua auth. **Xong khi** một route
   cấu hình `Public` vẫn loại bỏ identity header giả mạo, và path 404 cũng
   vậy.
4. Implement các cơ chế: `07-security/02-jwt.md` cho bearer token,
   `07-security/03-mtls.md` cho client cert. **Xong khi** một route cấu
   hình `Both` yêu cầu cả cert hợp lệ *lẫn* token hợp lệ.
5. Thêm constant-time comparison vào bất kỳ path nào dùng static-secret.
   **Xong khi** một bài test timing qua nhiều mẫu không thể phân biệt một
   key sai-byte-đầu với một key sai-byte-cuối.
6. Thêm một test về thứ tự trong pipeline. **Xong khi** nó chứng minh một
   JWT không hợp lệ bị từ chối trước khi chọn upstream *và* trước khi WAF
   body inspection chạy.
