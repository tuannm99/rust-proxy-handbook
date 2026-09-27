# Graceful Shutdown

## What to learn
### SIGTERM vs SIGKILL, và vì sao proxy phải tự xử lý SIGTERM
Các orchestrator (systemd, Kubernetes) gửi `SIGTERM` trước và cho process
một khoảng thời gian ân hạn trước `SIGKILL` (thứ không thể bắt được — nó
là một dừng cứng tức thì). Nếu proxy không bắt `SIGTERM` và hành động theo
nó, nó hoặc chết ngay lập tức giữa chừng request (kết nối bị rớt) hoặc bị
kill cứng sau khi hết thời gian ân hạn, cùng một kết quả. Xem
[`02-linux/10-signals.md`](../02-linux/10-signals.md).

Gotcha: là PID 1 trong một container, các disposition tín hiệu mặc định
không áp dụng — kernel không kill PID 1 với các tín hiệu nó chưa xử lý
tường minh. Một proxy *không* cài một handler `SIGTERM` và chạy như PID 1
do đó bỏ qua `docker stop` hoàn toàn và chờ hết toàn bộ thời gian ân hạn
trước khi bị kill. Mọi lần shutdown khi đó mất thời gian tối đa, thứ
người ta thường chẩn đoán là "shutdown chậm" thay vì "shutdown chưa bao
giờ bắt đầu."

Gotcha: cũng xử lý `SIGINT` (Ctrl-C khi phát triển cục bộ), và làm cho một
tín hiệu *thứ hai* ép thoát ngay lập tức. Một operator gửi `SIGTERM` và
không thấy gì xảy ra trong 30 giây sẽ leo thang; cho họ một "bấm lần nữa
để ngừng chờ" đã tài liệu hóa tốt hơn là để họ dùng `SIGKILL` và học được
rằng nó hiệu quả.

### Chuỗi drain
Khi `SIGTERM`: (1) ngừng nhận kết nối/request *mới* ngay lập tức — hủy
đăng ký khỏi bất kỳ load balancer/service discovery nào trước nếu có thể,
để traffic upstream ngừng tới trước cả khi bạn đóng listener; (2) để các
request in-flight hoàn thành bình thường; (3) sau một deadline có giới
hạn, cưỡng chế hủy bất cứ thứ gì còn chạy và thoát. Làm sai bước 1 và 3 là
nguyên nhân phổ biến nhất của một shutdown "graceful" mà vẫn gây lỗi thấy
được ở client.

```rust
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;
use std::time::Duration;

let (shutdown_tx, _) = broadcast::channel::<()>(1);
let mut sigterm = signal(SignalKind::terminate())?;

tokio::select! {
    _ = sigterm.recv() => {
        tracing::info!("SIGTERM received, draining");
        let _ = shutdown_tx.send(()); // tell all connection tasks to stop accepting new work
        tokio::time::timeout(Duration::from_secs(30), wait_for_all_connections_to_finish()).await.ok();
    }
}
```

### Tiếp tục accept một lúc sau SIGTERM — phần phản trực giác
Chuỗi ở trên vẫn rớt traffic trong Kubernetes, và lý do đáng để khắc cốt
ghi tâm vì nó đánh bại mọi cài đặt nếu không thì đúng.

Việc chấm dứt pod và xóa endpoint xảy ra **đồng thời, không theo thứ
tự.** kubelet gửi `SIGTERM` cùng lúc control plane bắt đầu lan truyền
việc xóa bạn tới mọi rule kube-proxy/ipvs trên mỗi node và tới danh sách
endpoint của mỗi ingress controller. Việc lan truyền đó mất từ vài trăm
mili giây tới vài giây. Vậy nên trong một cửa sổ *sau khi* bạn nhận
`SIGTERM`, các load balancer vẫn đang gửi cho bạn kết nối mới — và nếu bạn
đóng listener ngay khi tín hiệu tới, mỗi kết nối đó là một connection
refused.

Cách sửa là một **pre-stop delay** có chủ đích: khi `SIGTERM`, tiếp tục
accept và phục vụ bình thường trong vài giây (5-15 là điển hình), *rồi*
mới bắt đầu drain. Kubernetes cũng cung cấp một hook `preStop`
(`sleep 10`) chạy trước khi `SIGTERM` được gửi, đạt được cùng hiệu quả
ngoài code của bạn. Dù theo cách nào đây cũng là một khoảng chờ tường
minh, đã cấu hình — không phải thứ xảy ra mặc định.

