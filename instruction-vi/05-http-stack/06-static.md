# Static File Serving

## What to learn

### Zero-copy file sending
Serve một static file theo cách ngây thơ nghĩa là: đọc toàn bộ file vào một buffer userspace, rồi viết buffer đó ra socket — tốn thêm hai lần copy và hai lần context switch không cần thiết. `sendfile(2)` (xem [`02-linux/18-zerocopy.md`](../02-linux/18-zerocopy.md)) copy dữ liệu kernel-tới-kernel, bỏ qua userspace hoàn toàn; trên Linux, `tokio-uring`/`io_uring` ([`02-linux/15-io_uring.md`](../02-linux/15-io_uring.md)) có thể làm điều tương tự bất đồng bộ với overhead syscall thấp hơn `sendfile` dựa trên epoll.

Gotcha: `sendfile` nhanh khi page cache *hit* và block khi miss
([`16-kernel/08-page-cache.md`](../16-kernel/08-page-cache.md)). Trong một async runtime, một `sendfile`
block trên một file nguội làm đứng toàn bộ worker thread và mọi connection
khác nó đang đa hợp — nên "tối ưu hiển nhiên" đó biến một lần đọc file
chậm thành một đợt latency spike trên các request không liên quan. Đây
chính xác là lý do `tokio::fs` dispatch tới một blocking thread pool thay
vào đó: nó tốn một lần copy nhưng giữ reactor phản hồi được.

Quy tắc thực tế: zero-copy có lợi khi working set đang nóng và gây hại khi
không. Đo đạc với một working set lớn hơn RAM trước khi cam kết dùng nó.

### Filesystem là blocking, và nó có mặt khắp nơi
Ngoài phần thân file, một static handler còn thực hiện nhiều syscall *metadata* mỗi request — `stat` để lấy size và mtime, `canonicalize` cho path safety, chính `open`. Mỗi cái đều có thể block trên một dentry cache nguội hoặc một filesystem chậm/qua mạng, và không cái nào trông giống I/O khi bạn đọc code.

`tokio::fs` bọc chúng trong `spawn_blocking`, đúng đắn nhưng không miễn
phí: blocking pool có giới hạn (mặc định 512 thread), và một đợt static
request burst nhắm vào một filesystem chậm có thể làm bão hòa nó — lúc
đó *mọi* người dùng `spawn_blocking` trong process, kể cả những phần không
liên quan, đều queue phía sau chúng.

Hai cách giảm nhẹ đáng biết: cache metadata (và thậm chí cả file
descriptor đang mở) cho các file nóng, kiểu `open_file_cache` của nginx,
để các request lặp lại bỏ qua syscall hoàn toàn; và giới hạn concurrency
của static-file tách biệt khỏi concurrency request tổng thể
([`07-security/09-ddos.md`](../07-security/09-ddos.md)), để một lần đứng của filesystem nguội không thể
nuốt cả process.

Gotcha: một fd cache bị giới hạn bởi `ulimit -n` và phải evict
([`13-algorithms/lru.md`](../13-algorithms/lru.md)), và nó phải key trên thứ gì đó phát hiện được
việc file bị thay thế (device + inode, không phải path) — một lần deploy
thay file để lại bạn serve nội dung của fd cũ mãi mãi.

### Range request
Client (browser resume một download, video player tua) gửi `Range: bytes=1000-1999`. Một server tuân thủ trả về `206 Partial Content` kèm `Content-Range`, hoặc `416 Range Not Satisfiable` nếu range không hợp lệ — và phải xử lý request multi-range (`bytes=0-99,200-299`) hoặc từ chối tường minh qua `Accept-Ranges: none`.

Cú pháp chính xác (RFC 9110 §14), với một file 1000 byte. Vị trí đếm từ 0
và **cả hai đầu đều bao gồm**:

| `Range:` của request | Ý nghĩa | Response |
|---|---|---|
| `bytes=0-99` | 100 byte đầu tiên | `206`, `Content-Range: bytes 0-99/1000`, `Content-Length: 100` |
| `bytes=900-` | từ 900 tới hết | `206`, `Content-Range: bytes 900-999/1000` |
| `bytes=-100` | 100 byte *cuối cùng* (suffix) | `206`, `Content-Range: bytes 900-999/1000` |
| `bytes=0-5000` | điểm cuối vượt EOF: clamp lại về 999 | `206`, `Content-Range: bytes 0-999/1000` |
| `bytes=1000-` hoặc `bytes=-0` | bắt đầu tại hoặc sau điểm cuối, hoặc xin 0 byte | `416`, `Content-Range: bytes */1000` |
| `bytes=500-100`, `bytes=abc`, `items=0-5` | hoàn toàn không phải byte range hợp lệ | **bỏ qua header**: `200` với toàn bộ file |

