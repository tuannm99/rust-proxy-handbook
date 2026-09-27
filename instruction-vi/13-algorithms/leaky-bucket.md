# Leaky Bucket

[`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md) giới thiệu leaky bucket như "một hàng đợi
xả với tốc độ không đổi." File này nói về hai cách câu đó thực sự được
implement, vì chúng hành xử rất khác nhau dưới tải.

## What to learn

### Dạng meter: leaky bucket là token bucket lộn ngược
Cách diễn giải "leaky bucket như một đồng hồ đo" theo dõi một mực nước
tăng lên ở mỗi request và rút (rò rỉ) với tốc độ không đổi; một request bị
từ chối nếu nó sẽ làm tràn capacity của bucket.

```rust
struct LeakyBucketMeter {
    level: f64,       // mực "nước" hiện tại
    capacity: f64,
    leak_rate: f64,   // đơn vị rút mỗi giây
    last_leak: std::time::Instant,
}

impl LeakyBucketMeter {
    fn try_add(&mut self, now: std::time::Instant, cost: f64) -> bool {
        let elapsed = now.duration_since(self.last_leak).as_secs_f64();
        self.level = (self.level - elapsed * self.leak_rate).max(0.0);
        self.last_leak = now;
        if self.level + cost <= self.capacity {
            self.level += cost;
            true
        } else {
            false
        }
    }
}
```

Về mặt toán học đây là ảnh gương của token bucket (đổ đầy so với rút cạn,
từ chối-khi-đầy so với từ chối-khi-rỗng) và cho ra *cùng* quyết định
accept/reject như token bucket với cùng capacity và rate — xem GCRA trong
[`13-algorithms/token-bucket.md`](token-bucket.md). Nếu bạn đã có một token bucket đúng, bạn
không cần implement riêng dạng này; nó tồn tại chủ yếu vì tài liệu của các
nhà cung cấp (AWS, tài liệu quota của GCP, một số API gateway) mô tả
limiter của họ theo cách này.

### Dạng queue: leaky bucket là một buffer bị giới hạn tốc độ, không phải một meter
Cách diễn giải còn lại là một hàng đợi FIFO có giới hạn thật sự, được một
tiến trình nền rút ra với tốc độ cố định, mỗi lần một request:

```rust
struct LeakyBucketQueue<T> {
    queue: std::collections::VecDeque<T>,
    capacity: usize,
    drain_interval: std::time::Duration,
}
// enqueue: từ chối nếu queue.len() == capacity, ngược lại push_back
// một task chạy theo chu kỳ cố định pop một item và forward nó xuống dưới
```

Đây là một đảm bảo về bản chất khác với dạng meter: nó **làm mượt output
về đúng một tốc độ không đổi** — phía dưới không bao giờ thấy nhiều hơn
một request mỗi `drain_interval`, bất kể input bùng nổ (bursty) tới đâu,
miễn là hàng đợi còn chỗ. Cả token bucket lẫn dạng meter của leaky bucket
đều *cho phép* một burst đi qua ngay lập tức tới giới hạn capacity; dạng
queue thì không bao giờ. Dùng nó cụ thể khi thứ đang được bảo vệ thực sự
không thể hấp thụ một burst (một backend cũ với capacity cố định, một
thiết bị phần cứng chỉ xử lý một request tại một thời điểm) thay vì cho
"sử dụng API công bằng," nơi burst thường không sao.

### Failure mode thật của dạng queue: latency, không phải rejection
Một meter/token-bucket đầy sẽ từ chối ngay lập tức với một 429. Một
*queue* leaky bucket đầy có thể được cấu hình để hoặc từ chối khi đầy
(hàng đợi có giới hạn) hoặc block caller cho tới khi có chỗ — và phiên bản
blocking biến một vấn đề rate-limit thành một vấn đề latency: ở
`capacity = 1000` và `drain_rate = 10/giây`, một request thấy hàng đợi
đầy sẽ chờ tới 100 giây trước khi được phục vụ hoặc timeout. Điều này
thường tệ hơn cho caller so với một 429 ngay lập tức, vì timeout của chính
caller có thể nổ trước, và nó giờ giữ một kết nối/thread mở suốt thời gian
chờ.

Gotcha: một hàng đợi *không giới hạn* loại bỏ hoàn toàn trường hợp
rejection và biến quá tải kéo dài thành tăng trưởng bộ nhớ vô hạn *và*
latency vô hạn cùng lúc — hàng đợi không "rò rỉ" đủ nhanh để theo kịp, và
không gì ngăn nó lớn thêm. Luôn giới hạn hàng đợi, và ưu tiên từ chối khi
hàng đợi đầy hơn là block caller trong ngữ cảnh proxy, nơi mỗi task bị
block giữ tài nguyên (một kết nối, một task) suốt thời gian chờ.

## Practice
1. Trong [`labs/11-rate-limit`](../../labs/11-rate-limit), implement dạng meter và xác nhận, trên một
   timeline request có kịch bản, nó accept/reject giống hệt implementation
   [`token-bucket.md`](token-bucket.md) của bạn với capacity/rate tương ứng.
2. Implement dạng queue với một `VecDeque` có giới hạn và một task nền rút
   theo chu kỳ cố định; thêm một `try_enqueue` từ chối ngay khi đầy.
3. Chứng minh trực tiếp sự khác biệt về làm mượt: bắn một burst
   `2 * capacity` request vào cả token bucket lẫn leaky-bucket queue của
   bạn, và vẽ (hoặc chỉ in ra, có timestamp) khi nào mỗi request được cho
   qua — token bucket cho qua ngay `capacity` request đầu tiên, queue rỉ
   ra tất cả chúng theo `drain_rate`.
4. Đổi dạng queue thành block-cho-tới-khi-có-chỗ thay vì
   từ-chối-khi-đầy, đo thời gian chờ tệ nhất ở capacity/rate bạn đã chọn,
   và viết ra vì sao bạn sẽ hoặc sẽ không ship hành vi đó trong [`proxy/`](../../proxy).
