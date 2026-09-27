# Aho-Corasick

Multi-pattern string matching trong một lượt. Thuật toán khiến một WAF
(`07-security/06-waf.md`) trở nên khả thi về chi phí: match 5000 signature
với một request body trong thời gian một vòng lặp ngây thơ match được một.

## What to learn

### Bài toán nó giải quyết
Một WAF với P signature, nếu check ngây thơ, chạy P lần tìm kiếm riêng
biệt trên mỗi request — O(P × n) với body dài n, và nó đọc lại cùng những
byte đó P lần, phá vỡ cache locality. Aho-Corasick tìm *tất cả* các lần
xuất hiện của *tất cả* pattern trong O(n + matches), không phụ thuộc vào
P, trong một lượt chạm mỗi byte input đúng một lần.

Cái giá phải trả là một bước tiền xử lý trên tập pattern, làm một lần khi
khởi động hoặc reload config — không phải trên từng request.

### Automaton: trie cộng với failure link
Xây một trie từ tất cả pattern. Sau đó thêm một **failure link** từ mỗi
node tới node đại diện cho hậu tố (suffix) thực sự dài nhất của match hiện
tại mà cũng là tiền tố (prefix) của một pattern nào đó. Việc matching sau
đó không bao giờ backtrack trên input: khi mismatch, đi theo failure link
và tiếp tục với *cùng* byte input đó.

Ví dụ kinh điển: các pattern `he`, `she`, `his`, `hers`. Match chuỗi
`"ushers"`, automaton đi tới `sh`→`she` và báo `she`; failure link từ node
đó trỏ vào `he`, nên nó báo `he` ngay tại vị trí đó mà không cần lùi lại —
rồi tiếp tục vào `hers`.

**Output link** xử lý trường hợp một pattern là hậu tố của pattern khác
(`he` nằm trong `she`): một node phải báo không chỉ pattern của chính nó
mà mọi pattern có thể tới được qua chuỗi failure của nó, nếu không các
match lồng nhau sẽ bị bỏ sót âm thầm.

Gotcha: đây là bug mà người ta hay ship ra production. Nếu tập test của
bạn không có pattern nào là substring của pattern khác, một implementation
output-link bị hỏng vẫn pass mọi test — rồi bỏ sót match thật trong
production ngay ngày ai đó thêm một signature bị chồng lấn.

### Xây dựng
Failure link được tính bằng BFS trên trie, theo từng tầng: failure target
của một node được suy ra từ failure target của node cha, thứ mà BFS đảm
bảo đã được giải quyết xong. Con của root fail về root.

Để matching nhanh hơn, trie thường được chuyển thành một **DFA**: tính
trước đầy đủ transition cho mọi cặp (state, byte) để việc matching chỉ là
một lần tra bảng cho mỗi byte, không cần đuổi theo failure link. Đánh đổi
là bộ nhớ — `states × 256` entry — đó là lý do vì sao `aho-corasick` cung
cấp cả biến thể NFA và DFA và chọn dựa trên kích thước tập pattern.

```rust
use aho_corasick::AhoCorasick;

// build một lần khi khởi động / khi reload config, sau đó share read-only
let ac = AhoCorasick::new(&["' OR '1'='1", "<script", "../"])?;
for m in ac.find_iter(request_body) {
    // m.pattern() -> signature nào; m.start() -> ở đâu
}
```

Gotcha: xây automaton tốn kém hơn nhiều so với matching. Xây nó một lần và
share qua một `Arc`; rebuild theo từng request (hay tệ hơn, theo từng
rule) vứt bỏ toàn bộ lợi thế. Khi reload config, xây automaton mới hoàn
chỉnh rồi mới swap `Arc` — không bao giờ mutate một automaton đang sống.

### Normalization phải xảy ra trước khi matching
Aho-Corasick match byte theo nghĩa đen. `<ScRiPt>` không match `<script`,
và `%2e%2e%2f` không match `../`. Pipeline normalization từ
`07-security/06-waf.md` — URL-decode, lowercase, gộp khoảng trắng — chính
là thứ khiến literal matching khả thi, và nó phải được áp dụng giống hệt
nhau lên pattern lúc build và lên input lúc match.

Gotcha: normalize một lần vào một buffer, rồi mới match. Normalize lười
theo từng pattern lại đưa trở lại đúng cái chi phí O(P × n) mà bạn dùng
thuật toán này để tránh. Cũng để ý double-decoding: decode `%252e` hai lần
ra `.`, và việc attacker có khai thác được điều đó hay không phụ thuộc vào
việc *upstream* làm gì, cùng một lớp bug parser-mismatch như
`07-security/05-request-smuggling.md`.

### Ranh giới của literal-match nằm ở đâu
Aho-Corasick xử lý chuỗi literal, không phải regex. Rule WAF thật cần cả
hai: dùng Aho-Corasick như một **prefilter** nhanh — nếu không substring
literal nào của một rule xuất hiện, regex của rule đó không thể match,
nên bỏ qua nó. Đây chính xác là cách crate `regex` tăng tốc các alternation
bên trong, và nó biến "chạy 5000 regex" thành "chạy 3 regex mà literal của
chúng đã xuất hiện." Xem `13-algorithms/regex-engine.md` cho phía engine.

## Practice
1. Trong `labs/12-waf`, thay việc quét substring theo từng rule bằng một
   `AhoCorasick` duy nhất xây từ toàn bộ literal của signature; benchmark
   cả hai với body 100 KB ở 10, 100, và 1000 pattern và xác nhận thời gian
   của AC phẳng theo số lượng pattern.
2. Viết test cho pattern chồng lấn: gồm `he`, `she`, `his`, `hers` và
   assert `"ushers"` báo cả `she` lẫn `he`. Đây là bài kiểm tra output-link.
3. Tự tay xây trie và failure link (không dùng crate) cho bốn pattern đó
   và vẽ automaton; xác nhận thứ tự BFS của bạn giải quyết failure target
   của mỗi node trước khi con của nó cần đến.
4. Nối normalization: xác nhận cả `<ScRiPt>` và `%3Cscript` đều match
   signature `<script`, và pattern lẫn input đều được normalize qua cùng
   một code path.
5. Dùng AC làm prefilter đứng trước các rule regex của bạn; đo phần trăm
   rule bị bỏ qua trên traffic lành tính.
6. Reload tập signature dưới tải đồng thời bằng cách xây automaton mới rồi
   swap một `Arc` — xác nhận không request nào thấy một automaton xây dở.