Có ba luật đằng sau bảng đó. Một range *không thỏa mãn được* (cú pháp
đúng, nhưng không có gì trong file) nhận `416`, và `416` mang
`Content-Range: bytes */<size>` để client biết kích thước thật. Một header
`Range` *sai cú pháp*, hoặc có đơn vị lạ, không phải lỗi: server bỏ qua nó
và phục vụ toàn bộ representation với `200`. Và `Range` chỉ áp dụng cho
`GET`. Với method khác nó bị bỏ qua. Một `206` mang cùng `Content-Type` như
response đầy đủ, và server quảng bá việc hỗ trợ bằng `Accept-Ranges: bytes`.
File rỗng là trường hợp biên: `Content-Range` không có cách nào gọi tên một
range bên trong 0 byte, nên hãy trả lời range request tới nó bằng `200` và
body rỗng (bỏ qua `Range` luôn được phép).

Gotcha: multi-range là một vector khuếch đại. Một request liệt kê hàng
trăm range nhỏ chồng lấn buộc server phải dựng một response
`multipart/byteranges` lớn — output nhiều hơn hẳn input, cộng thêm CPU để
lắp ráp nó. Đây từng là một CVE thật ở cả Apache lẫn nginx. Giới hạn số
lượng range bạn chấp nhận (một vài cái), reject hoặc gộp các range chồng
lấn, và cân nhắc trả lời multi-range bằng toàn bộ body thay vào đó, điều
này luôn được phép.

Gotcha: `If-Range` tồn tại để một download được resume không âm thầm ghép
hai phiên bản khác nhau của một file lại với nhau. Nếu validator không
khớp, bạn phải trả về *toàn bộ* file (200), không phải range được yêu cầu.

### Conditional request: ETag & If-Modified-Since
Trả một response 200 đầy đủ cho một client đã có bản cache cập nhật là lãng phí bandwidth. Một `ETag` (hash hoặc dấu phiên bản của file) hay timestamp `Last-Modified` cho phép client gửi `If-None-Match`/`If-Modified-Since`; nếu không đổi, trả về `304 Not Modified` không body.

```rust
// sketch: conditional check before touching the file body at all
fn is_not_modified(etag: &str, if_none_match: Option<&str>) -> bool {
    if_none_match.is_some_and(|inm| inm == etag)
}
```

Gotcha: cách bạn sinh ETag quan trọng hơn vẻ ngoài của nó. Hash nội dung
file mỗi request là đúng nhưng tốn một lần đọc đầy đủ — đánh mất chính
mục đích của nó. Suy ra ETag từ `(inode, size, mtime)` là cách nginx và
hầu hết server làm: rẻ, và ổn định miễn là deploy của bạn thực sự thay đổi
mtime. Nhưng `mtime` có độ chi tiết một giây trên một số filesystem, nên
một file bị sửa hai lần trong cùng một giây giữ nguyên ETag và client
serve nội dung cũ vô thời hạn. Dùng độ chính xác nanosecond nếu có, hoặc
kèm một content hash tính một lần lúc khởi động/deploy.

Gotcha: biết sự khác biệt giữa ETag yếu (`W/"abc"`) và mạnh. Range request
đòi hỏi một validator *mạnh* — một `If-Range` khớp với một ETag yếu phải
bị coi là không khớp, vì "tương đương về mặt ngữ nghĩa" là không đủ tốt
khi ghép byte.

### Path traversal
`GET /../../etc/passwd` hay một biến thể đã encode (`%2e%2e%2f`) không bao giờ được resolve ra ngoài root được serve. Cách an toàn là canonicalize path đã resolve (`std::fs::canonicalize`) và xác minh nó vẫn là con cháu của thư mục root — so khớp chuỗi với `".."` là không đủ (symlink, mánh khóe encoding).

Gotcha: canonicalize-rồi-open là một cuộc đua TOCTOU. Giữa lúc kiểm tra và
lúc open, một symlink có thể bị đổi vào — trên một thư mục ai đó khác có
quyền ghi, đó là một exploit thật, không phải lý thuyết. Cách sửa vững
chắc là resolve tương đối với một thư mục root đã mở, dùng `openat2` với
`RESOLVE_BENEATH` (hoặc `cap-std`, thứ bọc pattern này trong Rust), để
kernel enforce việc containment một cách atomic thay vì bạn tự kiểm tra
một chuỗi.

Gotcha: cùng thảo luận về normalization như [`05-http-stack/04-router.md`](04-router.md)
áp dụng ở đây, và nếu router đã normalize path rồi, static handler không
được decode nó *lần nữa* — decode hai lần đưa traversal quay lại từ
`%252e%252e%252f`. Decode đúng một lần, ở một chỗ có ghi rõ.

