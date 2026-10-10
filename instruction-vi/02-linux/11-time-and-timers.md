# Time và Timer: Clock, Timeout, và Vì Sao Wall-Clock Time Nói Dối

"Bây giờ" nghĩa là gì với một chương trình, kernel giao "đánh thức tôi sau 30 giây" ra sao, và các
lỗi clock gây bug proxy: timeout không bao giờ nổ, duration âm, token hết hạn trước khi được cấp.
Là kiến thức tiên quyết cho mọi timeout trong [`06-proxy/`](../06-proxy) và [`07-security/`](../07-security).

## What to learn

### Hai clock, hai việc khác nhau
Linux cung cấp nhiều clock; hai cái quan trọng:

- **`CLOCK_REALTIME`** — wall-clock time (số giây kể từ 1970-01-01 UTC). Nó trả lời "giờ là mấy với
  con người," là thứ mà timestamp trong log, certificate và JWT có nghĩa, và nó **có thể nhảy**: NTP
  chỉnh nó (sai số nhỏ bằng cách **slew** — chạy nhanh hoặc chậm hơn chút; sai số lớn bằng cách
  **step**), admin có thể set nó, một VM khôi phục từ snapshot thức dậy chậm hàng giờ, và **leap
  second** chèn thêm một giây.
- **`CLOCK_MONOTONIC`** — thời gian kể từ một điểm tùy ý (boot), chỉ chạy tiến với tốc độ đều và không
  bao giờ nhảy. Nó trả lời "đã *trôi qua* bao nhiêu thời gian." (`CLOCK_BOOTTIME` giống nhưng tiếp tục
  đếm xuyên qua suspend.)

Rust ánh xạ trực tiếp: `std::time::SystemTime` là realtime, `std::time::Instant` là monotonic. **Đo
duration và implement timeout bằng `Instant`; chỉ dùng `SystemTime` cho timestamp hiển thị cho người
hoặc so với wall-clock của máy khác.** `SystemTime::duration_since(earlier)` trả `Err` nếu clock đi
lùi — các panic production thật đến từ `.unwrap()` trên nó.

```rust
let start = std::time::Instant::now();
do_work().await;
let elapsed = start.elapsed();            // không thể âm; miễn nhiễm với NTP/step
```

### Nơi wall-clock time có ý nghĩa chính đáng
Một số ý nghĩa *chính là* wall-clock: `notBefore`/`notAfter` của TLS certificate
([`01-network/19-tls.md`](../01-network/19-tls.md)), `exp`/`nbf` của JWT ([`07-security/02-jwt.md`](../07-security/02-jwt.md)), `Date`, `Expires` và
`Last-Modified` của HTTP ([`01-network/15-http.md`](../01-network/15-http.md)). Các máy khác nhau lệch nhau từ mili giây tới
giây, nên bên verify cho phép **clock skew** (độ nới vài giây–vài phút) — một token "cấp trong tương
lai" vài giây là bình thường, không phải tấn công. Một proxy host có clock trôi nặng sẽ từ chối
certificate và token hợp lệ và tạo ra thứ tự log vô nghĩa; hãy chạy một NTP client (`chronyd`,
`systemd-timesyncd`) và cảnh báo khi trôi.

### Một chương trình chờ như thế nào: từ `sleep` đến `epoll_wait`
Chờ thời gian chỉ là blocking với một deadline. `nanosleep` đỗ một thread cho tới một thời điểm. Một
event loop thay vào đó truyền **deadline kế tiếp** làm đối số *timeout* của `epoll_wait`
([`14-epoll.md`](14-epoll.md)): nó thức dậy khi một fd sẵn sàng *hoặc* khi timer sớm nhất đến hạn, cái nào trước.
Hoặc **`timerfd`** phơi một kernel timer thành fd trở nên đọc được khi hết hạn, để timer nhập vào
cùng tập `epoll` với socket ([`10-ipc.md`](10-ipc.md)). Kernel cài các cái này trên **high-resolution timer**
(`hrtimer`), về nguyên tắc dưới micro giây, dù các lần thức dậy thật có scheduling jitter hàng chục
micro giây tới mili giây dưới tải ([`12-cpu-scheduling.md`](12-cpu-scheduling.md)); một timer nghĩa là "không sớm hơn," không bao giờ
là "chính xác lúc."

