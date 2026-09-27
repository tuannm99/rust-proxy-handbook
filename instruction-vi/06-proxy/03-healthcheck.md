# Health Check

## What to learn
### Active health check
Proxy định kỳ tự gửi một probe request (TCP connect, hoặc một HTTP GET
tới `/healthz`) tới mỗi upstream theo một timer, độc lập với traffic
thật. Đây là cách duy nhất để phát hiện một upstream đã chết mà đơn giản
là không nhận request thật nào ngay lúc này.

```rust
async fn probe_loop(upstream: std::sync::Arc<Upstream>, interval: std::time::Duration) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let ok = tokio::time::timeout(std::time::Duration::from_secs(2), tcp_probe(&upstream.addr))
            .await
            .is_ok();
        upstream.healthy.store(ok, std::sync::atomic::Ordering::Relaxed);
    }
}
```
Gotcha: probe timeout phải ngắn hơn probe interval, nếu không các probe
chậm sẽ chất đống. Luôn giới hạn probe bằng `tokio::time::timeout`.

Một gotcha thứ hai, tinh vi hơn, với `tokio::time::interval`: hành vi mặc
định `MissedTickBehavior::Burst` của nó bắn ngay lập tức và liên tục để
"đuổi kịp" nếu một tick bị bỏ lỡ (vì một probe chạy quá lâu). Điều đó biến
một probe chậm thành một loạt probe liên tiếp nhắm vào một upstream đã
đang gặp khó khăn. `ticker.set_missed_tick_behavior(MissedTickBehavior::Delay)`
gần như luôn là thứ một health checker muốn.

### Mỗi loại probe thực sự chứng minh điều gì
Độ sâu của probe là một quyết định thiết kế thật sự, không phải chi tiết
vặt vãnh:
- **TCP connect** chứng minh kernel đã chấp nhận một kết nối. Nó *không*
  chứng minh có ứng dụng nào đứng sau socket đó — một process kẹt trong
  vòng lặp vô hạn, hoặc một process mà accept backlog chỉ được kernel một
  mình rút ra, vẫn vượt qua (`16-kernel/03-tcp-stack.md`).
- **HTTP GET `/healthz` trả 200** chứng minh vòng lặp HTTP server còn sống
  và đang lập lịch công việc. Nó không chứng minh upstream có thể phục vụ
  request *thật* nếu `/healthz` là một handler tĩnh không đụng vào gì cả.
- **Một deep check** (handler xác minh database, cache, hoặc dependency
  downstream của nó) chứng minh upstream có thể làm việc thật — và tạo ra
  failure mode bên dưới.

Gotcha: health check phổ biến nhất trong thực tế là một handler 200 tĩnh,
thứ chỉ phát hiện đúng một loại failure (process chết) mà TCP connect đã
phát hiện rẻ hơn. Nếu `/healthz` của bạn không đụng vào bất cứ thứ gì mà
đường đi request thật đụng vào, bạn có một liveness check, không phải một
health check — hãy biết bạn đã xây cái nào.

### Deep check và correlated failure
Một deep check khiến health của mỗi upstream phụ thuộc vào một dependency
*dùng chung*, nên khi dependency đó có sự cố thoáng qua, mọi upstream
trong hạm đội fail probe của nó cùng một thời điểm. Proxy mẫn cán đánh dấu
tất cả chúng unhealthy và giờ không còn ứng viên nào — biến một database
bị suy giảm thành một outage toàn phần, với proxy là cơ chế gây ra nó.

Mọi proxy nghiêm túc đều có một guard cho việc này. Envoy gọi nó là
**panic threshold**: nếu tỷ lệ host khỏe mạnh rơi xuống dưới một ngưỡng
(50% mặc định), Envoy *bỏ qua hoàn toàn health status* và load balance
trên tất cả host, với lý luận rằng nếu phần lớn hạm đội trông như đã chết,
lời giải thích khả dĩ hơn là tín hiệu health đang sai. Cài đặt cái tương
đương trước khi bạn ship deep check:

