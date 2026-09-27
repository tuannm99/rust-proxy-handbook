# Gossip

Lan truyền state kiểu epidemic — cách một fleet lớn biết về membership và
health mà không cần một coordinator trung tâm. Optional/advanced so với
[`proxy/`](../../proxy); liên quan nếu bạn chạy nhiều instance proxy cần biết về nhau.

## What to learn

### Ý tưởng, và khác biệt với consensus
Trong một gossip protocol, mỗi node định kỳ chọn vài peer ngẫu nhiên và
trao đổi state; thông tin lan như một dịch bệnh, đến toàn fleet trong
`O(log N)` round. Điểm mấu chốt là nó cung cấp *eventual* consistency,
không phải sự đồng thuận mạnh của Raft ([`18-distributed/01-raft.md`](01-raft.md)) — các
node có thể bất đồng trong chốc lát, và đó là sự đánh đổi có chủ đích để
lấy scale và khả năng chịu partition. Consensus dành cho state không bao
giờ được lệch; gossip dành cho state mà "mọi người converge trong vài giây"
là ổn, đúng chính xác cho membership và health.

### Nó được dùng để làm gì
Use case kinh điển là cluster membership và failure detection — node nào
tồn tại và node nào còn sống. Đây là thứ Consul, Cassandra, và Serf dùng
(tất cả xây trên SWIM hoặc một biến thể). Với một fleet proxy, lợi ích là
nhận biết health phân tán: thay vì mỗi proxy tự poll từng backend
([`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md)), các node gossip các quan sát health, nên
fleet converge về "backend X đã down" với tổng traffic probe ít hơn nhiều.

### SWIM: bộ phát hiện fail đáng biết
SWIM (Scalable Weakly-consistent Infection-style Membership) là chuẩn hiện
đại vì nó tách hai vấn đề mà gossip ngây thơ trộn lẫn:

- **Failure detection** bằng ping trực tiếp, với probing *gián tiếp* như
  cơ hội thứ hai — nếu A không reach được B, A hỏi vài node khác ping B
  giúp nó trước khi tuyên bố B đã chết. Đây là thứ triệt tiêu false
  positive từ một đường link tệ đơn lẻ.
- **Dissemination** các thay đổi membership được piggyback trên các
  message ping đó, nên detection và propagation chia sẻ traffic.

```text
A ──ping──▶ B      (không có reply)
A ──"ping B giúp tôi"──▶ C, D   (probe gián tiếp trước khi tuyên bố B chết)
```

Gotcha: bước probe gián tiếp là toàn bộ lý do SWIM dùng được trong
production. Không có nó, một đường mạng chập chờn giữa hai node đánh dấu
một node healthy là chết và tin sai đó gossip khắp fleet. Bất kỳ hệ thống
gossip nào bạn xây hoặc cấu hình phải có cơ chế ý-kiến-thứ-hai này, không
thì nó sẽ flap.

### Anti-entropy so với rumor-mongering
Hai kiểu propagation, thường kết hợp: *rumor-mongering* lan một fact mới
mạnh mẽ trong một thời gian rồi dừng (nhanh, nhưng một node bỏ lỡ nó vẫn
stale), và *anti-entropy* định kỳ reconcile toàn bộ state giữa các peer
(chậm, nhưng đảm bảo eventual convergence và sửa các rumor bị bỏ lỡ). Hệ
thống production chạy cả hai — rumor để nhanh, anti-entropy như lưới an
toàn.

### Mô hình chi phí
Gossip đổi latency và consistency để lấy scalability và resilience: không
có single point of failure, load trải đều, nhưng convergence tốn vài round
và các node tạm thời inconsistent. Tune fanout (số peer mỗi round) và
period theo kích thước fleet — quá aggressive lãng phí băng thông, quá lười
làm chậm convergence và failure detection.

### Không tự viết tay
Giống Raft, gossip đúng đắn là tinh vi (triệt tiêu false-positive,
khuếch đại message, incarnation number để giải quyết state cũ-so-với-mới).
Dùng `memberlist` (Go, qua Serf) làm tham chiếu, hoặc một crate SWIM Rust,
thay vì tự sáng chế. Hiểu nó cho bạn biết khi nào health phân tán thắng
polling tập trung — mà với một fleet proxy nhỏ, thường là không.

## Practice
1. Mô phỏng lan truyền rumor: N node, mỗi round mỗi node "đã nhiễm" nói
   cho `k` peer ngẫu nhiên; đo số round để phủ toàn bộ và xác nhận hình
   dạng `O(log N)` khi bạn scale N.
2. Thêm một đường link chập chờn giữa hai node và chỉ ra detection trực
   tiếp thuần túy tạo ra một verdict "chết" sai; rồi thêm probing gián
   tiếp kiểu SWIM và chỉ ra nó triệt tiêu false positive đó.
3. So sánh tổng traffic probe cho health fleet theo hai cách: mọi proxy
   poll mọi backend ([`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md)) so với các quan sát
   health được gossip, khi số fleet và backend tăng.
4. Suy luận về convergence so với consistency: dựng một thời điểm hai
   proxy bất đồng về health của một backend và quyết định sự bất đồng
   tạm thời đó có chấp nhận được cho traffic *của bạn* không (thường được
   với load balancing, không được với billing).
5. Chỉ nếu bạn chạy một fleet đa instance: nối một library SWIM vào
   [`proxy`](../../proxy) cho membership của instance và quan sát một instance bị kill
   được phát hiện và loại bỏ — không tự implement protocol.