### tokio làm timer như thế nào
Một runtime có thể có **hàng chục nghìn timeout đang chờ** (mỗi connection, mỗi request một cái). Xin
kernel cho từng cái sẽ lãng phí, nên tokio giữ **timer wheel phân cấp** của riêng nó — một mảng bucket
đánh index theo thời điểm hết hạn cho insert và cancel O(1) — và arm **một** lần chờ kernel cho deadline
sớm nhất ([`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md); wheel như một cấu trúc dữ liệu nằm ở
[`13-algorithms/priority-queue.md`](../13-algorithms/priority-queue.md)). Độ phân giải ~1 ms; `tokio::time::sleep(Duration::from_micros(10))` không cho
10 µs. `tokio::time::timeout(d, fut)` cho một future chạy đua với một timer; khi timer thắng, future bị
**drop** (hủy) — nên bất cứ việc gì nó đang làm dừng ở `.await` kế tiếp
([`04-runtime/04-structured-concurrency.md`](../04-runtime/04-structured-concurrency.md)). Trong test, `tokio::time::pause()` thay clock bằng một
clock ảo tự tiến, làm các test timeout 30 giây tức thì và tất định.

### Timeout là cơ chế an toàn trung tâm của proxy
Mọi lần chờ mà proxy thực hiện đều cần một timeout, vì một peer không bao giờ trả lời sẽ giữ một
connection, một task và bộ nhớ mãi mãi ([`07-security/10-slowloris.md`](../07-security/10-slowloris.md)):

- **connect** (TCP handshake tới upstream),
- **TLS handshake**,
- **đọc header / đọc body** (idle trên mỗi lần đọc, *và* một trần tổng),
- **upstream response** (time to first byte, rồi idle giữa các chunk),
- **idle keep-alive** ([`05-http-stack/05-keepalive.md`](../05-http-stack/05-keepalive.md)),
- **deadline tổng của request**.

Hai thiết kế quan trọng. Phân biệt **idle timeout** (reset mỗi byte) với **total deadline** (trần cứng):
chỉ idle thì client nhỏ giọt có thể giữ một connection vô thời hạn. Và **truyền deadline đi**: nếu client
đã bỏ sau 5 s, một lời gọi upstream có thể xong trong 4 s *sau khi* đã trôi qua 3 s là phí; hãy chuyển
ngân sách còn lại thành timeout của từng hop, và đừng retry quá deadline ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)). Timeout
cũng lồng nhau khó chịu: nếu upstream timeout của proxy vượt timeout của client, proxy làm việc cho một
client đã bỏ đi ([`01-network/21-life-of-a-request.md`](../01-network/21-life-of-a-request.md)).

### Đọc thời gian gần như miễn phí: vDSO
`clock_gettime` được gọi liên tục (mỗi dòng log, mỗi `Instant::now()`). Linux map một page nhỏ do kernel
cung cấp, **vDSO**, vào mỗi process để lời gọi đọc clock **không cần syscall** — hàng chục nano giây.
(Một số clock source ảo hóa, ví dụ vài thiết lập Xen, tắt điều này và biến `Instant::now()` thành syscall
thật; `strace` hiện `clock_gettime` vô tận là dấu hiệu.) Dù vậy, đọc thời gian mỗi byte hay mỗi sự kiện
tí hon là phí: hãy đọc một lần mỗi batch.

### Hết hạn mọi thứ: TTL và cache
Cache entry, DNS answer, cửa sổ rate-limit, cooldown circuit-breaker, health-check interval và việc dọn
idle của pool đều hết hạn. Hãy lưu **`Instant` hết hạn** (monotonic), không phải `SystemTime`, để một clock
step không làm một entry sống mãi hay chết ngay, và ưu tiên *hết hạn lười khi truy cập cộng một lần quét
định kỳ* hơn là một timer cho mỗi entry ([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md),
[`06-proxy/06-circuit-breaker.md`](../06-proxy/06-circuit-breaker.md)). **Jitter** TTL (ngẫu nhiên hóa ±10%) để các entry tạo cùng lúc không hết
hạn cùng lúc và dồn dập đổ vào upstream ([`05-http-stack/09-cache-stampede.md`](../05-http-stack/09-cache-stampede.md)).

### Gotcha: ngủ một thread trong code async
`std::thread::sleep` trong một async task block *cả worker thread* (không yield ở `.await`), bỏ đói mọi
task khác trên nó — và triệu chứng là timeout nổ trễ ở khắp nơi. Dùng `tokio::time::sleep`. Điều tương tự
áp dụng cho mọi lời gọi blocking; xem [`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md).

## Practice

1. In `date +%s`, `cat /proc/uptime`, và `chronyc tracking` (hoặc `timedatectl timesync-status`); giải thích
   cái nào là realtime và cái nào monotonic. Viết một chương trình Rust scratch in delta của `SystemTime::now()`
   và `Instant::now()` mỗi giây, rồi `sudo date -s '-1 hour'` khi nó chạy và cho thấy clock nào nhảy và
   `duration_since` trả về gì.
2. `strace -f -c` một vòng lặp nhỏ gọi `Instant::now()` 10 triệu lần và xác nhận `clock_gettime` không xuất
   hiện (vDSO); rồi đo thời gian vòng lặp để ra ns mỗi lời gọi.
3. Viết một chương trình tokio scratch với 100.000 task `sleep(10s)` và đo CPU và RSS của nó; rồi chạy
   `strace -f -c` và xác nhận số syscall rất nhỏ, tức timer wheel ghép chúng lại.
4. Tái hiện bug blocking-sleep: spawn nhiều task `std::thread::sleep(50ms)` vs `tokio::time::sleep(50ms)` trên
   một runtime một worker (`flavor = "current_thread"`) và đo các timer khác nổ trễ bao nhiêu.
5. Trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), thêm timeout connect, upstream-response và total-request, rồi kiểm chứng
   từng cái bằng một upstream chậm có chủ đích (một `nc -l` không bao giờ reply, hoặc một handler `sleep`) rằng
   status đúng (`504`, [`01-network/15-http.md`](../01-network/15-http.md)) được trả và connection được đóng, và với một client nhỏ giọt
   rằng chỉ một total deadline mới dừng được nó.
6. Dùng `tokio::time::pause()`/`advance` để unit-test việc hết hạn của một TTL cache và một retry backoff mà
   không ngủ thật.
