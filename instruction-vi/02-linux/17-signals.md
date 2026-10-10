# Signals

HUP, TERM, QUIT.

## What to learn

### Ba signal một proxy thực sự quan tâm
- `SIGHUP` — theo quy ước nghĩa là "reload config mà không restart."
  Không có handler mặc định nào ép nghĩa này; đó là quy ước mà nginx và
  hầu hết daemon tuân theo. Gắn trực tiếp với [`09-architecture/03-config.md`](../09-architecture/03-config.md).
- `SIGTERM` — "tắt một cách graceful": ngừng nhận connection mới, hoàn tất
  các request đang xử lý dở, rồi thoát. Đây là thứ các orchestrator
  (systemd, Kubernetes) gửi trước khi leo thang lên `SIGKILL`. Gắn trực
  tiếp với [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md).
- `SIGINT`/`SIGQUIT` — `SIGINT` là Ctrl-C, thường được xử lý giống
  `SIGTERM` khi dev; `SIGQUIT` theo truyền thống kích hoạt một core dump
  và hiếm khi được xử lý đặc biệt trong một proxy.

### Ràng buộc về signal-safety
Một signal handler Unix truyền thống chạy bất đồng bộ, có thể *bên trong*
một syscall khác, trên bất kỳ thread nào kernel chọn — allocate memory,
lock một mutex, hay gọi hầu hết hàm libc từ bên trong một raw handler là
undefined-behavior-liền-kề (không "async-signal-safe"). Cách sửa
idiomatic, và điều mọi async runtime nghiêm túc đều làm, là: raw handler
không làm gì ngoài ghi một byte vào một pipe/eventfd (hoặc tăng một
atomic), và logic reload/shutdown thật sự của bạn chạy sau đó trên một
thread bình thường được wake up bởi byte đó.

### tokio::signal
Tokio implement chính xác mẫu đó cho bạn:
```rust
use tokio::signal::unix::{signal, SignalKind};

let mut sighup = signal(SignalKind::hangup())?;
let mut sigterm = signal(SignalKind::terminate())?;

tokio::select! {
    _ = sighup.recv() => { /* reload config */ }
    _ = sigterm.recv() => { /* bắt đầu graceful shutdown */ }
}
```
Gotcha: đăng ký một handler `tokio::signal` cho một `SignalKind` cho
trước sẽ thay thế disposition mặc định một lần, toàn cục, theo từng
process — bạn không thể có hai listener độc lập tranh nhau là "cái"
handler SIGTERM; hãy phân phối tín hiệu nhận được duy nhất đó ra mọi
subsystem cần phản ứng (drain connection, flush log, dừng listener) từ một
chỗ.

### Thứ tự trong triển khai thật
Kubernetes gửi `SIGTERM`, chờ `terminationGracePeriodSeconds` (mặc định
30s), rồi gửi `SIGKILL`. Nếu graceful shutdown của bạn (drain request
đang xử lý dở, đóng connection upstream sạch sẽ) có thể mất lâu hơn cửa sổ
đó dưới tải đỉnh, request vẫn bị giết cứng — grace period phải được tune
theo p99 request duration thực tế của bạn, không để ở mặc định.

## Practice
1. Viết một binary nhỏ đăng ký handler `tokio::signal` cho SIGHUP và
   SIGTERM và chỉ in ra cái nào đã bắn.
2. Gửi `kill -HUP <pid>` và `kill -TERM <pid>` thủ công và xác nhận cả
   hai đều bắt được mà không giết process.
3. Implement config reload kích hoạt bởi SIGHUP trong [`proxy`](../../proxy) theo
   [`09-architecture/03-config.md`](../09-architecture/03-config.md) — xác nhận các connection hiện có không bị
   ảnh hưởng bởi một lần reload.
4. Implement graceful shutdown kích hoạt bởi SIGTERM theo
   [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md): dừng listener, để request
   đang xử lý dở hoàn tất, rồi thoát.
5. Mô phỏng kịch bản grace-period của Kubernetes: giữ một request chậm
   đang mở, gửi SIGTERM, và xác nhận shutdown của bạn hoặc hoàn tất kịp
   thời hoặc bị giết sạch sẽ thay vì làm hỏng trạng thái.
