# Request Smuggling

## What to learn
### Vì sao nó tồn tại: hai parser, một sự bất đồng
Một reverse proxy parse một HTTP/1.1 request để quyết định nó kết thúc ở
đâu, rồi chuyển tiếp byte tới một upstream, thứ sẽ parse lại *chính các
byte đó* bằng parser của riêng nó. Request smuggling xảy ra khi proxy và
upstream bất đồng về việc một request kết thúc ở đâu và request tiếp theo
bắt đầu ở đâu — kẻ tấn công tạo ra một request mà một parser đọc là "một
request" còn parser kia đọc là "một request cộng với phần đầu của một
request thứ hai, bị smuggle" và bị xử lý nhầm vào kết nối của một client
xui xẻo tiếp theo (trên một kết nối keep-alive/pooled được tái sử dụng tới
upstream). Đây chính xác là loại mơ hồ mà [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) và
[`labs/01-http-parser`](../../labs/01-http-parser) buộc bạn phải đối mặt bằng tay.

Điều kiện tiên quyết đáng chú ý: cuộc tấn công này tồn tại *vì* proxy pool
và tái sử dụng kết nối upstream ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)). Các byte bị
smuggle nằm ở đầu buffer của một kết nối, chờ bất kỳ ai dùng nó tiếp theo.
Một proxy mở kết nối mới cho mỗi request rồi đóng nó ngay sau đó sẽ miễn
nhiễm — và chậm hơn nhiều, đó là lý do không ai làm vậy, và cũng là lý do
lớp tấn công này vẫn tồn tại.

### Kẻ tấn công thực sự được gì
Đáng nói thẳng ra, vì các biện pháp giảm thiểu trông như chủ nghĩa hình
thức cho tới khi bạn thấy được cái giá phải trả:
- **Vượt qua hoàn toàn bảo mật ở front-end.** Proxy thực thi auth
  ([`01-auth.md`](01-auth.md)), IP filtering, và rule WAF trên các request nó *thấy
  được*. Một request bị smuggle không bao giờ được proxy nhìn thấy như một
  request — nó là byte của body — nên nó tới upstream sau khi đã bỏ qua
  mọi kiểm tra. Kẻ tấn công tới được `/admin` qua một proxy được cấu hình
  rõ ràng để chặn `/admin`.
- **Đánh cắp request của người dùng khác.** Phần đầu bị smuggle có thể
  được tạo sao cho request thật *tiếp theo* trên kết nối đó bị gắn vào nó
  như body content — và bị echo lại trong một response mà kẻ tấn công có
  thể đọc. Bao gồm cả session cookie và auth header.
- **Đầu độc cache.** Kết hợp với một cache ([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)),
  một response bị desync lưu vào sai key và phục vụ cho mọi người.

Một kẻ tấn công, không cần credential, và thiệt hại tỷ lệ với lượng traffic
chia sẻ kết nối bị đầu độc.

### CL.TE, TE.CL, TE.TE
- **CL.TE**: request có cả `Content-Length` lẫn `Transfer-Encoding:
  chunked`. Front-end (proxy) dùng `Content-Length` để định khung body;
  back-end (upstream) dùng `Transfer-Encoding` thay vào đó. Các byte mà
  proxy tưởng là "sau request" thực ra nằm bên trong chunked body theo góc
  nhìn của upstream — hoặc ngược lại — và bị diễn giải lại thành phần đầu
  của một request mới.
- **TE.CL**: ngược lại — front-end tôn trọng `Transfer-Encoding`, back-end
  tôn trọng `Content-Length`.
- **TE.TE**: cả hai đều tôn trọng `Transfer-Encoding`, nhưng một trong hai
  có thể bị lừa để bỏ qua nó thông qua một giá trị header bị làm sai
  lệch/obfuscate (ví dụ `Transfer-Encoding: chunked ` với một khoảng trắng
  ở cuối, hoặc một header trùng lặp) mà chỉ một trong hai parser coi là
  không hợp lệ và fallback về `Content-Length`.

Các byte làm cho nó trở nên cụ thể. Một payload CL.TE:

```http
POST / HTTP/1.1
Host: example.com
Content-Length: 6
Transfer-Encoding: chunked

0

G
```