```
SIGTERM ──► [ keep serving, ~5-15s ]  ──► stop accepting ──► drain in-flight ──► exit
            (LBs learn you're gone)       (close listener)    (bounded deadline)
```

Gotcha: điều này tương tác với service discovery của chính bạn
([`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md)). Nếu proxy tự đăng ký, hủy đăng ký
chủ động như bước drain đầu tiên rút ngắn cửa sổ đó rất nhiều — nhưng nó
không bao giờ loại bỏ hoàn toàn, vì các thành phần khác vẫn cache
membership cũ. Giữ khoảng chờ ngay cả khi bạn hủy đăng ký chủ động.

### Shutdown listener vs shutdown kết nối
Đóng socket listening ngừng các kết nối TCP *mới* nhưng không làm gì với
các kết nối keep-alive đã mở có thể vẫn gửi thêm request. Mỗi connection
handler cần tín hiệu riêng của nó (ví dụ một `broadcast::Receiver` được
clone theo từng task) để biết "hoàn thành request hiện tại, rồi trả
`Connection: close` và ngừng đọc thêm request trên socket này thay vì chờ
request tiếp theo."

Với HTTP/2, cái tương đương là `GOAWAY`, và nó tốt hơn `Connection: close`
theo một cách cụ thể: nó nêu tên stream ID cao nhất mà server sẽ xử lý,
nên một client với các request in-flight biết chính xác cái nào đã được
chấp nhận và cái nào nó phải retry ở nơi khác. Dạng graceful là hai frame
`GOAWAY` — một với stream ID tối đa để công bố ý định (để các stream
in-flight hoàn thành trong khi client ngừng mở stream mới), rồi một cái
cuối với ID đã-xử-lý-cuối-cùng thật ([`01-network/11-http2.md`](../01-network/11-http2.md)).

Gotcha: một kết nối keep-alive đang idle là cùng race như xung đột
close/request của [`05-http-stack/04-keepalive.md`](../05-http-stack/04-keepalive.md), giờ xảy ra trên cả
bảng kết nối của bạn cùng lúc. Công bố (`Connection: close` / `GOAWAY`)
trước khi đóng là thứ biến "client thấy một reset" thành "client mở một
kết nối mới ở nơi khác."

### Kết nối sống lâu cần một chính sách khác
"Để các request in-flight hoàn thành" giả định các request có hoàn thành.
Một WebSocket ([`05-http-stack/09-websocket.md`](../05-http-stack/09-websocket.md)), một lệnh gọi gRPC
server-streaming ([`05-http-stack/10-grpc.md`](../05-http-stack/10-grpc.md)), hoặc một stream SSE có thể
cách hoàn thành hàng phút hoặc hàng giờ, và chờ chúng nghĩa là không bao
giờ shutdown.

Chúng cần một chính sách tường minh, quyết định theo từng loại kết nối:
gửi một WebSocket close frame với mã "going away" (1001) để client
reconnect sạch sẽ, hoặc kết thúc một stream với một gRPC status
retry-được (`UNAVAILABLE`), thay vì để deadline ép một TCP reset trần
trụi. Một close sạch ở cấp protocol để client reconnect tới một instance
khác ngay lập tức; một reset khiến chúng retry một cách mù quáng và
thường chậm hơn.

### Deadline và cưỡng chế hủy
Luôn giới hạn drain bằng một timeout (`tokio::time::timeout`). Một kết nối
upstream bị kẹt duy nhất (TCP treo, client kiểu slowloris) không được
chặn shutdown mãi mãi — sau deadline, hủy các task còn lại và thoát dù
sao, log những request nào bị cưỡng chế hủy để nó hiển thị trong
[`08-observability/01-logging.md`](../08-observability/01-logging.md), không im lặng.

Gotcha: tổng của pre-stop delay cộng drain deadline của bạn phải **ít
hơn** grace period của orchestrator (`terminationGracePeriodSeconds`, mặc
định 30s trong Kubernetes), nếu không `SIGKILL` sẽ tới giữa lúc drain và
bạn nhận được đúng cái shutdown-không-graceful mà bạn viết tất cả code
này để tránh. Ghi phép tính đó xuống cạnh cả hai thiết lập; đó là hai con
số ở hai repository khác nhau và chúng *sẽ* lệch nhau.

### Còn gì khác phải flush trước khi thoát
Shutdown không chỉ là kết nối. Bất cứ thứ gì được buffer nhân danh hiệu
năng là dữ liệu chưa flush lúc thoát:
- **Buffer log và trace** — writer non-blocking của `tracing_appender` và
  OTLP batch exporter ([`08-observability/03-tracing.md`](../08-observability/03-tracing.md)) đều giữ bản ghi
  trong bộ nhớ. Mất đúng các bản ghi từ cửa sổ shutdown là mất bằng chứng
  cho bất cứ điều gì đã gây ra shutdown.
- **Metrics** — một lần scrape cuối sẽ không xảy ra, nên bất kỳ chuyển
  động counter nào từ lần scrape trước sẽ mất. Đây là một khoảng trống đã
  biết, được chấp nhận với metrics kiểu pull; biết nó tồn tại trước khi
  kết luận một lần deploy gây ra một cú rớt về không.
- **Kết nối upstream** — đóng các kết nối idle trong pool tường minh
  ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)) thay vì để process thoát làm rớt chúng, để
  upstream thấy các lần đóng sạch thay vì reset.

Gotcha: thứ tự flush quan trọng — flush telemetry *cuối cùng*, sau khi
drain hoàn thành, để các event của chính shutdown được bao gồm.

## Practice
Xây theo thứ tự.

1. Đăng ký handler `SIGTERM` và `SIGINT` trong [`proxy`](../../proxy), với một tín hiệu
   thứ hai ép thoát ngay lập tức. **Xong khi** `docker stop` kích hoạt
   dòng log drain — xác minh trong khi chạy như PID 1, vì đó là chỗ cái
   bẫy default-disposition sống.
2. Cài đặt broadcast shutdown: dừng accept loop, rồi báo cho các task
   theo-từng-kết-nối ngừng tái sử dụng keep-alive và gửi
   `Connection: close` / `GOAWAY`. **Xong khi** một client keep-alive
   đang idle được báo dừng thay vì phát hiện qua một reset.
3. Thêm drain deadline có giới hạn với log các request bị cưỡng chế hủy.
   **Xong khi** một upstream cố ý treo không ngăn cản việc thoát, và các
   request bị hủy được nêu tên trong log.
4. Load test trong khi gửi `SIGTERM` giữa test
   ([`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)). **Xong khi** các request in-flight
   thấy zero connection reset. Vẫn kỳ vọng thấy lỗi từ các request *mới
   đến* ở giai đoạn này — đó là bước 5.
