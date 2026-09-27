# Token Bucket

[`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md) nói về câu hỏi chính sách — theo từng client
hay toàn cục, giới hạn phân tán, khi nào nên dùng leaky bucket thay vào
đó. File này nói về việc làm cho bản thân bộ đếm đúng và nhanh.

## What to learn

### Lazy refill và state bạn thực sự cần
Công thức refill-khi-đọc trong [`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md) (tokens +
elapsed × rate, chặn trên ở capacity) là đúng: không cần timer nền cho
mỗi key, và state chỉ là một số token cộng một timestamp. Hai chi tiết
quyết định nó đúng hay không:

**Chặn trên trước khi tiêu thụ, không phải sau.** Một bucket rảnh phải
clamp về `capacity` khi refill, nếu không một client im lặng một giờ sẽ
tích lũy cả giờ token và đổ toàn bộ backlog đó ra cùng lúc — đúng cú spike
traffic mà limiter tồn tại để ngăn chặn.

**Đừng bao giờ để clamp chạy ngược.** Nếu `now` sớm hơn `last_refill`,
`elapsed` âm và token bị *lấy đi*. Đây không phải chuyện giả định:
`Instant` là monotonic và an toàn, nhưng ngay khi bạn lưu `SystemTime` (để
share bucket giữa các process, hoặc để persist chúng), một bước lùi NTP sẽ
âm thầm rút cạn mọi bucket. Dùng `Instant` cục bộ, và nếu timestamp phải
vượt qua ranh giới process, clamp elapsed về 0.

### GCRA: cùng một limiter với một con số
Generic Cell Rate Algorithm lưu một timestamp duy nhất — "thời điểm đến lý
thuyết" (TAT) của request được phép tiếp theo — thay vì một số thực cộng
một timestamp:

```rust
struct Gcra {
    tat: std::time::Instant, // khi request tiếp theo sẽ đúng-tốc-độ
}
// accept nếu now >= tat - burst_tolerance; khi accept, tat = max(tat, now) + interval
```

`interval` là `1/rate` và `burst_tolerance` là `capacity × interval`. Nó
tương đương về mặt toán học với token bucket, nhưng state chỉ là một
`Instant` (8-16 byte) không có sai số trôi dấu phẩy động, và nó cho luôn
giá trị `Retry-After` miễn phí: `tat - burst_tolerance - now` chính xác là
thời gian client phải chờ. Crate `governor` được xây trên nó.

Gotcha: số token dạng dấu phẩy động tích lũy sai số làm tròn qua hàng
triệu lần refill, nên một bucket lẽ ra phải ở đúng `capacity` sẽ trôi
xuống thấp hơn một chút và từ chối một request lẽ ra phải cho qua. Số học
integer/duration của GCRA né hoàn toàn vấn đề này.

### Làm nó concurrent
`Mutex<HashMap<IpAddr, TokenBucket>>` ngây thơ serialize mọi request trong
proxy trên một lock. Hai cách sửa, theo thứ tự:

1. **Shard cái map** — `dashmap`, như [`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md) gợi
   ý, tự shard bên trong để các key khác nhau hiếm khi tranh chấp.
2. **Làm chính bucket lock-free** — đóng gói TAT của GCRA vào một
   `AtomicU64` (nanosecond kể từ một epoch cố định) và cập nhật bằng một
   vòng lặp compare-and-swap. Khi CAS fail, đọc lại và thử lại; vòng lặp
   kết thúc vì một writer cạnh tranh chỉ bao giờ đẩy TAT về phía trước.

Gotcha: đọc-rồi-ghi dưới một read guard của `RwLock` là một race, không
phải một tối ưu. Hai request cùng đọc 1 token, cùng quyết định "cho phép",
cùng ghi 0 — limiter bị rò rỉ. Token bucket không có đường đi chỉ-đọc; việc
kiểm tra và trừ token phải là một thao tác atomic duy nhất.

### Số lượng key không bị chặn trên
Một map theo địa chỉ IP nguồn nằm dưới quyền kiểm soát của attacker: các
nguồn giả mạo hoặc phân tán mỗi cái tạo một entry, và cái map trở thành một
vector làm cạn kiệt bộ nhớ ([`07-security/09-ddos.md`](../07-security/09-ddos.md)). Hãy chặn nó bằng
một trong các cách:
- **Quét dọn entry rảnh.** Một bucket ở đầy capacity không mang thông tin
  gì cả — xóa nó tương đương với giữ nó. Quét bất cứ thứ gì không bị chạm
  tới trong vài khoảng refill.
- **Chặn trên cái map** và evict LRU ([`13-algorithms/lru.md`](lru.md)) khi vượt
  giới hạn.
- **Đếm xấp xỉ kích thước cố định.** Hash key vào một mảng bucket kích
  thước cố định và chấp nhận va chạm gộp giới hạn của hai client — bộ nhớ
  bị chặn trên bằng cấu trúc, đổi lại là thỉnh thoảng từ chối nhầm. Một
  count-min sketch ([`13-algorithms/count-min-sketch.md`](count-min-sketch.md)) là phiên bản bài
  bản của cách này.

Gotcha: quét theo timer trong khi request đang chạm vào map đồng thời cần
việc quét và việc chạm đồng thuận về ý nghĩa "rảnh", nếu không bạn xóa một
bucket đang giữa chừng được một request cập nhật và tặng client đó một lần
reset miễn phí. Check-and-remove phải atomic với đường cập nhật.

## Practice
1. Trong [`labs/11-rate-limit`](../../labs/11-rate-limit), implement cả token bucket dấu phẩy động lẫn
   GCRA đằng sau một trait; assert chúng accept/reject giống hệt nhau trên
   một timeline request đã kịch bản sẵn kèm khoảng rảnh.
2. Viết test tích lũy-khi-rảnh: để một bucket không bị chạm trong 60 lần
   khoảng refill, rồi bắn một burst — xác nhận nhiều nhất `capacity`
   request được qua.
3. Đưa vào một timestamp chạy ngược và xác nhận implementation của bạn
   không rút cạn bucket cũng không cấp token miễn phí.
4. Làm GCRA lock-free với một `AtomicU64` + vòng lặp CAS; dồn nó từ 8 task
   và assert tổng số được chấp nhận không bao giờ vượt mức tối đa toán học
   cho thời gian tường đã trôi qua.
5. Cố tình tái hiện race đọc-rồi-ghi với một `RwLock`, quan sát việc
   over-admission, rồi sửa nó.
6. Lấp map theo từng IP bằng 1 triệu địa chỉ nguồn tổng hợp và đo bộ nhớ;
   thêm quét-khi-rảnh và xác nhận nó chững lại (plateau).
