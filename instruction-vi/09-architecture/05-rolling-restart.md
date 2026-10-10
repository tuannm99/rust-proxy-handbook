# Rolling Restart Không cần Orchestrator

Nâng cấp binary zero-downtime trên một host duy nhất, không có rolling
deploy được quản lý bởi Kubernetes/systemd để dựa vào.

## What to learn
### Vì sao đây là một vấn đề khác với graceful shutdown
[`09-architecture/04-graceful-shutdown.md`](04-graceful-shutdown.md) bao quát việc dừng *một* process
sạch sẽ. Một rolling restart cần một process **mới** (binary mới, config
mới) tiếp quản listening port trước khi cái cũ biến mất — với zero
khoảng trống nơi một lượt thử connection của client bị từ chối. Trên một
host duy nhất không có orchestrator để dựng một instance thứ hai đứng sau
một load balancer, chính proxy phải làm cho việc chuyển giao này an toàn.

### SO_REUSEPORT: dual-accept trong cửa sổ chồng lấn
`SO_REUSEPORT` cho phép nhiều process bind *cùng* address:port đồng thời;
kernel load-balance các connection mới trên tất cả chúng. Khởi động process
mới với `SO_REUSEPORT` được đặt, để nó bind cạnh process cũ vẫn đang
chạy, xác nhận cái mới khỏe mạnh, rồi gửi cho cái cũ `SIGTERM` (kích hoạt
drain bình thường của nó từ [`04-graceful-shutdown.md`](04-graceful-shutdown.md)). Trong khoảng chồng
lấn ngắn, cả hai process accept connection mới — không có cửa sổ nào port bị
unbind.

```rust
use socket2::{Domain, Socket, Type};
use std::net::SocketAddr;

fn bind_reuseport(addr: SocketAddr) -> std::io::Result<std::net::TcpListener> {
    let socket = Socket::new(Domain::for_address(addr), Type::STREAM, None)?;
    socket.set_reuse_address(true)?;
    socket.set_reuse_port(true)?; // the key difference from a normal bind
    socket.bind(&addr.into())?;
    socket.listen(1024)?;
    Ok(socket.into())
}
```
Gotcha: `SO_REUSEPORT` phân phối các connection *mới* trên tất cả process đã
bind gần như ngẫu nhiên — trong cửa sổ chồng lấn, process cũ (đang drain)
vẫn có thể nhận connection hoàn toàn mới trừ khi nó cũng đang từ chối chúng ở
lớp ứng dụng, điều này phần nào phá hỏng mục đích. Giữ cửa sổ chồng lấn
ngắn.

Gotcha, và đây là cái âm thầm tốn của bạn vài connection: kernel gán một connection đến cho một listener **tại thời điểm SYN**, bằng cách hash 4-tuple, và
nó rơi vào accept queue của *listener đó*. Khi process cũ đóng socket
listening của nó, các connection đang nằm trong accept queue của nó — các
handshake đã hoàn thành mà client tin là đã thiết lập — bị reset, không
được phân phối lại. Vậy nên process đang drain phải tiếp tục gọi
`accept()` và phục vụ những gì đã queue sẵn trong một lúc sau khi nó
ngừng là mục tiêu ưu tiên, thay vì đóng listener ngay khi nó quyết định
drain. Đây là cùng bài học "ngừng-accept-không-miễn-phí" như pre-stop
delay trong [`09-architecture/04-graceful-shutdown.md`](04-graceful-shutdown.md), ở một tầng thấp
hơn.

### Chuyển giao socket qua `exec` (kiểu nginx)
Cách thay thế nginx dùng cho nâng cấp binary kích hoạt bởi `SIGUSR2`:
master process cũ giữ file descriptor listening của nó mở, spawn binary
mới và truyền fd đó xuống nó (thừa kế qua `exec`, hoặc trao qua một Unix
domain socket bằng `SCM_RIGHTS`), và process mới bắt đầu accept trên
*cùng* socket đó — không phải một cái thứ hai. Không có cửa sổ chồng lấn
và hoàn toàn không dựa vào `SO_REUSEPORT`; chỉ một process sở hữu fd tại
một thời điểm, nhưng nó đổi chủ mà không bao giờ đóng lại.

