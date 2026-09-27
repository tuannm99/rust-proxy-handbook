# Distributed

Điều phối đa node — ngoài phạm vi của một instance proxy L7 đơn lẻ, nhưng
là nền tảng nếu handbook này có lúc phát triển hướng tới một distributed
cache hoặc CDN. Coi thư mục này là optional/advanced so với
`proxy/00-README.md`.

## Trạng thái: đã viết, nhưng thực sự optional

Nội dung dưới đây đã được viết, nhưng khác với phần còn lại của handbook,
nó nằm *ngoài* deliverable theo thiết kế: `proxy/` là một proxy L7 một
instance, và không có gì trong đó cần consensus hay điều phối đa node. Mỗi
file ở đây đều mở đầu bằng "bạn có lẽ không cần cái này" và chỉ vào câu trả
lời single-node rẻ hơn. Bỏ qua hoàn toàn thư mục này là một cách hợp lệ để
hoàn thành handbook.

Nơi duy nhất ranh giới này đụng vào công việc của bạn là distributed rate
limiting (bài tập stretch của `07-security/07-ratelimit.md`, giải bằng một
counter Redis chia sẻ, không phải Raft) và một cache chia sẻ
(`04-distributed-cache.md`, file gần với công việc proxy thật nhất ở đây).
Chỉ quay lại nếu bạn mở rộng vượt `proxy/00-README.md` hướng tới một
multi-node cache hoặc CDN.

## Files

- `01-raft.md` — consensus dựa trên leader, cơ chế phía sau hầu hết các hệ
  thống điều phối production
- `02-gossip.md` — lan truyền state kiểu epidemic, được các hệ thống như
  Consul/Cassandra dùng cho membership
- `03-leader-election.md` — bài toán con cụ thể mà Raft (và các lựa chọn
  đơn giản hơn) giải quyết
- `04-distributed-cache.md` — sharding/replicate một cache qua các node,
  khi một cache single-node (`05-http-stack/07-cache.md`) không còn đủ

Consistent hashing được bao quát trong `13-algorithms/consistent-hash.md`,
và discovery cho single-service-instance trong
`06-proxy/07-service-discovery.md` — không lặp lại ở đây.
