# Time và Timer: Clock, Timeout, và Vì Sao Wall-Clock Time Không Đáng Tin

File này trả lời: "bây giờ" nghĩa là gì với một chương trình, kernel xử lý yêu cầu "đánh thức tôi
sau 30 giây" như thế nào, và những lỗi clock nào gây bug cho proxy: timeout không bao giờ nổ,
duration âm, token hết hạn trước khi được cấp. Đây là kiến thức nền cho mọi timeout trong
[`06-proxy/`](../06-proxy) và [`07-security/`](../07-security).

## What to learn

### Hai clock cho hai việc khác nhau
Linux có nhiều clock. Hai cái quan trọng:

- **`CLOCK_REALTIME`**: wall-clock time (số giây kể từ 1970-01-01 UTC). Nó trả lời câu hỏi "bây
  giờ là mấy giờ theo cách con người hiểu". Timestamp trong log, certificate và JWT đều dùng nó.
  Nó **có thể nhảy**:
  - NTP chỉnh nó. Sai lệch nhỏ thì chỉnh bằng **slew** (chạy nhanh hoặc chậm hơn một chút). Sai
    lệch lớn thì chỉnh bằng **step** (nhảy thẳng).
  - Admin có thể set tay.
  - Một VM khôi phục từ snapshot sẽ thức dậy với giờ chậm hàng giờ.
  - **Leap second** chèn thêm một giây.
- **`CLOCK_MONOTONIC`**: thời gian tính từ một điểm tùy ý (lúc boot). Nó chỉ chạy tiến, với tốc độ
  đều, và không bao giờ nhảy. Nó trả lời câu hỏi "đã *trôi qua* bao lâu". (`CLOCK_BOOTTIME` giống
  nó nhưng tiếp tục đếm cả khi máy suspend.)

Rust ánh xạ trực tiếp: `std::time::SystemTime` là realtime, `std::time::Instant` là monotonic. **Hãy
đo duration và implement timeout bằng `Instant`. Chỉ dùng `SystemTime` cho timestamp hiển thị cho
người, hoặc để so với wall-clock của máy khác.** `SystemTime::duration_since(earlier)` trả về `Err`
nếu clock đi lùi. Những panic thật trong production đến từ việc `.unwrap()` kết quả này.

```rust
let start = std::time::Instant::now();
do_work().await;
let elapsed = start.elapsed();            // không thể âm; không bị ảnh hưởng bởi NTP/step
```

### Khi nào wall-clock time có ý nghĩa đúng
Một số khái niệm *chính là* wall-clock: `notBefore`/`notAfter` của TLS certificate
([`01-network/19-tls.md`](../01-network/19-tls.md)), `exp`/`nbf` của JWT
([`07-security/02-jwt.md`](../07-security/02-jwt.md)), và `Date`, `Expires`, `Last-Modified` của HTTP
([`01-network/15-http.md`](../01-network/15-http.md)).

Các máy khác nhau lệch giờ nhau từ vài mili giây tới vài giây. Vì vậy bên verify phải cho phép
**clock skew** (một khoảng dung sai từ vài giây tới vài phút). Một token "được cấp trong tương lai"
vài giây là bình thường, không phải tấn công.

Một proxy host có clock trôi nhiều sẽ từ chối certificate và token hợp lệ, và tạo ra log có thứ tự
vô nghĩa. Hãy chạy một NTP client (`chronyd`, `systemd-timesyncd`) và đặt cảnh báo khi clock trôi.

### Chương trình chờ như thế nào: từ `sleep` đến `epoll_wait`
Chờ theo thời gian chỉ là block với một deadline. `nanosleep` cho một thread sleep đến một thời điểm.
Event loop làm khác: nó truyền **deadline kế tiếp** làm đối số *timeout* của `epoll_wait`
([`14-epoll.md`](14-epoll.md)). Nó sẽ tỉnh dậy khi một fd sẵn sàng *hoặc* khi timer sớm nhất đến hạn,
tùy cái nào đến trước.