Gotcha: "thừa kế qua `exec`" đòi hỏi xóa `FD_CLOEXEC` trên descriptor đó
— Rust đặt close-on-exec trên các socket nó tạo theo mặc định, và quên
xóa nó tạo ra một process mới khởi động, không thấy listener thừa kế nào,
và hoặc thoát hoặc bind một socket mới với một khoảng trống. Truyền số fd
cho process con một cách tường minh (một biến môi trường là kênh
thường dùng) thay vì giả định một quy ước.

Gotcha: phiên bản đầy đủ của nginx cho điều này giữ master cũ sống vô thời
hạn để một lần nâng cấp thất bại có thể rollback bằng cách tín hiệu cho
cái mới thoát và cái cũ tiếp tục accept. Đường rollback đó là lý do thật
sự khiến pattern này phức tạp hơn `SO_REUSEPORT` — nếu bạn không cài đặt
rollback, bạn đã trả chi phí phức tạp mà không có lợi ích.

### Systemd socket activation
Nếu systemd có sẵn (ngay cả không có orchestrator/Kubernetes đầy đủ), một
unit `.socket` có thể sở hữu socket listening độc lập với unit
`.service`. Systemd mở và giữ socket; `systemctl restart` trên service
chỉ restart process, và kernel queue các connection đến trong socket
backlog trong vài trăm mili giây process bị down — không cần code
`SO_REUSEPORT` hay fd-passing nào trong proxy cả, với chi phí là phụ
thuộc vào việc systemd có mặt.

Gotcha: điều này hoạt động vì accept queue của kernel hấp thụ khoảng
trống — nên nó chỉ hoạt động nếu khoảng trống ngắn hơn thời gian để queue
đầy. Ở connection rate cao, một process khởi động chậm (cert TLS cần
load, config cần validate, cache cần xây) làm tràn backlog và các connection
vẫn bị từ chối dù sao. Đo thời gian từ khởi động tới accepting của bạn và
so sánh nó với connection rate của bạn nhân độ sâu backlog
([`16-kernel/03-tcp-stack.md`](../16-kernel/03-tcp-stack.md)) trước khi tin tưởng nó.

