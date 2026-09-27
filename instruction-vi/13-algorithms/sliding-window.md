# Sliding Window Rate Limiting

`07-security/07-ratelimit.md` và `13-algorithms/token-bucket.md` nói về
token bucket và leaky bucket. File này nói về họ còn lại: giới hạn bằng
cách đếm số request trong một cửa sổ thời gian di chuyển thay vì mô hình
hóa một cái bucket.

## What to learn

### Fixed window counter: đơn giản, và sai ở biên
Phiên bản ngây thơ: một counter cho mỗi `(key, window)`, tăng theo từng
request, reset khi window lăn qua (ví dụ mỗi phút theo đồng hồ tường).

```rust
struct FixedWindow {
    window_start: std::time::Instant,
    window_len: std::time::Duration,
    count: u32,
    limit: u32,
}
```

Gotcha: cách này cho phép đúng `2 * limit` request trong bất kỳ khoảng
một-window thực tế nào, không phải `limit`. Một client gửi `limit` request
vào mili giây cuối cùng của window N và thêm `limit` nữa vào mili giây đầu
tiên của window N+1 không bao giờ thực sự vượt counter của window nào cả,
nhưng phía origin thấy `2 * limit` request trong ~2ms. Cú burst ở biên này
là kiểu thất bại kinh điển mà các limiter fixed-window mắc phải một cách
vô tình.

### Sliding window log: chính xác, nhưng tốn O(n) bộ nhớ
Lưu timestamp của mọi request trong window đang trôi (ví dụ một
`VecDeque<Instant>` cho mỗi key); mỗi request, bỏ các timestamp cũ hơn
`now - window_len`, rồi kiểm tra `deque.len() < limit`. Cách này chính xác
hoàn toàn — không có burst ở biên — vì window thực sự liên tục, không bị
chia bucket.

Gotcha: bộ nhớ tỷ lệ với `limit`, cho mỗi key. Ở limit 10.000 req/window
trên một triệu key, đó có thể là 10 tỷ timestamp được lưu. Cách này không
scale nổi cho việc giới hạn theo IP hoặc theo token của một proxy nếu
không kèm theo một giới hạn bộ nhớ.

### Sliding window counter: xấp xỉ mà ai cũng thực sự dùng
Giữ hai fixed-window counter — window hiện tại và window trước — và trọng
số hóa count của window trước theo mức nó còn overlap với window đang
trôi:

```rust
fn sliding_count(prev_count: u32, curr_count: u32, elapsed_into_curr: f64, window_len: f64) -> f64 {
    let prev_weight = (window_len - elapsed_into_curr) / window_len;
    curr_count as f64 + prev_count as f64 * prev_weight
}
// reject nếu sliding_count(..) >= limit
```

Đây là cách rate limiter của Cloudflare và Kong implement: bộ nhớ O(1) cho
mỗi key (hai số nguyên, không phải một log), và nó loại bỏ vấn đề burst ở
biên trong một sai số xấp xỉ bị chặn trên — giả định rằng request rải đều
trong window trước, đủ gần với thực tế và có thể chứng minh worst case chỉ
bị chặn ở `2x` trong đúng kiểu mẫu bệnh hoạn dồn hết vào các biên, và ngay
cả khi đó cũng bị hạ trọng số chứ không đếm đầy đủ.

Gotcha: phiên bản hai counter cần cả hai counter được đọc và lăn qua
(roll over) atomic với nhau, nếu không một request rơi đúng vào biên
window có thể đọc phải một trạng thái lăn nửa chừng (count trước đã reset
về 0, count hiện tại chưa thực sự là "hiện tại") và đếm thiếu. Hãy lăn
window như một thao tác duy nhất — swap `curr` vào `prev` và zero hóa
`curr` — dưới cùng một lock/CAS với thao tác tăng count, không phải hai
bước riêng biệt.

### Chọn so với token bucket
Token bucket và sliding window counter giải cùng một bài toán với đánh
đổi khác nhau: token bucket tự nhiên cho phép một burst tới `capacity` rồi
throttle về tốc độ ổn định, mãi mãi, với state O(1) không bao giờ cần một
"window trước". Sliding window counter thực thi "không quá N trong bất kỳ
window đang trôi nào", một cách đọc chặt chẽ và sát nghĩa đen hơn của "N
req/giây" — hữu ích khi một SLA downstream được phát biểu theo kiểu đó —
nhưng cần sổ sách của window trước và chỉ xấp xỉ count thật. Chọn dựa trên
việc giới hạn đó thực sự đang hứa hẹn điều gì với một client hoặc một
downstream.

## Practice
1. Trong `labs/11-rate-limit`, implement fixed window counter trước và viết
   tường minh test burst-ở-biên: gửi `limit` request tại `window_end - 1ms`
   và thêm `limit` nữa tại `window_end + 1ms`, và quan sát cả hai đều
   thành công dù đạt `2x` tốc độ dự định trong ~2ms.
2. Implement sliding window log và chạy lại cùng test đó; xác nhận nó
   reject đúng, rồi đo bộ nhớ của nó ở một `limit` lớn và nhiều key để
   thấy vì sao nó không scale nếu không sửa đổi.
3. Implement sliding window counter và chạy lại test biên lần thứ ba; xác
   nhận overshoot quan sát được bị chặn trên và nhỏ hơn nhiều so với `2x`
   của phiên bản fixed-window.
4. Cố tình tái hiện bug atomic-rollover (lăn `curr`→`prev` và reset thành
   hai bước riêng biệt, không atomic, dưới truy cập đồng thời), quan sát
   việc đếm thiếu, rồi sửa nó.
5. (Nâng cao) So sánh sliding window counter với implementation GCRA của
   bạn trong `token-bucket.md` trên cùng một trace traffic có burst, và mô
   tả, bằng lời của bạn, cái nào một client sẽ cảm thấy "bất công" hơn và
   vì sao.
