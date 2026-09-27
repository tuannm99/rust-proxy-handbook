# Arc, Mutex, RwLock, Atomics

## What to learn

### `Arc<T>`: shared ownership xuyên thread
`Arc` (atomic reference count) cho phép nhiều owner chia sẻ an toàn một
vùng heap allocation xuyên qua các thread; clone một `Arc` chỉ tăng một
refcount, nó không copy `T`. Đây là cách chuẩn để chia sẻ upstream pool,
config snapshot, hay connection registry của một proxy cho mọi task xử lý
connection mà không phải copy nó trên từng request.

```rust
let pool: Arc<UpstreamPool> = Arc::new(build_pool());
let pool2 = pool.clone(); // cheap: one atomic increment
tokio::spawn(async move { pool2.pick_upstream(); });
```

Gotcha: chỉ riêng `Arc<T>` chỉ cho quyền truy cập chia sẻ *đọc* (`&T`) —
bạn cần interior mutability (`Mutex`/`RwLock`, hoặc atomics) để thực sự
mutate giá trị được chia sẻ, và `Arc<T>` yêu cầu `T: Send + Sync` để đi
qua ranh giới thread, điều mà compiler tự check giúp bạn.

### `Mutex<T>` vs `RwLock<T>`
`Mutex<T>` cho phép một thread truy cập độc quyền tại một thời điểm (kể cả
reader); `RwLock<T>` cho phép nhiều reader đồng thời HOẶC một writer. Với
bảng health của upstream trong một proxy — đọc trên mỗi request, hiếm khi
ghi bởi một health-checker chạy nền — `RwLock` cho phép hàng nghìn task
request đọc đồng thời mà không chặn lẫn nhau, đúng chính là access pattern
bạn muốn. Dùng `Mutex` khi đọc và ghi có tần suất tương đương, hoặc khi sự
đơn giản quan trọng hơn tính đồng thời khi đọc.

```rust
struct HealthTable(RwLock<HashMap<UpstreamId, bool>>);

impl HealthTable {
    fn is_healthy(&self, id: UpstreamId) -> bool {
        *self.0.read().unwrap().get(&id).unwrap_or(&false)
    }
}
```

Gotcha: giữ một `std::sync::MutexGuard` qua một điểm `.await` khiến future
trở thành `!Send` (guard không phải `Send`, và nó vẫn còn sống qua
await), điều này phá vỡ `tokio::spawn`. Hoặc dùng `tokio::sync::Mutex`
(an toàn để giữ qua `.await`, đánh đổi bằng việc chậm hơn với lock không
bị tranh chấp), hoặc tái cấu trúc để guard được drop trước khi await.

### Atomics và `Ordering`
Atomics (`AtomicU64`, `AtomicBool`, ...) cho phép đọc/ghi lock-free trên
một giá trị đơn — lý tưởng cho counter (số connection đang hoạt động,
request-per-second) trên hot path nơi một `Mutex` sẽ là chi phí không cần
thiết. `Ordering` kiểm soát việc reorder mà compiler/CPU được phép làm
quanh thao tác atomic đó: `Relaxed` cho các counter độc lập,
`Acquire`/`Release` khi một thao tác atomic cần thiết lập quan hệ
happens-before với vùng nhớ khác (ví dụ publish một con trỏ), `SeqCst` khi
bạn muốn thứ tự toàn cục dễ suy luận nhất (và chậm nhất).

```rust
static ACTIVE_CONNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
ACTIVE_CONNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
```

### Shared state vs message-passing
"Đừng giao tiếp bằng cách chia sẻ bộ nhớ; hãy chia sẻ bộ nhớ bằng cách
giao tiếp." Các channel `mpsc`/`broadcast`/`watch` của tokio cho phép bạn
thay một `Mutex<State>` chia sẻ bằng một task duy nhất sở hữu state và mọi
nơi khác gửi message cho nó — không lock contention, không rủi ro deadlock
từ thứ tự lock. Dùng `tokio::sync::watch` đặc biệt cho dữ liệu kiểu "giá
trị mới nhất, nhiều reader" như một config được reload trực tiếp
([`09-architecture/03-config.md`](../09-architecture/03-config.md)); dùng `RwLock`/`Arc` chia sẻ khi dữ liệu
lớn và clone nó mỗi lần update sẽ lãng phí (ví dụ một routing table lớn).

Gotcha: dưới tải thực tế, lock contention trên một `Mutex<Vec<Upstream>>`
ngây thơ được chia sẻ bởi mọi task request là một nút thắt cổ chai tự gây
ra rất phổ biến — đo đạc trước khi nhảy sang các cấu trúc lock-free, nhưng
hãy biết rằng `RwLock` và `watch` là hai lối thoát đầu tiên nên thử.

## Practice
1. Xây một `HealthTable` như trên với `Arc<RwLock<HashMap<...>>>`, spawn 8
   task đọc nó trong một vòng lặp và 1 task ghi vào nó mỗi 100ms; xác nhận
   các reader không bị chặn lẫn nhau.
2. Tái tạo lỗi "future cannot be sent between threads" bằng cách giữ một
   `std::sync::MutexGuard` qua một `.await`, rồi sửa nó bằng
   `tokio::sync::Mutex` và sửa lại lần nữa bằng cách tái cấu trúc để drop
   guard trước.
3. Thay một counter được bảo vệ bởi `Mutex<u64>` bằng `AtomicU64` và xác
   nhận bằng một benchmark nhanh rằng nó nhanh hơn khi có contention.
4. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), quyết định xem upstream pool của bạn là
   `Arc<RwLock<Vec<Upstream>>>` hay thuộc sở hữu một task và được truy cập
   qua một channel `tokio::sync::watch` — implement một trong hai, và viết
   một câu giải thích vì sao bạn không chọn cái còn lại.
5. Cố tình tạo ra một deadlock hai-lock (task A lock X rồi Y, task B lock
   Y rồi X) rồi sửa nó bằng cách thiết lập một thứ tự lock nhất quán.