Proxy đọc `Content-Length: 6` và chuyển tiếp đúng sáu byte body
(`0\r\n\r\nG`), coi request đã hoàn tất. Upstream đọc
`Transfer-Encoding: chunked`, thấy chunk terminator độ dài 0, và coi body
đã kết thúc *trước* `G` — thứ bị bỏ lại trong buffer của nó. Request thật
tiếp theo trên kết nối pooled đó bị dán `G` vào đầu, trở thành
`GPOST / HTTP/1.1...` — và nạn nhân đó nhận được lỗi, trong khi một phần
đầu được chuẩn bị kỹ hơn sẽ cho kẻ tấn công thứ gì đó hữu ích hơn.

### Downgrade smuggling (H2.CL / H2.TE)
Biến thể hiện đại, và là loại liên quan nhất tới một proxy terminate
HTTP/2 rồi nói HTTP/1.1 lên upstream ([`01-network/12-http2.md`](../01-network/12-http2.md)). Frame
HTTP/2 mang độ dài tường minh của riêng chúng, nên không có sự mơ hồ nào
*bên trong* HTTP/2 — nhưng `content-length` vẫn tồn tại như một header
bình thường, và kẻ tấn công có thể gửi một request HTTP/2 với
`content-length` khai báo bất đồng với độ dài frame thực tế.

Nếu proxy dịch request đó sang HTTP/1.1 bằng cách copy header nguyên văn,
nó phát ra một request HTTP/1.1 với `Content-Length` không khớp với body
nó thực sự ghi ra — tạo ra chính xác sự desync mà các parser HTTP/1.1 đã
cẩn thận tránh. Điều tương tự xảy ra khi kẻ tấn công lén đưa
`transfer-encoding: chunked` qua như một header HTTP/2.

Quy tắc khi downgrade: **tự tạo lại framing header từ chính dữ liệu bạn
sắp ghi ra, không bao giờ copy chúng từ request inbound.** Và từ chối các
request HTTP/2 inbound có `content-length` bất đồng với tổng độ dài các
DATA frame, thay vì tin bất kỳ bên nào.

### CL.0 và desync trạng thái kết nối
Một nhóm ít ồn ào hơn: upstream bỏ qua hoàn toàn body cho một số request
(nhiều server bỏ qua body trên `GET`, hoặc trên một path map tới một static
file), coi `Content-Length` như bằng `0` một cách hiệu quả. Body mà proxy
đã trung thành chuyển tiếp thì nằm lại trong buffer của upstream, và nó bị
parse như request tiếp theo — không cần bất kỳ thủ thuật header nào cả,
chỉ cần một endpoint không đọc thứ nó được gửi.

Gotcha: kiểu này không thể sửa bằng cách validate header, vì các header
*hợp lệ*. Nó phụ thuộc hoàn toàn vào hành vi của upstream, đó là lý do biện
pháp "drain hoặc đóng kết nối khi có bất kỳ anomaly nào" bên dưới vẫn quan
trọng ngay cả khi việc validate framing của bạn hoàn hảo.

### Mitigations
1. **Từ chối sự mơ hồ ngay lập tức**: nếu một request có cả
   `Content-Length` lẫn `Transfer-Encoding`, từ chối nó với 400 — đừng cố
   đoán cái nào "thắng". RFC 9112 §6.1 cho server chọn hoặc từ chối, hoặc
   xử lý chỉ theo `Transfer-Encoding` (và dù cách nào cũng phải đóng
   connection sau đó). Từ chối là lựa chọn chặt hơn và là cái nên chọn
   ([`01-network/11-http1-wire-format.md`](../01-network/11-http1-wire-format.md) có thứ tự luật đầy đủ).
2. **Normalize trước khi chuyển tiếp**: loại bỏ/từ chối các framing header
   trùng lặp hoặc sai định dạng thay vì chuyển tiếp nguyên vẹn. Từ chối
   thay vì "dọn dẹp": một header bạn normalize thành hợp lệ là một header
   mà upstream có thể vẫn chấp nhận theo cách khác ở dạng gốc.
3. **Tự tạo lại framing ở mỗi hop.** Bất cứ thứ gì bạn chuyển tiếp lên
   upstream nên có `Content-Length`/`Transfer-Encoding` do *chính bạn*
   viết ra, tính từ body bạn thực sự đang gửi — không phải kế thừa.
4. **Ưu tiên HTTP/2 tới upstream** khi có thể — framing dạng
   length-prefixed của HTTP/2 không có sự mơ hồ
   `Content-Length`-so-với-`Transfer-Encoding` ngay từ đầu, đó là lý do
   "downgrade smuggling" (front-end HTTP/2, back-end HTTP/1.1) là một lớp
   tấn công riêng cần để ý khi dịch giao thức.
