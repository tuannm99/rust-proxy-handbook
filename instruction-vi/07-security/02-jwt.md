# JWT Validation

Xác thực một bearer token ở edge của proxy. [`07-security/01-auth.md`](01-auth.md) nói về
việc này nằm ở đâu trong pipeline và proxy làm gì với identity sau đó; file
này nói về việc làm đúng chính bản thân việc xác thực.

## What to learn
### JWT validation
Một JWT là một cấu trúc JSON đã ký, mã hóa base64url
(`header.payload.signature`). Xác thực nó ở edge của proxy nghĩa là: xác
minh signature với một key đã biết (shared secret HS256 hoặc public key
RS256/ES256 — không bao giờ chấp nhận `alg: none`, và không bao giờ để
`alg` header của chính token chọn thuật toán xác minh), sau đó kiểm tra
claim: `exp` (đã expire?), `nbf` (chưa tới lúc hợp lệ?), `aud`/`iss` (được
phát hành cho service này?).

```rust
struct Claims {
    exp: u64,
    nbf: Option<u64>,
    aud: String,
    sub: String,
}

fn validate_claims(claims: &Claims, expected_aud: &str, now: u64) -> Result<(), &'static str> {
    if claims.exp <= now { return Err("expired"); }
    if let Some(nbf) = claims.nbf { if now < nbf { return Err("not yet valid"); } }
    if claims.aud != expected_aud { return Err("wrong audience"); }
    Ok(())
}
```
Gotcha: xác minh signature và kiểm tra claim là hai bước tách biệt — một
thư viện "parse" một JWT mà không yêu cầu bạn gọi verify tường minh có thể
đưa cho bạn các claim từ một token mà signature chưa từng được kiểm tra.
Trong Rust, ưu tiên một crate được maintain (`jsonwebtoken`) thay vì tự
viết tay HMAC/RSA.

### Algorithm confusion, cụ thể là gì
"Không bao giờ để `alg` header của chính token chọn thuật toán xác minh"
xứng đáng được giải thích bằng chính cuộc tấn công thật, vì nó cho thấy vì
sao cách sửa lại có hình dạng như vậy.

Service của bạn xác minh RS256 bằng một key *public* — public theo định
nghĩa, nên kẻ tấn công có nó. Kẻ tấn công viết lại header thành
`{"alg":"HS256"}` và ký token bằng cách dùng bytes của chính public key đó
làm HMAC secret. Một verifier đọc `alg` từ token và dispatch dựa trên đó sẽ
gọi "HMAC-verify với key đã cấu hình", key material khớp, và token giả mạo
được xác thực thành công. Chỉ cần đổi một header đã biến một phép kiểm tra
signature thành một con dấu đóng cho có.

Cách sửa không phải là "từ chối `alg: none`" — mà là pin cứng thuật toán
được chấp nhận trong *cấu hình của bạn* và bỏ hoàn toàn tuyên bố của token
về chính nó. `jsonwebtoken` với
`Validation { algorithms: vec![Algorithm::RS256], .. }` làm đúng việc này;
lỗi xuất hiện khi ai đó "tốt bụng" truyền thẳng algorithm từ header đã
parse vào validator.

Gotcha: quy tắc tương tự mở rộng ra mọi trường header do attacker kiểm
soát. `kid` (key ID) được dùng để chọn key — nếu bạn dùng nó như một
filesystem path hay một database key mà không validate, đó là path
traversal hoặc injection với vài bước nữa. `jku`/`x5u` chỉ định một *URL*
để fetch key — tin tưởng chúng là một primitive server-side request
forgery, đồng thời còn cho phép attacker tự cung cấp verification key. Chỉ
chấp nhận `kid` như một lookup vào một tập key cố định; không bao giờ tin
`jku`/`x5u`.

### Xoay vòng key và JWKS
Các identity provider trong production xoay vòng signing key và công bố
chúng tại một endpoint JWKS. Proxy fetch tập key đó, cache nó, và tra cứu
đúng key theo `kid`. Hai kiểu thất bại theo sau trực tiếp:

**Cơn stampede unknown-kid.** Khi xoay vòng xảy ra, token đến với một
`kid` bạn chưa có, và cách implement tự nhiên là refetch JWKS để tìm nó.
Một attacker gửi token với các giá trị `kid` ngẫu nhiên sẽ khiến bạn thực
hiện một lượt fetch JWKS outbound cho mỗi request — bạn đã vô tình xây một
bộ khuếch đại request nhắm vào chính identity provider của mình, thứ sẽ bắt
đầu rate-limit bạn, và tại thời điểm đó *toàn bộ* auth thất bại. Rate-limit
chính việc refetch (tối đa một lần mỗi N giây bất kể bao nhiêu kid lạ đến)
và trả 401 trong lúc chờ.

