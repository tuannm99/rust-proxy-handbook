# HTTP Cache

Các thuật toán eviction (LRU, LFU, ARC, TinyLFU) được bao quát ở
`13-algorithms/`; file này bao quát semantics caching riêng của proxy.

## What to learn

### Các directive Cache-Control quan trọng với một proxy
`max-age`, `s-maxage` (riêng cho shared/proxy cache, ghi đè `max-age` với proxy), `no-store` (không bao giờ cache), `no-cache` (cache nhưng luôn revalidate), và `private` (không cache trong một shared cache như proxy này — chỉ end client mới được phép). Một proxy bỏ qua `private`/`no-store` có thể làm rò rỉ response của người dùng này sang người khác.

Bốn cái nữa đáng để tâm:
- **`must-revalidate`**: khi đã stale, cache *không được* serve bản stale
  ngay cả khi origin không thể liên lạc được. Nó loại bỏ lối thoát
  stale-on-error bên dưới, đôi khi chính xác là những gì một response
  cần đúng đắn tuyệt đối.
- **`immutable`**: cái này sẽ không bao giờ thay đổi trong suốt vòng đời
  tươi mới của nó, nên đừng cả revalidate khi người dùng chủ động reload.
  Đi cùng với tên file asset content-hashed
  (`05-http-stack/05-static.md`).
- **`stale-while-revalidate=N`**: serve ngay bản stale và làm mới ở nền
  trong tối đa N giây. Đây là directive có đòn bẩy cao nhất cho một proxy
  cache — nó tách hoàn toàn latency người dùng thấy khỏi latency của
  origin.
- **`stale-if-error=N`**: serve stale thay vì lan truyền lỗi origin. Biến
  một lần origin sập thành nội dung hơi cũ thay vì một 5xx người dùng
  thấy.

Gotcha: phát ra một header `Age`, và tính nó đúng (thời gian trong cache
này cộng bất kỳ `Age` nào đã có từ một cache upstream). Cache và client
downstream dùng nó để tính độ tươi còn lại; bỏ nó khiến mọi cache bên dưới
bạn coi response stale của bạn là hoàn toàn mới.

### Freshness so với validation
Một response đã cache hoặc *tươi* (trong `max-age`) và có thể serve nguyên trạng, hoặc *stale* và phải được revalidate với origin (một conditional request dùng `ETag`/`Last-Modified`, xem `05-http-stack/05-static.md`) trước khi tái sử dụng. Serve dữ liệu stale mà không revalidate là một bug về đúng đắn, không phải một tối ưu.

Gotcha: chuyện gì xảy ra khi origin không gửi *bất kỳ* thông tin freshness
nào? RFC 9111 cho phép **heuristic freshness** — thường là 10% thời gian
kể từ `Last-Modified` — và một proxy âm thầm áp dụng nó bắt đầu cache các
response mà origin chưa bao giờ nói là cacheable. Điều đó tuân thủ chuẩn
nhưng vẫn là một bất ngờ với người viết upstream. Với một proxy cache, mặc
định "không có freshness tường minh nghĩa là đừng cache" là chính sách an
toàn hơn; biến heuristic caching thành opt-in theo từng route.

### Cái gì thậm chí đủ điều kiện để cache
Trước tất cả những điều trên: method phải là `GET` hoặc `HEAD` (không bao giờ cache một response `POST` chỉ dựa trên URL), và status code phải là một trong những cái spec cho phép cache theo mặc định — 200, 203, 204, 206, 300, 301, 404, 405, 410, 414, 501. Cache 404 và 301 là hợp pháp và có giá trị; cache một 500 thì không.

Gotcha: một request mang `Authorization` không được lưu response của nó
vào một *shared* cache trừ khi response cho phép tường minh
(`public`, `s-maxage`, hoặc `must-revalidate`). Bỏ qua kiểm tra này là con
đường trực tiếp tới việc serve response của một user đã authenticate cho
một user khác — bug nghiêm trọng nhất mà file này có thể ngăn chặn.

### Thiết kế cache key
Cache key ngây thơ là URL, nhưng để đúng đắn cần bao gồm các header được liệt kê trong `Vary` (ví dụ `Vary: Accept-Encoding` nghĩa là response gzip và không nén phải được cache riêng) và thường cả ngữ cảnh auth/session nếu response khác nhau theo từng người dùng — nếu không bạn serve response của user A cho user B.

```rust
// sketch: cache key must fold in Vary-listed request headers
struct CacheKey {
    method: Method,
    uri: String,
    vary_header_values: Vec<(String, String)>, // (header name, request's value) for each name in Vary
}
```

Gotcha: `Vary` là kẻ hủy diệt hit-rate khi tuân theo nghĩa đen. `Vary:
User-Agent` nghĩa là mỗi phiên bản browser có entry riêng và hit rate của
bạn sụp đổ về gần zero; `Vary: *` nghĩa là không bao giờ tái sử dụng.
Và giá trị `Accept-Encoding` ngoài đời là những chuỗi rất đa dạng nhưng
đều mang cùng vài nghĩa — normalize chúng thành một tập bucket nhỏ
(`gzip` / `br` / `identity`) trước khi keying, nếu không bạn phân mảnh
cache thành hàng chục entry tương đương nhau.

Gotcha: query string là một phần của key, và *thứ tự* của nó thường không
nên là vậy. `?a=1&b=2` và `?b=2&a=1` là cùng một resource với hầu hết
origin nhưng là hai entry với một cache ngây thơ. Sắp xếp parameter (và
quyết định tường minh về những cái không ảnh hưởng response, như tracking
parameter) khi xây key.