### Điều gì phải luôn đúng bất kể kỹ thuật nào
Process mới phải qua được kiểm tra readiness của chính nó (config đã
parse, upstream tới được — gắn với [`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md)) *trước
khi* cái cũ được tín hiệu để drain, nếu không một binary/config mới tồi sẽ
làm sập cả proxy thay vì chỉ fail deploy. Các connection sống lâu (WebSocket,
[`05-http-stack/10-websocket.md`](../05-http-stack/10-websocket.md)) được giữ bởi process cũ cần cùng drain
deadline như [`04-graceful-shutdown.md`](04-graceful-shutdown.md) — một rolling restart không làm
vấn đề đó biến mất, nó chỉ thêm "và đừng từ chối connection mới trong khi
drain."

### Restart làm mất state, và state đó có ý nghĩa
Zero *connection bị rớt* không giống zero tác động, vì mọi thứ process cũ đã
tích lũy trong bộ nhớ đều biến mất. Mỗi cái dưới đây được bao quát ở nơi
khác; cùng nhau chúng là lý do một lần restart zero-downtime "thành công"
vẫn có thể xuất hiện như một cú tăng vọt trên mọi dashboard:
- **Response cache rỗng** ([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md)). Mọi entry là một
  miss, tất cả cùng lúc — một cache stampede tự gây ra nhắm vào origin
  đúng lúc bạn muốn mọi thứ yên tĩnh. Request coalescing là thứ giữ điều
  này sống sót được.
- **Rate limiter bucket reset** ([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)). Mọi
  client âm thầm nhận một budget mới; một client bạn đang chủ động
  throttle giờ hết bị throttle. Một kẻ tấn công có thể kích hoạt restart
  nhận một lần reset giới hạn theo yêu cầu.
- **Circuit breaker reset** ([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)). Process mới không
  biết một upstream đã hỏng và sẽ gửi traffic vào nó để tìm hiểu, học lại
  với chi phí là các request thật.
- **Connection pool nguội** ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)). Các request đầu
  tiên trả latency handshake, bao gồm TLS, nên p99 tăng vọt trong hàng
  chục giây sau lần chuyển giao.
- **Health state không rõ.** Cho tới khi chu kỳ probe đầu tiên hoàn
  thành, process mới hoặc coi mọi upstream là healthy (và gửi traffic tới
  các upstream đã chết) hoặc unhealthy (và không phục vụ gì cả) — quyết
  định cái nào, và ưu tiên làm readiness chờ đủ một chu kỳ probe đầy đủ.

Gotcha: cách sửa cho hầu hết những cái này là làm cho readiness nghĩa là
*ấm*, không chỉ *đã khởi động* — hoàn thành một chu kỳ health-check và
mở trước vài connection trong pool trước khi công bố sẵn sàng. Cache là
ngoại lệ; làm nóng nó nói chung không đáng, nhưng biết trước cú tăng miss
sắp tới nghĩa là không nhầm nó với một regression.

### Xác minh một cách trung thực
Một load test chỉ báo cáo status code HTTP sẽ vui vẻ tuyên bố thành công
trong khi các connection đang bị reset, vì một connection bị reset thường không
tạo ra status code nào cả — request đơn giản biến mất khỏi tập kết quả,
hoặc được đếm vào một hạng mục không ai đọc. Đo ở cấp connection: đếm lỗi
connection, reset, và từ chối riêng biệt với các response không-2xx, và
khẳng định cả ba đều bằng không xuyên suốt quá trình chuyển giao.

## Practice
Xây theo thứ tự.

1. Cài đặt helper bind `SO_REUSEPORT` trong [`proxy`](../../proxy) và chạy hai instance
   trên một port. **Xong khi** log theo-từng-instance cho thấy kernel
   phân phối connection mới trên cả hai.
2. Thêm một kiểm tra readiness process mới phải vượt qua — config đã
   parse, cert đã load, một chu kỳ health-check đầy đủ hoàn thành, vài
   connection upstream trong pool đã mở. **Xong khi** một process với config
   hỏng hoặc upstream không tới được không bao giờ báo cáo sẵn sàng.
3. Viết script restart: khởi động cái mới, chờ readiness, `SIGTERM` cái
   cũ (tái sử dụng drain từ [`09-architecture/04-graceful-shutdown.md`](04-graceful-shutdown.md)),
   xác nhận nó thoát sau khi drain. **Xong khi** một binary mới tồi để
   lại cái cũ vẫn phục vụ, không bị đụng tới.
4. Làm cho process đang drain tiếp tục accept các connection đã queue sẵn
   trước khi đóng listener. **Xong khi** một load test ở connection rate
   cao xuyên qua quá trình chuyển giao cho thấy zero reset — chạy nó
   không có bước này trước và đếm chúng, vì chúng vô hình trừ khi bạn tìm.
5. Load test xuyên qua một lần restart, đo lỗi ở cấp connection riêng biệt
   ([`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)). **Xong khi** connection refusal,
   reset, và non-2xx đều bằng không xuyên suốt quá trình chuyển giao.
6. Đo chi phí mất state. **Xong khi** bạn có một biểu đồ tỷ lệ request tới
   origin (cache miss), p99 latency (pool nguội), và tỷ lệ lỗi upstream
   (circuit breaker reset) xuyên qua một lần restart — và đã quyết định
   cái nào bạn sẽ giảm nhẹ.
7. (Stretch) Cài đặt chuyển giao fd qua `SCM_RIGHTS` hoặc thừa kế `exec`,
   bao gồm xóa `FD_CLOEXEC`. **Xong khi** process mới accept trên đúng
   socket đó với không cửa sổ chồng lấn nào — xác minh bằng `ss -tlnp`
   rằng chỉ một process sở hữu listener tại bất kỳ thời điểm nào.
8. (Stretch) Thêm đường rollback: nếu process mới fail readiness sau khi
   tiếp quản, tín hiệu nó thoát và để cái cũ tiếp tục. **Xong khi** một
   lần nâng cấp cố ý hỏng tự động rollback với zero connection bị rớt.
9. Tùy chọn: viết một cặp `.socket` + `.service` của systemd và so sánh.
   **Xong khi** `systemctl restart` đạt cùng kết quả zero-dropped mà
   không cần bất kỳ code nào ở trên — và bạn đã đo thời gian khởi động
   của mình so với độ sâu backlog để biết giới hạn của cách tiếp cận đó.