5. Thêm pre-stop delay và chạy lại bước 4 đứng sau một load balancer (hoặc
   một proxy thứ hai đóng vai một cái). **Xong khi** các lỗi
   connection-refused còn sót từ bước 4 biến mất — đo cả hai lần chạy, vì
   đó chính là mấu chốt của khoảng chờ này.
6. Xác minh phép tính. **Xong khi** pre-stop delay + drain deadline chứng
   minh được là dưới grace period của orchestrator, và bạn đã test điều
   gì xảy ra khi *không phải vậy* (đặt grace period thấp và xem
   `SIGKILL` rơi vào giữa lúc drain) để bạn nhận ra failure đó.
7. Thêm các chính sách close tường minh cho kết nối sống lâu. **Xong khi**
   một WebSocket nhận một close frame 1001 và một lệnh gọi gRPC streaming
   nhận `UNAVAILABLE`, và cả hai client reconnect tới một instance khác
   mà không có lỗi hiển thị cho người dùng.
8. Flush telemetry sau khi drain. **Xong khi** các dòng log và span từ
   các request hoàn thành trong lúc shutdown vẫn tới được backend của
   chúng, và các kết nối upstream trong pool được đóng sạch thay vì bị
   reset.
9. Nếu bạn đã xây service discovery ([`06-proxy/07-service-discovery.md`](../06-proxy/07-service-discovery.md)),
   hủy đăng ký như bước drain đầu tiên. **Xong khi** metrics cho thấy tỷ
   lệ request vào giảm về không *trước khi* listener đóng.
