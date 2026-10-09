# Router Design

## What to learn

### Chiến lược matching
Quét tuyến tính một `Vec<Route>` là O(n) mỗi request nhưng dễ implement và ổn với vài route. Một router dạng radix/trie (thứ mà hầu hết router Rust production — `matchit`, router của axum — dùng) chia sẻ các tiền tố path chung trong một cây nên lookup xấp xỉ O(độ dài path), và xử lý được path parameter (`/users/:id`) cùng wildcard mà không tốn chi phí backtracking như regex.

Bản thân các cấu trúc dữ liệu này nằm ở [`13-algorithms/trie.md`](../13-algorithms/trie.md) (matching
tiền tố theo segment) và [`13-algorithms/radix-tree.md`](../13-algorithms/radix-tree.md) (dạng nén mà router
production thực sự ship, bao gồm cả phần khó là tách longest-common-prefix
khi insert). Đọc hai file đó để biết cách implement; file này nói về những
gì một *proxy* cần thêm trên nền chúng.

### Normalize path trước khi match, nếu không bạn có một bug bảo mật
Đây là điều quan trọng nhất trong file này, và nó không hiển nhiên: path
bạn dùng để match và path mà upstream cuối cùng resolve phải là cùng một
path, nếu không attacker sống trong sự khác biệt đó.

Hãy hình dung một proxy cấu hình để block `/admin` và cho phép
`/public/*`. Một request tới `/public/../admin` match `/public/*` theo
cách so khớp segment ngây thơ — và một upstream normalize [`..`](../..) trước khi
serve sẽ trả về `/admin`. Quyết định routing của proxy và cách diễn giải
của upstream không khớp nhau, và access control là nạn nhân. Đây là cùng
họ parser-differential như [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md), áp dụng
cho path thay vì framing.

Các biến thể cần xử lý, tất cả đều đã từng gây ra bypass thật:
- **Dot segment**: `/a/../b`, `/a/./b` — resolve chúng trước khi match.
- **Ký tự phân tách đã encode**: `%2f` là `/` sau khi decode. Quyết định
  tường minh xem một slash đã encode có được coi là path separator hay
  không (hầu hết upstream nói có, hầu hết router ngây thơ nói không) và
  reject thay vì đoán nếu không chắc.
- **Double-encoding**: `%252e%252e%252f` — decode tới một độ sâu cố định,
  có ghi rõ, khớp với những gì upstream của bạn làm
  ([`07-security/06-waf.md`](../07-security/06-waf.md) có cùng thảo luận này).
- **Slash trùng lặp**: `//admin` so với `/admin` — gộp chúng lại.
- **Trailing slash**: `/admin/` so với `/admin`. Chọn một dạng canonical
  và redirect dạng kia thay vì đăng ký cả hai.
- **Chữ hoa/thường**: path phân biệt hoa thường trong HTTP và trên
  filesystem Linux, nhưng không phải trên macOS/Windows — một upstream
  chạy trên filesystem không phân biệt hoa thường sẽ serve `/ADMIN` cho
  một rule viết là `/admin`.

Quy tắc: normalize một lần, sớm, thành một dạng canonical; route trên dạng
đó; và forward *đúng dạng đó* lên upstream thay vì byte gốc, để không còn
cách diễn giải thứ hai nào có thể bất đồng.

### Path parameter & precedence
Khi cả `/users/:id` lẫn `/users/me` đều có thể match `/users/me`, router cần một quy tắc precedence tất định — segment tĩnh thắng segment tham số thắng wildcard. Làm sai chỗ này nghĩa là kết quả match trở nên phụ thuộc vào thứ tự đăng ký, một nguồn bug tinh vi khi bảng route lớn dần.

Gotcha: phát hiện xung đột thật sự ở *thời điểm đăng ký*, không phải thời
điểm match. Hai route không bao giờ phân biệt được (cùng một pattern đăng
ký hai lần, hoặc `/a/:x` và `/a/:y`) là một lỗi cấu hình, và fail ngay lúc
khởi động — hoặc lúc reload config, giữ nguyên bảng cũ đang chạy
([`09-architecture/03-config.md`](../09-architecture/03-config.md)) — tốt hơn là âm thầm chọn một cái và để
lại một endpoint không bao giờ với tới được.

### Method dispatch
Routing là hai chiều: path *và* method. Một bug phổ biến là match được path và trả về 404 thay vì 405 (Method Not Allowed) khi path tồn tại nhưng method thì không — proxy và API được kỳ vọng phân biệt hai cái này.

Hai trường hợp mà mô hình hai chiều thường bỏ sót:
- **`HEAD` phải hoạt động ở bất cứ đâu `GET` hoạt động.** RFC 9110 định
  nghĩa `HEAD` là `GET` không có body. Một router yêu cầu đăng ký `HEAD`
  tường minh sẽ trả 405 cho một request hoàn toàn hợp lệ, và các công cụ
  monitoring dùng `HEAD` liên tục.
- **`OPTIONS`** là một CORS preflight, và browser gửi nó *không kèm*
  credential trước request thật. Nếu proxy áp auth
  ([`07-security/01-auth.md`](../07-security/01-auth.md)) lên preflight, mọi lời gọi cross-origin từ
  browser sẽ fail theo kiểu trông giống CORS misconfiguration nhưng thực
  ra là bug thứ tự auth.

Gotcha: response 405 phải kèm một header `Allow` liệt kê các method thực
sự match. Đây là yêu cầu của spec và là thứ khiến sự khác biệt có thể debug
được.

### Routing trên nhiều hơn chỉ path
Một proxy route trên toàn bộ request, không chỉ path: `Host` (xem
[`05-http-stack/12-vhost-routing.md`](12-vhost-routing.md)), header tùy ý (canary routing theo
`X-Version`, [`09-architecture/06-canary-deploy.md`](../09-architecture/06-canary-deploy.md)), đôi khi weight cho
traffic splitting.

