# Leader Election

Chọn đúng một node để giữ một vai trò — bài toán con nằm dưới Raft
([`18-distributed/01-raft.md`](01-raft.md)), và thường giải được rẻ hơn nhiều khi tách
riêng. Optional/advanced so với [`proxy/`](../../proxy) một instance.

## What to learn

### Vì sao một fleet proxy muốn có một leader
Một số công việc phải chạy trên đúng một instance dù nhiều instance được
deploy: poll một service registry và đẩy config cho các instance khác,
chạy một cache-warm hoặc cleanup định kỳ, là writer duy nhất tới một store
chia sẻ. Chạy chúng trên mọi instance nhân đôi công việc hoặc làm hỏng
shared state; chạy trên không instance nào nghĩa là chúng không bao giờ
xảy ra. Leader election là cách fleet chỉ định một node, và tự chỉ định
lại khi node đó chết.

### Khó khăn cốt lõi: split-brain
Thất bại làm việc này khó là hai node đều tin mình là leader — split-brain
— xảy ra dưới một network partition khi mỗi bên không thấy được bên kia
và cho rằng nó đã chết. Hai leader viết vào shared state chính là sự hỏng
hóc bạn đang cố ngăn. Mọi giải pháp thật đều về bản chất là ngăn hoặc
bound split-brain, và câu trả lời luôn là một *đa số*: một node
chỉ có thể là leader nếu đa số đồng ý, và một partition chỉ có thể có
nhiều nhất một bên là đa số.

### Câu trả lời thực tế: một lease từ một store bạn đã có
Bạn hầu như không bao giờ tự implement election từ đầu. Pattern chuẩn là
một lease (lock) sống ngắn trong một store tự nó đã giải quyết consensus:

```text
leader = ai giữ key "leader" với một TTL;
  người giữ renew nó mỗi TTL/3;
  nếu người giữ chết, TTL expire và một node khác chiếm lấy nó.
```

etcd, Consul, và ZooKeeper expose chính xác cái này; `leader-election` của
Kubernetes (object `Lease`) chính là pattern này và là cách hầu hết service
Go/Rust trong môi trường k8s chọn một leader. Redis `SET NX PX` là phiên
bản nhà nghèo. Bạn đang mượn consensus đã đúng sẵn của store
([`18-distributed/01-raft.md`](01-raft.md)) thay vì tự suy dẫn lại nó.

Gotcha: TTL của lease là một trade-off thật. Quá dài và công việc của một
leader chết bị đứng suốt cả TTL trước khi failover; quá ngắn và một
GC pause ngắn hoặc network blip làm một leader healthy mất lease và gây
churn không cần thiết. Và quan trọng nhất — leader *cũ* phải ngừng hành
động ngay khi nó fail renew, trước khi TTL cho phép người khác vào, hoặc
bạn có hai leader hoạt động trong lúc overlap. Fence các write (kèm
version/epoch của lease vào mỗi write tới shared store, để store reject
write của một leader cũ) thay vì tin vào timing.

### Khi bạn không cần election chút nào
Thường thiết kế sạch hơn là không cần leader nào: làm job định kỳ
idempotent và để mọi instance chạy nó (trùng lặp vô hại), hoặc shard công
việc bằng consistent hashing ([`13-algorithms/consistent-hash.md`](../13-algorithms/consistent-hash.md)) để mỗi
key có một owner tự nhiên mà không cần một leader toàn cục, hoặc đẩy yêu
cầu single-writer xuống một store tự nó serialize write. Chọn một leader
thêm một failure mode (chính việc election); tránh nhu cầu cần một leader
loại bỏ nó. Ưu tiên cách đó khi công việc cho phép.

## Practice
1. Implement election dựa trên lease với Redis `SET key id NX PX <ttl>`
   cùng một renewal loop; chạy ba process và xác nhận đúng một giữ lease
   tại một thời điểm.
2. Kill leader và đo thời gian failover; liên hệ trực tiếp nó với TTL và
   renewal interval bạn chọn.
3. Tái tạo sự nguy hiểm: pause process leader (SIGSTOP) qua khỏi TTL của
   nó để một node khác chiếm lease, rồi resume nó và chỉ ra leader cũ
   trong chốc lát vẫn tin nó đang là leader — đây là split-brain thu nhỏ.
4. Thêm fencing: đóng dấu mỗi write vào shared store bằng epoch của lease
   và để store reject epoch cũ; chỉ ra nó trung hòa overlap ở bước 3.
5. Với một job fleet thật (poll config, cleanup cache), quyết định nên
   chọn một leader hay làm nó idempotent/sharded
   ([`13-algorithms/consistent-hash.md`](../13-algorithms/consistent-hash.md)) thay vào đó, và giải thích cái nào
   đơn giản hơn cho nhu cầu thật của [`proxy`](../../proxy).
