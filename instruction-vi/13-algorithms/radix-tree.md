# Radix Tree (Compressed Trie)

[`13-algorithms/trie.md`](trie.md) nói về trie thường và chi phí bộ nhớ của nó: các
chuỗi node dài chỉ có một con. Radix tree (còn gọi là Patricia trie) là
cách sửa vấn đề đó, và đây là thứ mà các HTTP router production thật sự
implement (`httprouter`, `gin`, router của `actix-web`) chứ không phải một
trie thường.

## What to learn

### Edge giữ cả chuỗi, không phải từng đơn vị
Trong khi edge của trie được gán nhãn bằng một ký tự hoặc một segment, edge
của radix tree được gán nhãn bằng một substring bất kỳ — cả một chuỗi node
lẽ ra chỉ nối tiếp nhau mà không rẽ nhánh được gộp lại thành một edge duy
nhất:

```rust
struct RadixNode {
    prefix: String,             // substring dùng chung mà edge này đại diện
    children: Vec<RadixNode>,   // các edge rẽ nhánh từ đây
    handler: Option<RouteHandler>,
}
```

Insert `/users` rồi `/user` tạo ra một node cho prefix dùng chung `/user`
với một nhánh rẽ: một child kết thúc chuỗi (handler của riêng `/user`, nếu
có đăng ký) và một child tiếp tục với `s` (handler của `/users`). Không có
chuỗi node một-con thừa thãi ở giữa.

### Insertion: tìm longest common prefix rồi split
Insert một key mới đi xuống dần, so sánh với `prefix` của từng edge cho tới
khi tìm được longest common prefix với một edge đã có. Có ba trường hợp
tiếp theo: key mới khớp chính xác một edge (gắn handler ngay đó), key mới
đi dài hơn một edge (đi xuống tiếp và lặp lại), hoặc key mới rẽ nhánh giữa
chừng một edge (split edge đó thành một node prefix dùng chung với hai
con — phần tiếp diễn cũ và phần mới). Làm đúng trường hợp split, kể cả xử
lý một handler đang gắn ở node bị split, là chỗ đa số bug của radix tree
tự viết từ đầu nằm ở đó.

### Segment tham số và wildcard vẫn cần precedence tường minh
Nén là một tối ưu bộ nhớ/lookup; nó không thay đổi ngữ nghĩa routing từ
[`13-algorithms/trie.md`](trie.md) — các segment static, tham số (`:id`), và wildcard
(`*rest`) vẫn cần đúng quy tắc precedence tường minh, không phụ thuộc thứ
tự đăng ký. Một radix tree thường giữ nhánh tham số và wildcard ở dạng
không nén (như các child riêng biệt tại điểm rẽ nhánh) chính vì chúng
không thể gộp vào một prefix literal cùng với một sibling static.

### Vì sao đây là lựa chọn production
Ít node hơn nghĩa là ít lần nhảy con trỏ hơn mỗi lần lookup và cache
locality tốt hơn — với một proxy phải check route trên mỗi request, đây là
một cấu trúc nằm trên hot path, không phải chi phí setup một lần. Độ phức
tạp tập trung hoàn toàn ở insertion (chỉ xảy ra một lần, lúc khởi động hoặc
khi config reload); lookup vẫn là cùng một kiểu đi xuống từng segment như
trie thường, chỉ là qua ít edge hơn và mỗi edge dài hơn.

## Practice
1. Chuyển trie trong [`labs/03-router`](../../labs/03-router) của bạn thành một radix tree: implement
   insertion theo longest-common-prefix kèm trường hợp split, và đăng ký
   cùng bộ route như bài tập của [`trie.md`](trie.md).
2. Đếm số node ở cả hai cách biểu diễn với một bộ route thực tế (một REST
   API với `/api/v1/users`, `/api/v1/users/:id`, `/api/v1/orders`,
   `/api/v1/orders/:id/items`, ...) và xác nhận radix tree dùng ít node hơn
   rõ rệt.
3. Cố tình kích hoạt trường hợp split-với-handler-đã-có (insert `/users`
   sau khi `/user` đã có handler) và viết một test xác nhận cả hai handler
   vẫn reachable sau đó.
4. Chạy lại benchmark của [`trie.md`](trie.md) (thời gian lookup ở 10/100/1000 route)
   trên radix tree và so sánh cả lookup latency lẫn bộ nhớ.