### Cache poisoning: tấn công input chưa được keyed
Nếu bất kỳ input nào ảnh hưởng *response* nhưng không phải một phần của *key*, một attacker kiểm soát input đó có thể đầu độc entry cho mọi người khác. Hình mẫu kinh điển: một origin phản chiếu `X-Forwarded-Host` vào URL tuyệt đối trong trang, proxy không đưa header đó vào key, nên một request của attacker với `X-Forwarded-Host: evil.com` cache một trang trỏ mọi khách truy cập sau đó tới domain của attacker.

Hai biện pháp phòng thủ, cả hai đều cần. Strip các header mà origin dù
sao cũng không nên thấy từ client (kỷ luật trust-boundary từ
`07-security/08-ip-filtering.md` và `07-security/01-auth.md`). Và coi bất
kỳ header nào bạn cố tình forward là một input của key trừ khi bạn đã xác
lập chắc chắn response không phụ thuộc vào nó.

Gotcha: "input chưa keyed" bao gồm cả những thứ không trông giống input —
các quirk của request method, port trong `Host`, path có trailing slash
hay không, và bất cứ gì chính proxy của bạn thêm vào trước khi forward.
Cách hệ thống để tìm chúng là thay đổi từng input một và diff response.

### Cache stampede
Khi một entry phổ biến hết hạn, mọi request đồng thời cho nó miss cùng lúc và tất cả đi tới origin — cache gây thiệt hại tối đa đúng lúc nó ngừng giúp ích. Request coalescing (single-flight) cộng `stale-while-revalidate` loại bỏ hoàn toàn vấn đề này, và một cold start sau một lần restart là cùng vấn đề đó cho mọi key cùng một lúc.

Xem `05-http-stack/08-cache-stampede.md`.

### Invalidation
Hết hạn theo thời gian (`max-age`) là trường hợp dễ. Invalidation tường minh (origin đẩy một lần purge, hoặc một lần ghi làm invalidate một lần đọc liên quan) là trường hợp khó mà mọi cache thật cuối cùng đều cần — lên kế hoạch cho một cơ chế purge-theo-key hoặc purge-theo-prefix ngay từ đầu thay vì gắn thêm sau.

Gotcha: với N instance proxy, mỗi cái giữ cache riêng, nên một lần purge
phải tới được tất cả chúng — và một purge endpoint mà bất kỳ client nào
cũng gọi được là một denial-of-service kiểu xả cache
(`07-security/09-ddos.md`). Authenticate đường purge, và chấp nhận rằng
việc lan truyền là eventually-consistent: thiết kế cho "purge tới mọi
instance trong vài giây", không phải tức thì.

Gotcha: invalidation theo tag ("purge mọi thứ gắn tag `user:42`") tổng
quát hóa tốt hơn nhiều so với purge-theo-URL, vì code thực hiện một lần
ghi biết nó đã thay đổi gì nhưng không biết những URL nào đã render nó.
Lưu một tập tag cho mỗi entry tốn ít và rất khó để thêm vào sau.

### Vị trí của một proxy cache so với một CDN
Một CDN cache ở edge, gần người dùng, trên nhiều origin. Cache của một L7 proxy (nếu có) nằm trực tiếp trước một origin/service, chủ yếu để chắn cho origin đó khỏi các request giống hệt nhau lặp lại — quy mô khác nhau, cơ chế giống nhau (RFC 9111).

## Practice
Làm theo thứ tự này.

1. Trong `labs/10-cache`, thêm một cache trong memory key theo
   method+URI chỉ cho `GET`. **Xong khi** một request giống hệt lần hai
   được serve từ cache mà không chạm origin (chứng minh bằng một counter
   phía origin).
2. Thêm các kiểm tra điều kiện: method, status code cacheable,
   `no-store`, `private`, và quy tắc `Authorization`. **Xong khi** một
   response đã authenticate không bao giờ được serve cho user thứ hai —
   viết một test fail với implementation bước 1 của bạn.
3. Thêm hỗ trợ `Vary` với normalization `Accept-Encoding` và sắp xếp
   query parameter. **Xong khi** hai client với `Accept-Encoding` khác
   nhau nhận hai entry riêng, và `?a=1&b=2` / `?b=2&a=1` dùng chung một.
4. Thêm freshness (`max-age`/`s-maxage`), header `Age`, và revalidation
   qua conditional request. **Xong khi** một entry stale kích hoạt đúng
   một conditional request và một `304` làm mới nó mà không truyền body.
5. Làm các bài tập của `05-http-stack/08-cache-stampede.md`. **Xong khi**
   500 request đồng thời cho một key vừa hết hạn tạo ra đúng một lần chạm
   origin, và `stale-while-revalidate` nghĩa là không cái nào phải chờ.
6. Thêm `stale-if-error`. **Xong khi** đưa origin hoàn toàn offline vẫn
   serve nội dung đã cache thay vì 5xx.
7. Dàn dựng một cuộc tấn công cache-poisoning: làm origin phản chiếu một
   header bạn forward nhưng không keying, đầu độc một entry, rồi lấy nó
   như một client khác. **Xong khi** cuộc tấn công thành công, và rồi
   **xong lần nữa khi** strip/keying header đó chặn được nó.
8. Thêm purge có authenticate, theo key và theo tag. **Xong khi** một lần
   purge theo tag evict mọi entry liên quan, một lần purge không
   authenticate bị reject, và chạy hai instance proxy cho thấy cả hai
   hội tụ sau một lần purge.