```rust
// pick among healthy upstreams, but fall back to the whole pool
// once too few are healthy to trust the signal
let healthy: Vec<_> = pool.iter().filter(|u| u.is_healthy()).collect();
let candidates = if (healthy.len() as f64) < 0.5 * pool.len() as f64 {
    pool.all()      // panic mode: health signal is not credible
} else {
    healthy
};
```
Gotcha: fail-open là đúng cho một dependency *dùng chung* và sai cho một
dependency theo-từng-upstream. Nếu các upstream fail độc lập (một deploy
tồi trên một host, một ổ đĩa đầy), panic mode gửi traffic tới các host
thực sự đã chết. Deep check nên kiểm tra các dependency mà upstream sở
hữu riêng; các dependency dùng chung thuộc về một alert
(`08-observability/06-alerting.md`), không thuộc về một phán quyết health
theo-từng-host.

### Passive detection, damping, và slow start
Active probe chỉ là một nửa bức tranh. Các lỗi request thật phát hiện một
upstream tồi nhanh hơn bất kỳ probe interval nào, các ngưỡng giữ cho một
sự cố thoáng qua không rút cạn một host, và một ramp giữ cho một host vừa
hồi phục không bị stampede ngay lúc nó quay lại.

Cả ba nằm trong `06-proxy/04-outlier-detection.md`. Sự phân chia: file này
là "chúng ta đã chủ động đi hỏi"; file kia là "chúng ta nhận ra từ traffic
đang gửi sẵn, và chúng ta giảm nhẹ phản ứng của mình."

### Chi phí probe ở quy mô hạm đội
Traffic probe là `số proxy × số upstream × (1 / interval)` request mỗi
giây, và nó phải trả dù có traffic thật hay không. Hai mươi instance proxy
probe 100 upstream mỗi giây là 2000 req/s chi phí thuần đập vào hạm đội,
và mỗi probe trong số đó rơi vào cùng một thời điểm nếu mọi proxy khởi
động từ một lần deploy config được rollout cùng lúc.

Jitter interval theo từng upstream (một offset ngẫu nhiên ở tick đầu tiên
là đủ) để các probe dàn trải ra trong cửa sổ thay vì dồn dập cùng nhịp —
cùng vấn đề đồng bộ hóa như retry storm trong `05-retry.md`, với cùng cách
sửa.

## Practice
Xây theo thứ tự.

1. Trong `labs/05-reverse-proxy`, thêm một vòng lặp active TCP-connect
   probe cho mỗi upstream với `tokio::time::interval`, giới hạn bởi
   `tokio::time::timeout` và dùng `MissedTickBehavior::Delay`. **Xong khi**
   một upstream giả bạn `kill -STOP` (không kill — dừng lại, để socket vẫn
   mở) vẫn được báo cáo là healthy, và bạn có thể giải thích tại sao từ
   phần probe-depth ở trên.
2. Thay nó bằng một probe HTTP `/healthz`. **Xong khi** trường hợp
   `kill -STOP` giờ báo cáo đúng là unhealthy.
3. Thêm fallback panic-threshold. **Xong khi** một test khiến *mọi*
   upstream fail probe của nó vẫn route traffic (thay vì trả 503 cho tất
   cả), và một test chỉ làm fail một upstream vẫn chỉ loại trừ đúng cái
   đó.
4. Thêm jitter theo từng upstream vào lịch probe. **Xong khi** timestamp
   probe đến tại một upstream, từ 3 instance proxy chạy đồng thời, dàn
   trải khắp interval thay vì dồn cục.
5. Đo chi phí probe. **Xong khi** bạn có thể nói ra số request mỗi giây
   các probe của bạn tạo ra ở quy mô hạm đội của bạn, và đó là một con số
   bạn sẵn sàng trả.
6. Đi qua `06-proxy/04-outlier-detection.md` để học passive detection,
   flap damping, và slow start. **Xong khi** một upstream fail các request
   thật bị loại bỏ trước khi probe tiếp theo bắn, và một upstream đã hồi
   phục ramp trở lại thay vì bị stampede.
