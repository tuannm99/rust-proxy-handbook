# Regex Engine

Một regex engine thực sự được xây như thế nào, và câu trả lời quyết định
WAF của bạn (`07-security/06-waf.md`) là một lớp phòng thủ hay một vector
denial-of-service.

## What to learn

### Hai họ engine, và cái quan trọng ở đây
**Backtracking engine** (PCRE, Python `re`, JavaScript, Java) thử một
alternative, nếu fail thì rewind và thử cái tiếp theo. Đổi lại bạn có
backreference và lookaround, nhưng phải trả giá bằng thời gian *exponential*
ở worst case.

**Automata engine** (RE2, `regex` của Rust, `regexp` của Go) compile
pattern thành một NFA rồi mô phỏng nó, theo dõi *tập* các state đang active
thay vì đi từng đường một. Đảm bảo O(n × m) — tuyến tính theo độ dài input —
đổi lại là phải bỏ backreference và lookaround tùy ý, chính là những tính
năng khiến matching thời gian tuyến tính trở nên bất khả thi.

Với bất cứ thứ gì match input do attacker kiểm soát, đây không phải là sở
thích. Đây là ranh giới giữa một chi phí bị chặn trên và một vụ crash từ xa.

### ReDoS: kiểu thất bại
Một backtracking engine chạy `(a+)+b` trên `"aaaaaaaaaaaaaaaaaaaaaaaaX"` sẽ
duyệt qua mọi cách chia các ký tự `a` giữa `+` bên trong và `+` bên ngoài.
Đó là exponential theo độ dài input: ~25 ký tự đã mất vài giây, ~35 ký tự
mất vài phút. Một request, một CPU core bị treo vô thời hạn.

Đây là một nhóm outage có thật và lặp lại — outage toàn cầu năm 2019 của
Cloudflare là do một rule WAF bị catastrophic backtracking, và outage năm
2016 của Stack Overflow là do một regex trim đơn giản. Kiểu bug luôn giống
nhau: một rule trông vô hại, được deploy để kiểm tra input người dùng, gặp
phải một chuỗi mà tác giả của nó chưa từng thử.

Gotcha: crate `regex` bảo vệ bạn *bằng cấu trúc*, và rất dễ đánh mất sự bảo
vệ đó nếu bạn dùng một crate backtracking để có backreference
(`fancy-regex`), hoặc shell ra một engine dựa trên PCRE. Nếu một rule WAF
cần backreference, hãy coi đó là tín hiệu rule sai vị trí trong stack, chứ
không phải lý do để đổi engine.

### Thompson construction và mô phỏng NFA
Compile đi theo hướng regex → NFA qua Thompson construction: mỗi toán tử
ánh xạ thành một fragment NFA nhỏ (concatenation nối chúng lại, alternation
rẽ nhánh bằng epsilon transition, `*` thêm một vòng lặp), ghép lại đệ quy.

Mô phỏng sau đó tiến một *tập* state active từng byte input một. Vì tập
state không bao giờ vượt quá tổng số state `m`, và mỗi byte chỉ xử lý một
lần, bound là O(n × m) mà không cần backtracking — và input được đọc
nghiêm ngặt về phía trước, đó là điều cho phép streaming.

### Xây DFA và đánh đổi bộ nhớ
Một DFA chuyển mỗi *tập* state NFA riêng biệt thành một state duy nhất,
biến matching thành một lần tra bảng mỗi byte — lựa chọn nhanh nhất. Vấn
đề là số lượng tập con là exponential ở worst case, nên một DFA đầy đủ xây
ngây thơ có thể làm nổ bộ nhớ với một pattern độc hại.

RE2 và crate `regex` giải quyết bằng **lazy DFA**: xây state theo nhu cầu
khi input được tiêu thụ, cache lại, và evict cache khi chạm giới hạn kích
thước, fallback về mô phỏng NFA. Bạn có tốc độ DFA với pattern thực tế và
bộ nhớ bị chặn trên với pattern bệnh hoạn. Chính vì fallback đó mà `regex`
expose `size_limit` và `dfa_size_limit` — hãy set chúng tường minh khi bộ
pattern được load từ config thay vì do chính bạn viết.

### Literal prefilter
Cú tăng tốc lớn nhất trên thực tế không nằm ở engine: trích các substring
literal bắt buộc từ pattern, quét tìm chúng trước bằng một multi-pattern
matcher nhanh (`13-algorithms/aho-corasick.md` hoặc memchr), và chỉ chạy
engine đầy đủ ở chỗ có literal khớp. Một pattern như `\d+-admin-\w+` không
thể match nếu thiếu `-admin-`, nên đa số input bị loại ở tốc độ memchr.
Crate `regex` tự làm điều này bên trong, và đây cũng chính là thiết kế bạn
nên áp dụng cho toàn bộ rule set của một WAF.

### RegexSet: nhiều pattern, một lượt quét
Match P pattern bằng cách loop qua P engine là O(P × n). `RegexSet` compile
alternation của tất cả pattern thành một automaton và báo cáo pattern nào
khớp trong một lượt duy nhất:

```rust
use regex::RegexSet;

// build một lần khi load config, share qua Arc
let set = RegexSet::new(&[r"(?i)<script", r"(?i)union\s+select", r"\.\./"])?;
let hits: Vec<usize> = set.matches(input).into_iter().collect();
```

Gotcha: `RegexSet` cho bạn biết pattern nào khớp, chứ không cho biết *ở
đâu*. Với anomaly scoring (`07-security/06-waf.md`) như vậy là đủ và nhanh
hơn nhiều. Chỉ chạy lại từng pattern riêng để lấy vị trí match khi bạn thực
sự cần log đoạn nội dung vi phạm.

### Compile pattern không tin cậy
Nếu rule WAF đến từ config mà người khác ngoài bạn có thể sửa, pattern đó
là input không tin cậy đối với compiler. Áp giới hạn kích thước sau khi
compile, giới hạn độ dài pattern, và reject lỗi compile ngay tại thời điểm
*load* trong khi để rule set cũ tiếp tục chạy (`09-architecture/03-config.md`)
— đừng bao giờ để một rule sai có hiệu lực hoặc làm sập proxy khi reload.

## Practice
1. Chứng minh ReDoS: match `(a+)+b` với `"a"*n + "X"` bằng một backtracking
   engine (`fancy-regex`) cho n = 20, 25, 30 và vẽ đồ thị thời gian. Lặp
   lại với crate `regex` và xác nhận nó vẫn tuyến tính.
2. Xây một NFA bằng Thompson construction cho `a(b|c)*d` trên giấy, rồi
   trace tập state active từng byte một trên `"abccd"`.
3. Trong `labs/12-waf`, thay các vòng lặp `Regex` theo từng rule bằng một
   `RegexSet` duy nhất; benchmark ở 10, 100, và 1000 rule trên một body
   100 KB.
4. Thêm một literal prefilter bằng Aho-Corasick trước `RegexSet` và đo tỷ
   lệ request lành tính không bao giờ chạm tới engine.
5. Set `size_limit`/`dfa_size_limit` tường minh, rồi đưa vào một pattern
   được thiết kế để vượt giới hạn đó; xác nhận bạn nhận được lỗi sạch tại
   thời điểm load thay vì bộ nhớ tăng không kiểm soát.
6. Load một rule cố tình sai qua config reload và xác nhận rule set trước
   đó vẫn còn active.