Cách khác là dùng **`timerfd`**: nó phơi một kernel timer ra thành fd, fd này đọc được khi timer
expire. Nhờ đó timer nằm trong cùng tập `epoll` với socket ([`10-ipc.md`](10-ipc.md)).

Kernel cài các timer này trên **high-resolution timer** (`hrtimer`), về nguyên tắc có độ chính xác
dưới micro giây. Tuy vậy, thời điểm tỉnh dậy thực tế có scheduling jitter từ vài chục micro giây đến
vài mili giây khi tải cao ([`12-cpu-scheduling.md`](12-cpu-scheduling.md)). Một timer nghĩa là "không
sớm hơn lúc này", không bao giờ là "chính xác lúc này".

### tokio làm timer như thế nào
Một runtime có thể có **hàng chục nghìn timeout đang chờ** (mỗi connection, mỗi request một cái). Nhờ
kernel quản lý từng cái là lãng phí. Vì vậy tokio giữ một **timer wheel phân cấp** của riêng nó: một
mảng các bucket đánh index theo thời điểm hết hạn, cho phép insert và cancel trong O(1). Nó chỉ nhờ
kernel hẹn **một** lần chờ cho deadline sớm nhất
([`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md); wheel như một cấu trúc dữ liệu nằm ở
[`13-algorithms/priority-queue.md`](../13-algorithms/priority-queue.md)).

Độ phân giải khoảng 1 ms, nên `tokio::time::sleep(Duration::from_micros(10))` không cho bạn 10 µs.

`tokio::time::timeout(d, fut)` cho một future chạy đua với một timer. Khi timer thắng, future bị
**drop** (bị hủy). Vì vậy việc nó đang làm sẽ dừng lại ở `.await` kế tiếp
([`04-runtime/04-structured-concurrency.md`](../04-runtime/04-structured-concurrency.md)).

Trong test, `tokio::time::pause()` thay clock bằng một clock ảo tự tiến. Nhờ đó các test timeout 30
giây chạy tức thì và có kết quả tất định.

### Timeout là cơ chế an toàn trung tâm của proxy
Mọi lần chờ của proxy đều cần một timeout. Lý do: một peer không bao giờ trả lời sẽ giữ một
connection, một task và bộ nhớ mãi mãi ([`07-security/10-slowloris.md`](../07-security/10-slowloris.md)).
Các loại timeout cần có:

- **connect**: TCP handshake tới upstream.
- **TLS handshake**.
- **đọc header / đọc body**: idle timeout cho mỗi lần đọc, *và* một trần tổng.
- **upstream response**: time to first byte, rồi idle giữa các chunk.
- **idle keep-alive** ([`05-http-stack/05-keepalive.md`](../05-http-stack/05-keepalive.md)).
- **deadline tổng của request**.

Có hai điểm thiết kế quan trọng.

1. Phân biệt **idle timeout** (reset sau mỗi byte nhận được) với **total deadline** (trần cứng). Nếu
   chỉ có idle timeout, một client gửi dữ liệu nhỏ giọt (slow-drip) có thể giữ connection vô thời
   hạn.
2. **Truyền deadline đi tiếp.** Nếu client đã bỏ đi sau 5 giây, một lời gọi upstream mất 4 giây mà
   bắt đầu sau khi đã trôi qua 3 giây là lãng phí. Hãy đổi ngân sách thời gian còn lại thành timeout
   cho từng hop, và đừng retry vượt quá deadline ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)).

Timeout cũng lồng vào nhau theo kiểu khó chịu. Nếu upstream timeout của proxy dài hơn timeout của
client, proxy sẽ làm việc cho một client đã bỏ đi
([`01-network/21-life-of-a-request.md`](../01-network/21-life-of-a-request.md)).

### Đọc thời gian gần như miễn phí: vDSO
`clock_gettime` được gọi liên tục (mỗi dòng log, mỗi `Instant::now()`). Linux map một page nhỏ do
kernel cung cấp, gọi là **vDSO**, vào mỗi process. Nhờ đó việc đọc clock **không cần syscall** và chỉ
tốn vài chục nano giây.

(Một số clock source ảo hóa, ví dụ vài thiết lập Xen, tắt tính năng này và biến `Instant::now()` thành
syscall thật. Nếu `strace` hiện `clock_gettime` liên tục, đó là dấu hiệu.)

Dù vậy, đọc thời gian cho từng byte hay từng event nhỏ vẫn là lãng phí. Hãy đọc một lần cho mỗi batch.

### Mọi thứ đều hết hạn: TTL và cache
Cache entry, DNS answer, cửa sổ rate-limit, cooldown của circuit-breaker, health-check interval và việc
dọn connection idle của pool đều có hạn. Hãy lưu **`Instant` hết hạn** (monotonic), không dùng
`SystemTime`, để một lần clock step không làm entry sống mãi hoặc chết ngay lập tức.

Nên ưu tiên *expire lười khi truy cập, cộng một lần quét định kỳ*, hơn là một timer cho mỗi entry
([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md),
[`06-proxy/06-circuit-breaker.md`](../06-proxy/06-circuit-breaker.md)).

Hãy **thêm jitter** vào TTL (ngẫu nhiên hóa ±10%). Nhờ đó các entry được tạo cùng lúc sẽ không hết hạn
cùng lúc rồi dồn một đợt request vào upstream
([`05-http-stack/09-cache-stampede.md`](../05-http-stack/09-cache-stampede.md)).

### Gotcha: sleep cả thread trong code async
`std::thread::sleep` trong một async task sẽ block *cả worker thread*, vì nó không nhường lại ở
`.await`. Mọi task khác trên thread đó bị đói tài nguyên, và triệu chứng là timeout nổ trễ ở khắp nơi.
Hãy dùng `tokio::time::sleep`. Điều tương tự áp dụng cho mọi lời gọi blocking; xem
[`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md).

## Practice

1. In `date +%s`, `cat /proc/uptime` và `chronyc tracking` (hoặc `timedatectl timesync-status`). Giải
   thích cái nào là realtime, cái nào là monotonic. Viết một chương trình Rust scratch mỗi giây in
   delta của `SystemTime::now()` và `Instant::now()`. Trong lúc nó chạy, gõ `sudo date -s '-1 hour'`.
   Cho thấy clock nào nhảy và `duration_since` trả về gì.
2. Chạy `strace -f -c` trên một vòng lặp nhỏ gọi `Instant::now()` 10 triệu lần và xác nhận
   `clock_gettime` không xuất hiện (nhờ vDSO). Sau đó đo thời gian của vòng lặp để tính ra số ns cho
   mỗi lời gọi.
3. Viết một chương trình tokio scratch với 100.000 task `sleep(10s)` và đo CPU, RSS của nó. Sau đó
   chạy `strace -f -c` và xác nhận số syscall rất ít, nghĩa là timer wheel đã gộp chúng lại.
4. Tái hiện bug blocking-sleep. Spawn nhiều task `std::thread::sleep(50ms)` rồi so với
   `tokio::time::sleep(50ms)` trên runtime chỉ có một worker (`flavor = "current_thread"`), và đo các
   timer khác bị nổ trễ bao nhiêu.
5. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), thêm timeout cho connect,
   upstream-response và total-request. Kiểm chứng từng cái bằng một upstream chậm có chủ đích (một
   `nc -l` không bao giờ reply, hoặc một handler `sleep`): status đúng (`504`,
   [`01-network/15-http.md`](../01-network/15-http.md)) được trả về và connection được đóng. Với một
   client slow-drip, kiểm chứng rằng chỉ có total deadline mới dừng được nó.
6. Dùng `tokio::time::pause()`/`advance` để unit-test việc hết hạn của một TTL cache và một retry
   backoff mà không phải sleep thật.

---

<!-- nav -->
← Trước: [`02-linux/10-ipc.md`](10-ipc.md) · [Mục lục](00-README.md) · Tiếp theo: [`02-linux/12-cpu-scheduling.md`](12-cpu-scheduling.md) →
