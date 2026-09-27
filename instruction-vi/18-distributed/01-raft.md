# Raft

Consensus dựa trên leader — cách một tập node đồng thuận trên một log các
thay đổi có thứ tự dù một số node fail. Ngoài phạm vi của [`proxy/`](../../proxy) một
instance; chỉ đọc file này nếu bạn mở rộng hướng tới một control plane hoặc
cache đa node ([`18-distributed/04-distributed-cache.md`](04-distributed-cache.md)).

## What to learn

### Bài toán nó giải quyết
Consensus là làm cho N node đồng thuận trên một chuỗi giá trị dù có crash
và network delay, sao cho chúng không bao giờ bất đồng. Đó là nền tảng
dưới mọi hệ thống lưu config hay coordination state một cách đáng tin cậy
— etcd, Consul, ZooKeeper (qua ZAB), CockroachDB. Nếu proxy của bạn có lúc
cần một control plane *chia sẻ, nhất quán* (mọi instance đồng thuận về
routing config hiện tại, hoặc về instance nào sở hữu một shard), Raft là
cơ chế bạn nên dùng thay vì tự sáng chế.

### Ba phần
Raft chủ động chia consensus thành các phần bạn có thể suy luận riêng lẻ:

- **Leader election.** Một node là leader mỗi *term* (một số tăng dần đơn
  điệu). Follower không nghe gì từ leader trước một timeout ngẫu nhiên trở
  thành candidate và request vote; một candidate có đa số trở thành leader.
  Timeout ngẫu nhiên là thứ phá vỡ đối xứng để hai candidate hiếm khi hòa —
  xem [`18-distributed/03-leader-election.md`](03-leader-election.md).
- **Log replication.** Client gửi thay đổi tới leader, leader append vào
  log của nó và replicate cho follower. Một entry được *commit* khi đa số
  đã lưu nó; chỉ sau đó nó được apply vào state machine và ack. Đây là lý
  do Raft cần cluster size lẻ (3, 5) — một đa số phải sống sót.
- **Safety.** Một leader chỉ bao giờ append; nó không bao giờ overwrite
  log của nó. Các quy tắc election đảm bảo một node thiếu các entry đã
  commit không thể thắng, nên history đã commit không bao giờ mất.

### Phép toán quorum điều khiển mọi thứ
Một cluster `2f+1` node chịu được `f` lần fail, vì commit cần đa số
(`f+1`). Ba node sống sót một lần fail; năm node sống sót hai lần. Số node
chẵn không mua được gì — bốn node vẫn chỉ chịu được một lần fail nhưng tốn
nhiều coordination hơn, nên cluster size là số lẻ.

```text
3 node → đa số 2 → chịu được 1 lần fail
5 node → đa số 3 → chịu được 2 lần fail
```

Gotcha: yêu cầu đa số này cũng là chi phí của Raft. Mỗi write được commit
chờ một round-trip tới đa số, nên consensus *chậm* so với một thao tác cục
bộ — vài ngàn write/giây trên một cluster WAN, không phải hàng triệu.
Không bao giờ đưa một hot path mỗi request qua Raft. Nó dành cho
control-plane state ít thay đổi (config, membership, leadership), không
phải cho data-plane traffic.

### Không tự implement nó
Raft nổi tiếng là "dễ hiểu" so với Paxos, và vẫn nổi tiếng là tinh vi khi
implement đúng — leader completeness, log compaction/snapshotting, thay
đổi membership, và các quy tắc commit chính xác đều giấu bug chỉ lộ ra
dưới partition. Dùng một library đã được chứng minh (`openraft`, `raft-rs`)
nếu bạn thực sự cần consensus. Giá trị của việc hiểu nó là biết *khi nào*
bạn cần — và, thường xuyên hơn nhiều, nhận ra khi nào bạn không cần.

### Khi bạn gần như chắc chắn không cần nó
Hầu hết các nhu cầu "distributed" mà một proxy có yếu hơn consensus và có
giải pháp rẻ hơn: counter rate-limit chia sẻ dùng Redis
([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)), không phải Raft; membership/discovery
dùng gossip ([`18-distributed/02-gossip.md`](02-gossip.md)) hoặc một registry có sẵn
([`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md)); một cache chia sẻ dùng consistent
hashing ([`13-algorithms/consistent-hash.md`](../13-algorithms/consistent-hash.md)) và chấp nhận inconsistency.
Chỉ dùng consensus khi các node *không bao giờ* được bất đồng về một
history có thứ tự.

## Practice
1. Vẽ trên giấy một write được commit qua một Raft 3-node: client → leader
   append → replicate → majority ack → commit → apply. Xác định chính xác
   điểm nó trở nên durable.
2. Tính failure tolerance cho 3, 4, 5, và 7 node và trình bày vì sao số
   chẵn là lãng phí.
3. Suy luận về một partition: một cluster 5 node chia 3/2. Bên nào có thể
   commit, bên nào không, và vì sao bên minority từ chối thay vì fork
   history?
4. Với mỗi nhu cầu "distributed" trong handbook này — rate limiting
   ([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)), service discovery
   ([`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md)), cache chia sẻ
   ([`18-distributed/04-distributed-cache.md`](04-distributed-cache.md)) — quyết định nó có thực sự
   cần consensus hay một cơ chế yếu hơn, và giải thích từng cái.
5. Chỉ khi bạn mở rộng [`proxy/`](../../proxy) thành một control plane đa node: dựng một
   cluster 3 node với `openraft` lưu routing config, và kill leader dưới
   load để xem election và tính liên tục — không tự viết tay algorithm.