### Đừng serve thứ bạn không định serve
Serve root của một thư mục nghĩa là serve mọi thứ bên dưới nó, kể cả những gì bạn quên là nó ở đó. Các rò rỉ thực tế lặp đi lặp lại:
- **`.git/`** — một working tree bị deploy làm lộ toàn bộ lịch sử source,
  bao gồm credential đã commit rồi sau đó xóa.
- **`.env`, `config.yml`, `*.bak`, file swap của editor** — bí mật, trần
  trụi.
- **Directory listing** — tắt theo mặc định trừ khi bạn cố tình muốn nó,
  vì một listing biến "attacker phải đoán tên file" thành "attacker đọc
  mục lục".

Từ chối dotfile theo mặc định, serve từ một thư mục chỉ chứa *build
output* (không phải một repo checkout), và tắt listing. Mỗi cái trong số
này chỉ tốn một dòng và mỗi cái đã từng gây ra sự cố thật.

### Header MIME type & caching
Content-Type nên đến từ một bảng extension→MIME thật, không đoán từ nội dung. Asset tĩnh có tên file chứa content hash (`app.a3f9c1.js`) có thể được serve với `Cache-Control: max-age=31536000, immutable` rất dài; file không hash cần TTL ngắn hơn nhiều hoặc phải dựa vào conditional request.

Gotcha: luôn gửi `X-Content-Type-Options: nosniff`. Thiếu nó, browser có
thể bỏ qua `Content-Type` của bạn và sniff nội dung — nên một file bạn
serve như `text/plain` có thể bị thực thi như `text/html` hay JavaScript.

Gotcha: serve file *người dùng tải lên* từ cùng origin với ứng dụng của
bạn là một lỗ hổng stored-XSS mà không header nào đóng hoàn toàn được —
một `.html` tải lên (hoặc một file bị sniff thành vậy) chạy với cookie của
origin bạn. Serve nội dung người dùng từ một origin riêng, hoặc buộc
`Content-Disposition: attachment` cho bất cứ gì được tải lên.

## Practice
Làm theo thứ tự này.

1. Trong [`labs/04-static-server`](../../labs/04-static-server), serve file từ một thư mục root bằng
   `tokio::fs::read` cộng một response body. **Xong khi** một file đã biết
   được trả về với đúng `Content-Type` từ một bảng extension.
2. Viết các test traversal trước khi hardening: `../../../etc/passwd`,
   các biến thể `%2e%2e%2f`, một biến thể double-encode, và một symlink
   trỏ ra ngoài root. **Xong khi** ít nhất một cái thoát được — rồi thêm
   containment và **xong lần nữa khi** không cái nào thoát được.
3. Thay canonicalize-rồi-open bằng `cap-std` (hoặc `openat2` với
   `RESOLVE_BENEATH`). **Xong khi** một test đổi symlink giữa lúc kiểm tra
   và lúc open không thể thoát khỏi root.
4. Từ chối dotfile và tắt listing. **Xong khi** `GET /.git/config` và
   `GET /` đều trả 404 thay vì lộ ra bất cứ gì.
5. Thêm `ETag` từ `(inode, size, mtime-với-nano)` và xử lý
   `If-None-Match`/`If-Modified-Since`. **Xong khi** một request lặp lại
   nhận `304` với body rỗng, và sửa một file hai lần trong cùng một giây
   vẫn tạo ra ETag đã thay đổi.
6. Thêm `Range` với `206`/`416`/`Content-Range`, `If-Range`, và một giới
   hạn số lượng multi-range. **Xong khi** một video player có thể tua,
   một range không hợp lệ nhận 416, và một request 500-range bị từ chối
   thay vì được lắp ráp.
7. Thêm `nosniff` và caching `immutable` sống lâu chỉ cho tên file có
   content hash. **Xong khi** asset đã hash nhận TTL một năm và asset chưa
   hash thì không.
8. Thêm một cache open-file/metadata key theo device+inode với LRU
   eviction. **Xong khi** request lặp lại cho một file nóng không tạo ra
   syscall `stat` nào (xác minh bằng `strace -c`), và thay file trên đĩa
   được nhận ra thay vì serve nội dung cũ.
9. Thay đọc+viết ngây thơ bằng `sendfile` hoặc `io_uring`. **Xong khi**
   throughput cải thiện trên một working set nóng, và bạn đã đo cả trường
   hợp *nguội* — nếu p99 tệ đi khi working set vượt RAM, bạn đã tìm ra
   gotcha ở trên và nên ghi lại điều đó trong ghi chú của mình.
