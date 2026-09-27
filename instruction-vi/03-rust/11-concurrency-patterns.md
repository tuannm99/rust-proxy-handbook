# Concurrency Patterns: Channels and Message Passing

## What to learn

### Shared state vs. message passing
`03-rust/04-sync.md` bao quát shared state kiểu `Arc<Mutex<_>>`: nhiều
task đọc/ghi cùng một vùng nhớ dưới một lock. Lựa chọn thay thế là message
passing — các task không chia sẻ bộ nhớ, chúng gửi giá trị qua một
channel, và chỉ một task sở hữu dữ liệu tại một thời điểm. Mô hình
ownership của Rust khiến message passing dễ suy luận một cách bất thường:
gửi một giá trị vào một channel là một move thật sự, nên sender mất khả
năng chạm vào nó sau đó do cấu trúc, không phải do quy ước.

### Vườn thú channel của tokio, và khi nào nên dùng cái nào
- `mpsc` — nhiều sender, một receiver; hàng đợi công việc mặc định (nhiều
  connection handler đổ vào một task tổng hợp).
- `oneshot` — đúng một giá trị, đúng một lần; cách chuẩn để lấy một
  *phản hồi* từ một task bạn đã spawn (gửi một `oneshot::Sender` bên
  trong một message request, `.await` `oneshot::Receiver` tương ứng để
  lấy kết quả).
- `broadcast` — một-tới-nhiều: mọi receiver đều nhận mọi message (một tín
  hiệu config-reload được phát tới mọi connection handler — xem
  `09-architecture/03-config.md`).
- `watch` — giống broadcast nhưng chỉ giữ giá trị *mới nhất*; một receiver
  bắt đầu muộn chỉ thấy state hiện tại, không phải một backlog (trạng thái
  health-check trực tiếp, hoặc snapshot config hiện tại mà mỗi request
  handler đọc).

```rust
let (tx, rx) = tokio::sync::oneshot::channel();
tokio::spawn(async move {
    let result = do_upstream_call().await;
    let _ = tx.send(result); // ignore send error: receiver may have been dropped (caller cancelled)
});
let result = rx.await?;
```
Gotcha: gửi trên một channel `mpsc` bounded tự nó là một điểm `.await` có
thể block vô thời hạn nếu receiver chậm hoặc bị kẹt — đây là backpressure
đúng đắn, nhưng nó nghĩa là một lần gửi trên channel bounded đáng được
suy nghĩ về cancellation-safety giống như bất kỳ thao tác "đang chờ một
peer chậm" nào khác (đua nó với `tokio::select!` và một timeout, giống
phần thảo luận về cancellation trong `03-rust/05-async.md`).

### Actor pattern
Một "actor" là một task sở hữu độc quyền một phần state và chỉ để lộ nó
qua message trên một channel — không cần `Mutex`, vì chỉ mỗi task đó
chạm vào state trực tiếp. Điều này đánh đổi lock contention lấy chi phí
message-passing (một vòng đi-về qua channel cộng thêm một bước nhảy task),
và là hình dạng tự nhiên cho bất cứ thứ gì có invariant dễ bị vi phạm khi
mutate một phần: sliding window của một rate limiter
(`07-security/07-ratelimit.md`), một config store có thể hot-reload
(`09-architecture/03-config.md`), free-list của một connection pool
(`06-proxy/01-upstream.md`).

```rust
enum PoolMsg { Acquire(tokio::sync::oneshot::Sender<Conn>), Release(Conn) }

async fn pool_actor(mut rx: tokio::sync::mpsc::Receiver<PoolMsg>) {
    let mut free: Vec<Conn> = Vec::new();
    while let Some(msg) = rx.recv().await {
        match msg {
            PoolMsg::Acquire(reply) => { if let Some(c) = free.pop() { let _ = reply.send(c); } }
            PoolMsg::Release(conn) => free.push(conn),
        }
    }
}
```

### Chọn shared-state hay actor
Nếu critical section ngắn và contention thấp, `Arc<Mutex<_>>` đơn giản
hơn và thường nhanh hơn — không tốn bước nhảy task thêm. Nếu state có
invariant không tầm thường, dễ bị hỏng khi mutate một phần, hoặc bạn muốn
một điểm serialization rõ ràng, dễ instrument, một actor thường đáng cái
giá về throughput. Không cái nào là "lựa chọn idiomatic" duy nhất — một
codebase proxy thật dùng cả hai, tùy theo từng component.

## Practice
1. Xây một vòng request/reply với `oneshot`: spawn một worker làm việc và
   phản hồi, để caller await phản hồi đó, rồi drop receiver sớm (giả lập
   cancellation) và xác nhận `tx.send()` của worker fail một cách vô hại
   thay vì panic.
2. Implement connection pool của `06-proxy/01-upstream.md` một lần dưới
   dạng `Arc<Mutex<Vec<Conn>>>` và một lần dưới dạng một actor đứng sau
   một channel `mpsc`; load-test cả hai và so sánh latency/throughput dưới
   contention.
3. Dùng `tokio::sync::watch` để phát một tín hiệu config-reload tới vài
   task "handler" đang chạy; xác nhận một handler được spawn *sau* lần
   update cuối cùng vẫn thấy ngay giá trị hiện tại thay vì phải chờ lần
   update kế tiếp.
4. Tái tạo gotcha backpressure của `mpsc` bounded: làm receiver chậm một
   cách nhân tạo, gửi từ nhiều task, quan sát các sender bị nghẽn ở
   `.send().await`, rồi đua một lần gửi với một `tokio::time::timeout` để
   giới hạn thời gian caller phải chờ.
5. Chọn một component thật của `proxy` (rate limiter, connection pool,
   hoặc config store) và viết ra bạn sẽ chọn pattern nào và vì sao, trích
   dẫn đúng file handbook cho component đó.