**Fetch thất bại.** Nếu JWKS không thể truy cập, fail static trên tập key
đã cache ([`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md) — cùng nguyên tắc): tiếp tục
xác thực bằng key tốt cuối cùng đã biết thay vì từ chối toàn bộ traffic.
Một lần xoay vòng bạn bỏ lỡ sẽ tạo ra 401 cho các token thực sự mới; một
key cache rỗng tạo ra 401 cho *mọi thứ*.

### Clock skew
`exp` và `nbf` là các timestamp tuyệt đối so sánh với đồng hồ của bạn, và
đồng hồ của bạn không khớp hoàn toàn với đồng hồ của issuer. Không có
leeway, một token vừa được tạo vài mili giây trước có thể fail `nbf` trên
một server chạy chậm hơn vài giây, và các lỗi 401 kết quả là không liên
tục, không tái hiện được, và không tương quan với bất cứ điều gì mà một
application developer có thể nhìn thấy.

Cho phép một khoảng leeway nhỏ (30-60 giây là quy ước) trên cả `exp` lẫn
`nbf`. Lưu ý sự bất đối xứng về rủi ro: leeway trên `nbf` không tốn gì cả,
leeway trên `exp` kéo dài tuổi thọ của một token đã expire thêm đúng
khoảng đó — điều này ổn ở mức 60 giây và không ổn ở mức một giờ.

Gotcha: leeway che giấu clock drift chứ không sửa nó. Theo dõi độ lệch thực
tế ([`08-observability/02-metrics.md`](../08-observability/02-metrics.md)); một server trôi vượt quá leeway của
bạn sẽ fail mọi token cùng một lúc, và bạn muốn có cảnh báo trước khi điều
đó xảy ra.

### Revocation: thứ mà JWT không làm được
Một token đã ký không trạng thái là hợp lệ cho tới `exp` vì việc xác thực
nó không cần bất kỳ server state nào — đó chính là toàn bộ luận điểm về
hiệu năng của JWT, và nó cũng có nghĩa là bạn không thể thu hồi một token.
Một token bị lộ, một session đã logout, một nhân viên vừa bị sa thải: tất
cả vẫn xác thực được cho tới khi expire.

Các câu trả lời thực tế, theo chi phí tăng dần: giữ `exp` ngắn (vài phút,
kèm luồng refresh token để renew), duy trì một denylist các giá trị `jti`
đã bị thu hồi (điều này đưa shared state trở lại, nhưng chỉ cho một tập nhỏ
các token bị thu hồi tường minh), hoặc chấp nhận cửa sổ rủi ro đó một cách
có chủ đích và ghi lại nó. Chọn "expiry ngắn" không phải là trốn tránh vấn
đề — đó là câu trả lời chuẩn — nhưng nó phải là một quyết định thực sự, vì
mặc định `exp` 24 giờ nghĩa là một cửa sổ bị lộ 24 giờ.

## Practice
Làm lần lượt theo thứ tự sau.

1. Trong [`proxy`](../../proxy), thêm JWT validation bằng `jsonwebtoken`: thuật toán được
   pin cứng, signature + `exp`/`aud`, ngược lại trả 401. **Xong khi** một
   token hợp lệ đi qua và một token có payload bị sửa thì fail.
2. Tự dàn dựng cuộc tấn công algorithm-confusion nhắm vào chính endpoint
   của bạn: lấy public key RS256 của bạn, ký một token dùng nó như một
   HS256 secret, và gửi đi. **Xong khi** nó bị từ chối — và, để chứng minh
   bài test là thật, tạm thời cấu hình validator để chấp nhận `alg` của
   chính token và xem token giả mạo thành công.
3. Đưa vào các giá trị `kid` chứa [`../`](../..) và một `jku` trỏ tới một URL bạn
   kiểm soát. **Xong khi** không cái nào được tin — `kid` chỉ resolve vào
   tập key cố định của bạn và không có outbound fetch nào cho `jku`.
4. Thêm việc fetch JWKS với tra cứu theo `kid`, rate limiting refetch, và
   fail-static khi fetch thất bại. **Xong khi** 1000 request với các giá
   trị `kid` ngẫu nhiên chỉ tạo ra tối đa một lượt fetch JWKS outbound, và
   việc block (blackhole) endpoint JWKS vẫn để các key hiện có tiếp tục hoạt
   động.
5. Thêm leeway cho clock-skew và một metric đo skew. **Xong khi** một token
   được tạo 2 giây trong tương lai vẫn xác thực được, và metric của bạn báo
   cáo đúng độ lệch thực tế so với issuer.
6. Quyết định và ghi lại chiến lược revocation của bạn. **Xong khi** hoặc
   `exp` đủ ngắn để bạn có thể nói rõ cửa sổ bị lộ tính bằng phút, hoặc một
   denylist `jti` được thực thi — và lựa chọn đó được ghi lại ở nơi config
   sống.
