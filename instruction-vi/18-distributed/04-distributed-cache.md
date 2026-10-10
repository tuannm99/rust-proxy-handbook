# Distributed Cache

Trải một cache qua nhiều node khi một cache single-node
([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md), [`13-algorithms/lru.md`](../13-algorithms/lru.md)) không còn đủ. Topic
[`18-distributed/`](.) duy nhất connection trực tiếp nhất với một proxy — nhưng
vẫn vượt ngoài deliverable single-instance của [`proxy/`](../../proxy).

## What to learn

### Vì sao phải đi distributed
Một cache theo từng instance trong một fleet N proxy có hai vấn đề: cùng
một object bị cache N lần (N× bộ nhớ, N× miss origin khi cold start), và
hit rate bị cap bởi bộ nhớ của một instance. Một distributed cache làm
cho fleet chia sẻ một cache logic duy nhất — mỗi object sống trên một
(hoặc vài) node, nên tổng capacity là tổng của tất cả và mỗi object được
fetch từ origin đại khái một lần. Đây chính là bài toán edge-cache của
CDN.

### Đặt chỗ: consistent hashing, không phải modulo
Node nào sở hữu một key phải ổn định khi node join và leave, nếu không mỗi
lần thay đổi membership sẽ xáo trộn toàn bộ cache và làm origin bị
stampede. Đây chính xác là bài toán mà [`13-algorithms/consistent-hash.md`](../13-algorithms/consistent-hash.md)
(và [`13-algorithms/maglev.md`](../13-algorithms/maglev.md), [`13-algorithms/rendezvous-hash.md`](../13-algorithms/rendezvous-hash.md)) giải
quyết: `hash(key)` map vào một điểm trên một ring, key thuộc về node kế
tiếp theo chiều kim đồng hồ, và thêm một node chỉ chuyển các key trong một
cung, không phải tất cả. Dùng lại thuật toán đó trực tiếp ở đây —
distributed caching là ứng dụng tiêu biểu nhất của nó.

```text
key "GET /img/x" → hash → vị trí trên ring → node sở hữu
thêm một node → chỉ các key trong cung của nó di chuyển → ~1/N bị xáo trộn, không phải tất cả
```

### Hai topology
- **Client-side sharding.** Mỗi proxy biết ring và route một lượt tra
  cache thẳng tới peer sở hữu (hoặc tới một store chia sẻ như một pool
  Memcached/Redis được shard bằng consistent hash). Đơn giản, latency
  thấp, không tốn thêm một hop coordination. Đây là thiết kế proxy phổ
  biến.
- **Server-side / peer forwarding.** Một proxy nhận một request nó không
  sở hữu forward nó tới owner, owner cache và serve. Groupcache và các
  setup shared-cache-cluster của Nginx hoạt động theo cách này.

Gotcha: mỗi lượt tra distributed cache giờ là một thao tác *network*,
không phải một memory read. Một remote hit tốn một round-trip; nếu
round-trip đó gần bằng thời gian fetch từ origin, distributed cache không
đáng làm. Câu trả lời thường gặp là hai tầng — một cache *local* nhỏ,
nhanh ([`13-algorithms/lru.md`](../13-algorithms/lru.md)) đứng trước cache distributed — để các
object hot không bao giờ rời process và chỉ phần long tail đi qua network.

### Consistency và invalidation là phần khó
Cache trên các node trôi dần khỏi nhau. Khi một object bị purge hoặc
update, mọi node đang giữ nó phải biết — và không có câu trả lời
strong-consistency rẻ nào. Các công cụ thực tế: TTL ngắn để staleness tự
lành, versioned key để một update viết một key *mới* thay vì mutate, và
một purge broadcast (thường qua gossip, [`18-distributed/02-gossip.md`](02-gossip.md))
chấp nhận inconsistency trong chốc lát. Một proxy cache hầu như luôn chọn
eventual consistency ở đây — strong consistency
([`18-distributed/01-raft.md`](01-raft.md)) trên một cache ở data-path sẽ tốn nhiều hơn
nó tiết kiệm được.

### Thundering herd trên toàn fleet
Khi một object phổ biến expire, mọi proxy nhận request cho nó có thể hit
origin đồng thời — một stampede toàn fleet còn tệ hơn nhiều phiên bản
single-node (request coalescing của [`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)). Cách
sửa distributed là chỉ node *sở hữu* fetch từ origin và các node khác
coalesce vào nó, cộng với request-coalescing/single-flight trên owner đó.
Việc đặt chỗ (consistent hashing) chính là thứ làm cho "chỉ owner fetch"
khả thi.

## Practice
1. Mở rộng cache single-node từ [`labs/10-cache`](../../labs/10-cache) với việc đặt chỗ bằng
   consistent-hash ([`13-algorithms/consistent-hash.md`](../13-algorithms/consistent-hash.md)) trên một tập 3
   node giả lập; xác nhận một key luôn resolve về cùng một node.
2. Thêm một node và đo tỷ lệ key bị di chuyển — xác nhận nó là ~1/N, không
   phải tất cả, và đối chiếu với việc đặt chỗ `hash % N` vốn xáo trộn hết
   tất cả.
3. Xây hình dạng hai tầng: một LRU local nhỏ đứng trước lượt tra
   distributed, và đo lượng traffic network mà tầng local hấp thụ được
   trên một workload object hot.
4. Tái tạo một thundering herd toàn fleet khi expire, rồi sửa nó bằng
   origin fetch chỉ-owner cộng single-flight coalescing trên owner.
5. Implement purge với versioned key và một invalidation được gossip
   ([`18-distributed/02-gossip.md`](02-gossip.md)); suy luận rõ ràng về khoảng staleness
   và vì sao eventual consistency là lựa chọn đúng cho một proxy cache so
   với [`18-distributed/01-raft.md`](01-raft.md).
