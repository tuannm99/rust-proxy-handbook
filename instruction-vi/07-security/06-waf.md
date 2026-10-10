# WAF Engine
Rule engine, signature, anomaly scoring.

## What to learn
### Thiết kế rule engine
Một rule của WAF match một phần nào đó của request (path, header, query
string, body) với một pattern và thực hiện một hành động (block, log,
score). Model rule như dữ liệu, không phải code, để chúng có thể được cập
nhật mà không cần redeploy proxy:

```rust
struct Rule {
    id: u32,
    target: RuleTarget,       // Path, Header(String), Query, Body
    pattern: regex::Regex,
    action: RuleAction,       // Block, Log, Score(i32)
}
```
Gotcha: chạy regex của mọi rule trên mọi body của mọi request là tốn kém —
sắp xếp rule theo thứ tự rẻ-trước (kiểm tra path/header trước regex body)
và short-circuit ngay khi gặp match `Block` đầu tiên.

Gotcha: rule được load từ config là input gần-attacker cho *compiler* regex
của bạn, không chỉ cho matcher. Thực thi một giới hạn kích thước sau khi
compile và từ chối một rule set xấu ngay tại thời điểm load, giữ nguyên
rule set trước đó đang chạy ([`09-architecture/03-config.md`](../09-architecture/03-config.md)) — một WAF làm
sập proxy vì một lỗi gõ trong file rule là một outage tự gây ra. Xem
[`13-algorithms/regex-engine.md`](../13-algorithms/regex-engine.md) cho khía cạnh ReDoS của việc này, đây là lý
do crate `regex` của Rust (linear-time, không backtracking) là engine đúng
ở đây còn `fancy-regex` thì không.

### Phát hiện dựa trên signature
Match trực tiếp các pattern tấn công đã biết (ví dụ `' OR '1'='1` cho
SQLi, `<script>` cho XSS, [`../../`](../../..) cho path traversal) — chính xác và
nhanh, nhưng chỉ bắt được các tấn công khớp với một signature đã biết; dễ
dàng bị bypass bởi kẻ tấn công thay đổi encoding/case/whitespace, nên
signature cần normalization (URL-decode, lowercase, gộp whitespace) được
áp dụng nhất quán trước khi match.

### Normalization là nơi WAF thực sự thất bại
Signature match theo byte, nên ai quyết định *byte nào* được match sẽ
quyết định WAF có hoạt động hay không. Việc này bị lệch một chút so với
cách parse của chính upstream — decode depth, Unicode, charset, tham số
trùng tên nào trong hai tham số được tính — là cách gần như mọi WAF bypass
thực tế hoạt động, và đó là một khoảng trống cấu trúc vĩnh viễn chứ không
phải một bug bạn sửa một lần là xong.

Đây là một topic đủ lớn để có file riêng của nó, và nó được chia sẻ với
routing ([`05-http-stack/04-router.md`](../05-http-stack/04-router.md)) và framing
([`07-security/05-request-smuggling.md`](05-request-smuggling.md)): xem
[`07-security/04-normalization.md`](04-normalization.md). Không có gì trong file này hoạt động
nếu cái đó sai.

### Body inspection: chi phí và giới hạn cứng
Các rule cần toàn bộ body cùng lúc (một regex với ngữ cảnh không giới
hạn, parse JSON để kiểm tra một field) đồng nghĩa với việc phải buffer nó,
và buffer bị giới hạn bởi lượng memory bạn sẵn sàng chi cho mỗi request
đồng thời. Signature dạng literal thì không cần vậy. Chúng có thể được
match trên một stream (phần tiếp theo), giữ memory phẳng. Với việc kiểm
tra có buffer, đây là ràng buộc giống hệt với retry buffering ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)), và cả hai nên
dùng chung một giới hạn thay vì mỗi bên giữ bản sao riêng của mình.

Quyết định không thể tránh khỏi là chuyện gì xảy ra với một body lớn hơn
giới hạn:
- **Fail open** (chuyển tiếp mà không kiểm tra): kẻ tấn công thêm padding
  để vượt quá giới hạn của bạn và bypass WAF hoàn toàn, mỗi lần.
- **Fail closed** (từ chối): một upload lớn hợp pháp nhận 413.

Không cái nào là "đúng" — nhưng fail-open là một bypass *im lặng* còn
fail-closed là một lỗi hiện rõ, nên mặc định đóng và tạo các route riêng,
đã xác thực cho upload lớn. Nếu bạn fail open, hãy cảnh báo về nó; một sự
gia tăng đột ngột các body quá khổ tự nó là một dấu hiệu tấn công.