5. Ưu tiên kết nối upstream riêng cho mỗi request (hoặc drain kết nối
   quyết liệt khi có bất kỳ anomaly parse nào) hơn là các kết nối tái sử
   dụng sống lâu khi bạn không thể tin tưởng hoàn toàn tính nhất quán của
   parser upstream.
6. **Nghiêm khắc về whitespace và line ending.** Chỉ chấp nhận `\r\n` như
   line terminator, từ chối một `\n` trần, từ chối khoảng trắng giữa tên
   header và dấu hai chấm, từ chối ký tự không phải số trong
   `Content-Length`. Mỗi điều trong số này từng là một cách bypass thật,
   vì "hãy khoan dung với những gì bạn chấp nhận" và "hai parser phải đồng
   thuận" là hai mục tiêu trực tiếp mâu thuẫn nhau — và với một proxy, sự
   đồng thuận phải thắng.

### Phát hiện nó
Bạn không thể dựa vào việc nhận ra thiệt hại, vì nạn nhân là một client
khác với kẻ tấn công. Các tín hiệu đáng thiết lập
([`08-observability/01-logging.md`](../08-observability/01-logging.md)):
- **Lỗi parse ở upstream trên các kết nối pooled.** Một 400 từ upstream
  cho một request mà proxy của bạn coi là hợp lệ là bằng chứng rõ ràng nhất
  cho một kết nối bị desync.
- **Request với tiền tố phi lý.** Log upstream cho thấy các method như
  `GPOST` hoặc path bị dán vào body trước đó.
- **Timing.** Kỹ thuật phát hiện chuẩn (của PortSwigger) là gửi một payload
  khiến *upstream* chờ một body không bao giờ tới: nếu response treo tới
  read timeout thay vì trả về ngay, các parser đã bất đồng. Đáng để xây
  dựng vào chính bộ test của bạn thay vì chỉ đọc về nó.

Khi bạn phát hiện một anomaly, **đóng kết nối upstream** thay vì trả nó về
pool — bất cứ thứ gì còn lại trong buffer của nó chính là payload.

## Practice
Làm lần lượt theo thứ tự sau.

1. Trong [`labs/01-http-parser`](../../labs/01-http-parser), thêm một test với cả `Content-Length` lẫn
   `Transfer-Encoding: chunked`. **Xong khi** parser trả về lỗi thay vì
   chọn một trong hai.
2. Thêm các test cho các biến thể obfuscation: khoảng trắng ở cuối sau
   `chunked`, header `Transfer-Encoding` trùng lặp, `Content-Length: 6 `
   với khoảng trắng ở cuối, một `\n` trần làm line ending, và khoảng trắng
   trước dấu hai chấm. **Xong khi** mỗi cái đều bị từ chối, và bạn có thể
   nói rõ cho từng trường hợp một upstream có thể đã làm gì khác đi với nó.
3. Xây payload CL.TE ở trên và gửi nó qua [`proxy`](../../proxy) tới một upstream đồ chơi
   dùng một parser *khác* (một dòng Python hay Node là lý tưởng — parser
   khác, bug khác). **Xong khi** bạn quan sát thấy một desync thật sự:
   upstream đồ chơi thấy một request thứ hai bị làm hỏng. Bạn cần tận mắt
   thấy nó hoạt động trước khi tin rằng bản sửa của bạn ngăn được nó.
4. Thêm một tầng framing-validation trong [`proxy`](../../proxy) trước khi gọi upstream,
   và tự tạo lại framing header khi chuyển tiếp. **Xong khi** payload ở
   bước 3 bị từ chối với 400, một security event được log, và request mà
   proxy của bạn phát ra lên upstream mang framing header do chính nó
   tính toán.
5. Thêm bộ phát hiện dựa trên timing như một test. **Xong khi** một payload
   đáng lẽ khiến upstream chờ một body ma bị bắt bởi validation của bạn
   thay vì treo tới read timeout.
6. Khiến bất kỳ anomaly parse nào đầu độc kết nối. **Xong khi** một kết nối
   đã tạo ra lỗi framing bị đóng thay vì trả về pool — xác minh bằng
   `ss -tan` rằng nó không xuất hiện lại như một kết nối pooled đang rảnh.
7. (Mở rộng) Nếu [`proxy`](../../proxy) terminate HTTP/2 ([`labs/08-http2`](../../labs/08-http2)), xây một
   payload downgrade nơi `content-length` HTTP/2 bất đồng với các DATA
   frame. **Xong khi** nó bị từ chối ngay ở tầng h2 thay vì bị dịch thành
   một request HTTP/1.1 sai định dạng.
