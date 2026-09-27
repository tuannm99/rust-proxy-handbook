# CAP Theorem và Kết quả bất khả thi FLP

## What to learn

### CAP: chọn hai trong ba, dưới partition
Với một data store phân tán, CAP theorem nói không thể đồng thời đảm bảo
cả ba: Consistency (mọi lần đọc thấy write mới nhất), Availability (mọi
request nhận được response không lỗi), và Partition tolerance (hệ thống
vẫn hoạt động khi giao tiếp network giữa các node hỏng) — và vì network
thật *sẽ* partition, lựa chọn thật trong thực tế là giữa C và A *trong
lúc* partition, không phải một lựa chọn tự do giữa cả ba. Đây là căn cứ
hình thức đằng sau mọi quyết định "eventual consistency ở đây" mà
[`18-distributed/`](../18-distributed) đưa ra.

```text
Partition xảy ra (network chia tách): phải chọn
-> từ chối một số request tới khi partition lành  (chọn C hơn A)
-> phục vụ request ở cả hai bên, chấp nhận rủi ro đọc cũ/xung đột  (chọn A hơn C)
```

### Nơi [`18-distributed/`](../18-distributed) đã đưa ra lựa chọn này, giờ nói rõ ra
Thiết kế purge/invalidation của [`18-distributed/04-distributed-cache.md`](../18-distributed/04-distributed-cache.md)
(TTL ngắn, versioned key, invalidation gossiped, "chấp nhận inconsistency
trong chốc lát") chọn Availability hơn Consistency trong lúc partition —
một cache phục vụ dữ liệu hơi cũ tốt hơn nhiều một cache ngừng phục vụ.
Consensus của [`18-distributed/01-raft.md`](../18-distributed/01-raft.md), ngược lại, chọn Consistency
hơn Availability theo thiết kế: một cluster Raft không đạt được đa số từ
chối commit write hoàn toàn thay vì rủi ro hai bên bất đồng — chính xác
là lý do Raft được dành cho control-plane state (config, leadership) và
không bao giờ cho data-plane request path.

### FLP: vì sao consensus không thể giải quyết mà không có thêm gì đó
Kết quả bất khả thi Fischer-Lynch-Paterson (FLP) chứng minh một điều
mạnh hơn CAP cho một bối cảnh hẹp hơn: trong một network hoàn toàn *bất
đồng bộ* (không có giới hạn về delay message) nơi dù chỉ một node có thể
crash, không giải thuật deterministic nào có thể đảm bảo consensus kết
thúc — không thể phân biệt đáng tin cậy "node đó chậm" với "node đó chết"
mà không có một giả định về timing nào đó. Đây không phải một giới hạn kỹ
thuật chờ được tối ưu đi — đó là một chứng minh.

### Cách mọi hệ thống consensus thật âm thầm né FLP
Vì mô hình bất đồng bộ, hoàn toàn đối kháng của FLP không thể giải quyết
nói chung, mọi hệ thống thật thêm một giả định mà FLP không cho phép:
Raft và Paxos dùng *timeout* (election timeout ngẫu nhiên trong
[`18-distributed/03-leader-election.md`](../18-distributed/03-leader-election.md) — một giả định partial-synchrony
rõ ràng rằng "im lặng đủ lâu có lẽ nghĩa là chết, không chỉ chậm"), đánh
đổi tính đúng hoàn hảo ở trường hợp xấu nhất để lấy sự kết thúc ở trường
hợp thường gặp. Đây chính xác là lý do phần thảo luận lease-TTL của
[`18-distributed/03-leader-election.md`](../18-distributed/03-leader-election.md) gọi lựa chọn TTL là "một trade-off
thật": quá ngắn tuyên bố sai một node còn sống là đã chết (một false
positive kiểu FLP), quá dài trì hoãn phục hồi. Không có TTL nào loại bỏ
được trade-off này; FLP nói không thể có TTL đó.

### Vì sao điều này quan trọng cho một proxy hầu như tránh distributed state
[`proxy`](../../proxy) là một instance đơn theo thiết kế ([`18-distributed/00-README.md`](../18-distributed/00-README.md)
nói rõ điều đó), điều này né hoàn toàn cả hai kết quả trên — CAP và FLP
là các định lý về hệ thống phải điều phối *state* qua network không đáng
tin cậy, và một proxy một instance không có state điều phối thì không
gặp vấn đề nào trong hai cái đó. Giá trị của việc biết chúng là nhận ra
chính xác thời điểm một quyết định thiết kế vượt qua ranh giới đó: ngay
khi [`proxy`](../../proxy) cần hai instance đồng thuận về bất cứ điều gì — một counter
rate-limit chia sẻ, một job được leader-elect, một cache được replicate —
CAP và FLP ngừng là lý thuyết và bắt đầu là ràng buộc thật về những gì
khả thi, bất kể implementation tốt đến đâu.

## Practice
1. Với thiết kế purge của [`18-distributed/04-distributed-cache.md`](../18-distributed/04-distributed-cache.md), viết
   ra rõ ràng nó hy sinh C hay A trong lúc network partition giữa các
   cache node, và vì sao đó là lựa chọn đúng cho một cache cụ thể.
2. Với consensus của [`18-distributed/01-raft.md`](../18-distributed/01-raft.md), làm tương tự — xác
   định nó hy sinh gì, và giải thích vì sao đó là lựa chọn đúng cho
   control-plane config thay vào đó.
3. Mô phỏng trực tiếp vấn đề cốt lõi của FLP: dựng một "leader election"
   3-node đồ chơi hoàn toàn không có timeout (chỉ message-passing, không
   đồng hồ), và chứng minh nó có thể treo vô hạn nếu message của một node
   chỉ đơn thuần bị delay thay vì mất — xác nhận không logic protocol cố
   định nào một mình phân biệt được "chậm" với "chết" nếu không có một
   giả định timing.
4. Thêm election timeout ngẫu nhiên vào cùng hệ thống đồ chơi đó (như
   Raft thật làm) và chỉ ra nó giờ kết thúc đáng tin cậy — nhưng dựng một
   pattern delay message bệnh lý vẫn gây ra một verdict "chết" sai cho
   một node còn sống, và nối lại với trade-off TTL trong
   [`18-distributed/03-leader-election.md`](../18-distributed/03-leader-election.md).
5. Chọn một tính năng thật hoặc giả định sẽ cần [`proxy`](../../proxy) chạy như nhiều
   hơn một instance điều phối (một rate limiter chia sẻ, một cache
   cluster-wide) và viết một đoạn về việc bạn sẽ chọn bên nào của CAP và
   vì sao, trích dẫn file [`18-distributed/`](../18-distributed) cụ thể đã đưa ra lựa chọn này
   cho một bài toán tương tự.