### Match signature literal trên một body dạng stream
Body đến dưới dạng các chunk, và một signature có thể nằm vắt qua hai chunk
(`...UNION SE` | `LECT...`). Match từng chunk riêng lẻ sẽ bỏ sót nó, và
attacker có thể cố tình sắp đặt chỗ cắt đó. Có hai cách xử lý mà không cần
buffer toàn bộ body:

- **Mang state của automaton qua các chunk.** State hiện tại của một
  automaton Aho-Corasick *chính là* "prefix dài nhất của một pattern bất kỳ
  mà tôi vừa thấy" ([`13-algorithms/aho-corasick.md`](../13-algorithms/aho-corasick.md)). Đưa chunk 1 vào
  từng byte, giữ lại state cuối, và bắt đầu chunk 2 từ state đó thay vì từ
  start state, thì một signature bị cắt đôi vẫn được tìm thấy y như body
  liền mạch. Trong crate `aho-corasick`, `AhoCorasick::find` ở mức cao luôn
  bắt đầu lại từ đầu. Trait mức thấp `aho_corasick::automaton::Automaton`,
  được implement bởi `aho_corasick::dfa::DFA` và các kiểu `nfa`, cung cấp
  `start_state`, `next_state`, `is_match` và `match_pattern`, đủ mọi thứ cần
  thiết để mang một `StateID` từ chunk này sang chunk kế. Memory cho mỗi
  request: một state ID.
- **Cửa sổ chồng lấn.** Giữ lại `max_pattern_len - 1` byte cuối của mỗi
  chunk (`AhoCorasick::max_pattern_len()`) và quét chúng nối với chunk kế
  tiếp. Cách này đơn giản hơn, nhưng bạn phải loại bỏ các match bị tìm thấy
  hai lần trong vùng chồng lấn.

Quyết định mà điều này buộc bạn đưa ra là **khi nào chuyển tiếp một chunk
lên upstream**. Nếu bạn chuyển chunk 1 ngay khi quét xong và match hoàn tất
ở chunk 2, thì một phần của cuộc tấn công đã tới upstream. Bạn có thể giữ
lại `max_pattern_len - 1` byte cuối của mỗi chunk cho tới khi chunk kế đến
(latency có giới hạn, memory có giới hạn), đảm bảo không bao giờ có thứ gì
khớp bị chuyển tiếp. Hoặc bạn chấp nhận chuyển tiếp một phần, và khi có
match thì hủy request lên upstream và bỏ connection upstream đó (framing
của nó giờ không còn xác định, xem [`07-security/05-request-smuggling.md`](05-request-smuggling.md)),
rồi trả `403` cho client.

Gotcha: normalization gặp cùng vấn đề ranh giới chunk. Một percent-escape
bị cắt thành `%2` | `7` chỉ decode ra `'` nếu decoder cũng mang state qua
các chunk. Hãy đặt một decoder dạng stream trước matcher dạng stream.
Đừng decode từng chunk riêng lẻ.