Gotcha: `Host` không phải một thứ duy nhất. Trong HTTP/1.1 nó là header
`Host`; trong HTTP/2 và HTTP/3 nó là pseudo-header `:authority`; và dưới
TLS còn có tên SNI từ handshake ([`01-network/19-tls.md`](../01-network/19-tls.md)), thứ mà client
chọn *trước khi* gửi bất kỳ cái nào trong số đó. Chúng đều có thể bất
đồng — một attacker connect với SNI `public.example.com` rồi gửi
`Host: admin.internal`. Quyết định cái nào là authoritative cho routing,
validate rằng các cái còn lại khớp với nó, và reject request khi chúng
không khớp.

### Middleware như một phép hợp thành, không phải một trường hợp đặc biệt
Hãy nghĩ một route handler như một `tower::Service<Request> -> Response`. Middleware (auth, rate limiting, logging) chỉ là một `Service` khác bọc lấy cái bên trong — đây là lý do [`07-security/01-auth.md`](../07-security/01-auth.md) và [`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md) cắm vào cùng một abstraction router thay vì bị gắn thêm riêng lẻ.

```rust
// sketch: router as a Vec of (matcher, handler), method-aware
struct Route {
    method: Method,
    matcher: PathMatcher, // static segments + :param + wildcard
    handler: Handler,
}
```

Câu hỏi về thứ tự theo sau đó: middleware phải chạy *trước* routing
(connection limit, IP filtering — bạn không thể route trước) so với
middleware cần route đã tồn tại (auth policy theo từng route, rate limit
theo từng route). Sự phân chia đó là cấu trúc thật của pipeline, và
[`09-architecture/01-components.md`](../09-architecture/01-components.md) là nơi nó được thiết kế.

### Routing trong reverse proxy so với API server
Một API server route tới một handler function; một reverse proxy ([`06-proxy/`](../06-proxy)) route tới một *upstream pool* — "handler" là "forward request này tới load balancer của service X". Logic matching (path prefix, host header, routing theo header) là cùng một bài toán, chỉ khác hành động cuối cùng.

Một hệ quả riêng cho trường hợp proxy: bảng route thay đổi lúc runtime
([`09-architecture/03-config.md`](../09-architecture/03-config.md), [`labs/13-hot-reload`](../../labs/13-hot-reload)) trong khi request
đang bay. Xây bảng như một cấu trúc immutable được swap atomic
(`arc_swap`, như trong [`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md)) thay vì một
cấu trúc bị mutate dưới lock — routing nằm trên hot path của mọi request,
và nó không bao giờ nên chờ một lần cập nhật config.

Gotcha: quyết định router làm gì với prefix đã match khi forward. Strip nó
đi (`/api/v1/users` → `/users` phía upstream) là phổ biến và là một chỗ
thứ hai nơi path proxy match và path upstream thấy khác nhau — áp cùng kỷ
luật như phần normalization, và làm việc rewrite tường minh theo từng
route thay vì ngầm định.

## Practice
Làm theo thứ tự này.

1. Trong [`labs/03-router`](../../labs/03-router), implement một route matcher tuyến tính
   (method + path chính xác) nối vào hyper handler. **Xong khi** hai path
   khác nhau route tới hai handler khác nhau.
2. Viết các test path-confusion trước khi thêm normalization: request
   `/public/../admin`, `//admin`, `/admin/`, `/%2e%2e/admin`, và
   `/pub%2f../admin` nhắm vào một router cho phép `/public/*` và block
   `/admin`. **Xong khi** bạn xác nhận ít nhất một trong số đó chạm tới
   route bị block — bạn cần một baseline đang fail.
3. Thêm path normalization (resolve dot-segment, gộp slash, độ sâu decode
   có ghi rõ, trailing slash canonical) áp trước khi match *và* dùng cho
   path được forward. **Xong khi** mọi test ở bước 2 đều bị block, và
   upstream nhận được path đã normalize.
4. Thêm path parameter và wildcard với precedence tường minh, và phát
   hiện xung đột lúc đăng ký. **Xong khi** `/users/me` thắng
   `/users/:id` bất kể thứ tự đăng ký, và đăng ký một cặp thật sự mơ hồ
   fail ngay lúc khởi động.
5. Thêm method dispatch với 404 và 405 đúng cùng header `Allow`, cộng với
   `HEAD` fallback về route `GET` và `OPTIONS` bỏ qua auth. **Xong khi**
   `HEAD` trên một route chỉ có `GET` trả về 200 không body, và một CORS
   preflight thành công mà không cần credential.
6. Thay router tuyến tính bằng radix tree từ [`13-algorithms/radix-tree.md`](../13-algorithms/radix-tree.md).
   **Xong khi** toàn bộ test suite pass không đổi và latency lookup phẳng
   từ 10 tới 1000 route.
7. Refactor handler đứng sau một trait kiểu `Service` và thêm middleware
   logging. **Xong khi** cùng một middleware bọc được cả một handler nội
   bộ và (về sau) một route proxy mà không cần sửa đổi.
8. Thêm validate tính nhất quán `Host`/`:authority`/SNI. **Xong khi** một
   request có `Host` bất đồng với SNI bị reject, và bạn có thể nói rõ
   nguồn nào router của bạn coi là authoritative.
9. Khi tới [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), làm cho một route đã match resolve
   tới một tên upstream pool, với việc rewrite prefix theo từng route
   tường minh, và swap bảng atomic khi reload. **Xong khi** một lần reload
   bảng route dưới tải đồng thời tạo ra zero request bị rớt hoặc route
   sai.
