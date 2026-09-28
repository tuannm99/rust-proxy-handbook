# Trie (Prefix Tree)

[`05-http-stack/04-router.md`](../05-http-stack/04-router.md) nói về việc match method+path ở tầng proxy.
File này nói về cấu trúc mà matching của một router thật được xây trên đó:
lookup theo prefix dùng chung, đúng là thứ mà một path (`/users/:id/posts`)
được tạo thành từ đó.

## What to learn

### Cấu trúc
Một trie là một cây mà mỗi edge được gán nhãn bằng một đơn vị của key (một
ký tự, hoặc với một router, một path segment), và một lượt lookup đi qua
cây từng đơn vị một thay vì so sánh cả key:

```rust
struct TrieNode {
    children: std::collections::HashMap<String, TrieNode>, // segment -> child
    handler: Option<RouteHandler>,
}
```

Đăng ký `/users/:id` và `/users/:id/posts` dùng chung node `users` và
`:id` — prefix dùng chung được lưu một lần, và một path request được match
bằng cách đi xuống từng segment cho tới khi hết handler hoặc hết path.

### Vì sao trie thắng việc quét một danh sách route
Một lượt quét tuyến tính N route đã đăng ký là O(N) cho mỗi request bất kể
hình dạng path. Một trie theo segment là O(depth) — tỷ lệ với độ dài của
chính path đó, không phải số route đã đăng ký — điều này quan trọng khi
một service có hàng trăm route: route thứ 500 tốn thời gian lookup như
route đầu tiên.

### Segment static vs tham số vs wildcard
Trie của một router thật cần ba loại con ở mỗi level: một segment
exact-match (`/users`), một capture tham số (`:id`, khớp bất kỳ một
segment nào và bind một giá trị), và một wildcard (`*rest`, khớp mọi thứ
còn lại). Precedence phải tường minh và nhất quán — static thắng tham số
thắng wildcard ở cùng level — nếu không hai route đăng ký theo thứ tự
khác nhau sẽ match khác nhau, một bug đúng nghĩa mà người dùng trải nghiệm
như "route này hôm qua còn chạy".

### Chi phí bộ nhớ, và cách sửa
Một trie đánh chỉ số theo từng byte (thay vì theo từng segment) tạo ra các
chuỗi node dài chỉ có một con — một node cho mỗi ký tự của `/users` là sáu
bước nhảy để lưu một segment. Đây chính xác là vấn đề mà
[`13-algorithms/radix-tree.md`](radix-tree.md) sửa bằng cách gộp các chuỗi đó thành một
edge duy nhất; đọc file đó khi số lượng node của một trie theo segment
bắt đầu trở thành vấn đề.

## Practice
1. Trong [`labs/03-router`](../../labs/03-router), implement một trie theo segment (map đánh chỉ
   số theo path segment, không phải theo ký tự) và đăng ký cả route static
   lẫn route có tham số.
2. Đăng ký `/users/:id` và `/users/new`, viết quy tắc precedence khiến
   `/users/new` match route static thay vì bind `id = "new"`; test cả hai
   thứ tự đăng ký và xác nhận kết quả giống nhau.
3. Benchmark lookup của trie so với quét tuyến tính cùng bộ route ở 10,
   100, và 1000 route đã đăng ký; xác nhận thời gian lookup của trie giữ
   phẳng trong khi của lượt quét tăng lên.
4. Thêm hỗ trợ segment wildcard (`/static/*path`) và xác nhận nó chỉ match
   khi không có route static hay route tham số cụ thể hơn ở cùng vị trí.