### Anomaly scoring
Thay vì block cứng cho mỗi rule, mỗi rule match cộng thêm điểm; request chỉ
bị block khi *tổng* điểm vượt một ngưỡng. Đây là model cốt lõi của
ModSecurity (OWASP Core Rule Set) — nó giảm false positive từ bất kỳ rule
đơn lẻ nào quá rộng, đổi lại là khó suy luận hơn ("vì sao cái này bị
block?" cần cộng xem rule nào đã kích hoạt).

CRS thêm một nút điều chỉnh thứ hai lên trên: **paranoia level**, nơi mức
cao hơn bật thêm các rule ngày càng aggressive hơn (và dễ false-positive
hơn). Mô hình vận hành là tăng dần paranoia level và hạ dần ngưỡng, đo false
positive ở mỗi bước, thay vì deploy cấu hình nghiêm ngặt nhất rồi mới phát
hiện nó phá vỡ traffic hợp lệ nào trong production.

Gotcha: luôn log các rule ID và điểm số góp phần khi block. Một quyết định
block mà bạn không thể giải thích sau đó là một quyết định bạn không thể
tune, và "WAF block một khách hàng và không ai nói được vì sao" là cách WAF
bị tắt vĩnh viễn.

### False positive mới là rủi ro thật sự
Threat model mà mọi người mang tới WAF là "kẻ tấn công lọt qua". Outage mà
nó thực sự gây ra là "WAF block traffic hợp lệ" — một rule quá rộng khớp với
dữ liệu bình thường của khách hàng (một cái tên có dấu nháy đơn, một đoạn
code trong ticket support, một blob base64 tình cờ chứa [`../`](../..)) âm thầm phá
vỡ một tính năng, và vì việc block xảy ra ở edge, application log không
cho thấy gì cả.

Biện pháp giảm thiểu chuẩn là chạy mọi rule set mới ở **chế độ chỉ phát
hiện** trước: score và log, không bao giờ block. So sánh những gì *lẽ ra*
đã bị block với traffic thật trong vài ngày, tune, rồi mới enforce. Đây là
một quy trình canary ([`09-architecture/06-canary-deploy.md`](../09-architecture/06-canary-deploy.md)) áp dụng cho
security rule, và bỏ qua nó là cách một bản cập nhật rule thường ngày biến
thành một sự cố.

Gotcha: chế độ chỉ phát hiện phải chạy qua đúng code path như chế độ block,
chỉ khác nhau ở hành động cuối cùng. Một "dry run" path riêng bỏ qua
normalization hoặc short-circuit khác đi sẽ không cho bạn biết gì về việc
enforcement thực sự sẽ làm gì.

### Hiệu năng: prefilter trước khi match
Chạy một rule set hàng trăm regex trên mọi request là linear theo số
lượng rule, và nó nằm trên hot path của mọi request bạn phục vụ. Hai cách
sửa mang tính cấu trúc, cả hai đều nằm trong [`13-algorithms/`](../13-algorithms):
- Trích ra các chuỗi con literal bắt buộc và quét chúng trước bằng một
  multi-pattern matcher ([`13-algorithms/aho-corasick.md`](../13-algorithms/aho-corasick.md)). Phần lớn traffic
  vô hại không chứa cái nào trong số đó và không bao giờ chạm tới regex.
- Compile các pattern còn sống sót vào một `RegexSet` duy nhất
  ([`13-algorithms/regex-engine.md`](../13-algorithms/regex-engine.md)) để P pattern chỉ tốn một lượt quét,
  không phải P lượt.

Gotcha: đo tầng WAF tách riêng khỏi tổng latency của request
([`08-observability/02-metrics.md`](../08-observability/02-metrics.md)). Nếu gộp vào một con số tổng, một WAF
thêm 8ms ở p50 và 200ms ở p99 trên body lớn trông giống như chậm chung
chung thay vì một tầng có thể tune riêng.

### Tự xây so với dùng một cái thật
Viết một rule engine đồ chơi ở đây có giá trị để hiểu cách request
inspection khớp vào pipeline của một proxy (xem
[`09-architecture/01-components.md`](../09-architecture/01-components.md)) và chi phí hiệu năng của nó. Với bất
cứ thứ gì hướng ra internet, ưu tiên nhúng một engine được maintain —
Coraza (Go, tương thích OWASP CRS, có câu chuyện FFI Rust) hoặc shell ra
ModSecurity — thay vì tin tưởng một signature set tự viết tay để bao phủ
traffic tấn công thực tế.

## Practice
Làm lần lượt theo thứ tự sau.

1. Trong [`labs/12-waf`](../../labs/12-waf), implement `Rule`/`RuleTarget` với 5-10 signature
   viết cứng (SQLi, XSS, path traversal). **Xong khi** một payload rõ ràng
   trong query string bị block và traffic bình thường đi qua.
2. Làm qua các bài tập của [`07-security/04-normalization.md`](04-normalization.md) với rule
   engine này. **Xong khi** các cách bypass mixed-case, double-encoded, và
   tham số trùng lặp đều bị bắt, và router cùng WAF đọc chung một dạng
   canonical.
3. Thêm anomaly scoring với trọng số theo từng rule và một ngưỡng; log các
   rule ID góp phần khi block. **Xong khi** một match trọng số trung bình
   đơn lẻ được cho qua và hai cái cộng lại thì block, với cả hai rule ID
   trong dòng log.
4. Thêm chế độ chỉ phát hiện như một flag trên cùng code path. **Xong khi**
   một request đáng lẽ bị block được log kèm điểm số và chuyển tiếp nguyên
   vẹn, và đổi một giá trị config sẽ enforce nó.
5. Thêm body inspection với một giới hạn buffer dùng chung và hành vi
   fail-closed. **Xong khi** một payload trong giới hạn bị bắt, một body
   quá khổ nhận 413 thay vì đi qua mà không kiểm tra, và trường hợp quá
   khổ tăng metric riêng của nó.
6. Thêm một Aho-Corasick literal prefilter trước một `RegexSet`. **Xong
   khi** bạn có thể báo cáo tỷ lệ request vô hại không bao giờ chạm regex,
   và một benchmark ở 10 / 100 / 1000 rule cho thấy latency gần như phẳng
   thay vì linear theo số rule.
7. Benchmark tầng WAF bật so với tắt dưới tải, như một metric riêng. **Xong
   khi** bạn có số liệu p50 và p99 cho riêng tầng này, cả với body nhỏ lẫn
   lớn, và có thể chỉ ra rule nào chiếm ưu thế.
