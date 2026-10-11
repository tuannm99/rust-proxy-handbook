# Signal

HUP, TERM, QUIT.

## What to learn

### Ba signal mà proxy thực sự quan tâm
- `SIGHUP`: theo quy ước nghĩa là "reload config mà không restart". Không có handler mặc định nào ép nghĩa này. Đó là quy ước
  mà nginx và hầu hết daemon tuân theo. Liên quan trực tiếp tới [`09-architecture/03-config.md`](../09-architecture/03-config.md).
- `SIGTERM`: "tắt một cách graceful". Ngừng nhận connection mới, hoàn tất các request đang xử lý dở, rồi thoát. Đây là thứ mà
  orchestrator (systemd, Kubernetes) gửi trước khi leo thang lên `SIGKILL`. Liên quan trực tiếp tới
  [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md).
- `SIGINT`/`SIGQUIT`: `SIGINT` là Ctrl-C, thường được xử lý giống `SIGTERM` khi dev. `SIGQUIT` theo truyền thống tạo ra core
  dump, và hiếm khi được xử lý riêng trong một proxy.

### Ràng buộc về signal-safety
Một signal handler Unix truyền thống chạy bất đồng bộ. Nó có thể chạy *bên trong* một syscall khác, trên bất kỳ thread nào mà
kernel chọn. Allocate bộ nhớ, lock một mutex, hay gọi hầu hết hàm libc từ trong một raw handler là hành vi gần với
undefined behavior (chúng không "async-signal-safe").

Cách sửa chuẩn, và là điều mọi async runtime nghiêm túc đều làm: raw handler chỉ ghi một byte vào pipe/eventfd (hoặc tăng một
atomic). Logic reload/shutdown thật chạy sau đó trên một thread bình thường, được wake up bởi byte đó.

### tokio::signal
Tokio implement đúng mẫu đó cho bạn:
```rust
use tokio::signal::unix::{signal, SignalKind};

let mut sighup = signal(SignalKind::hangup())?;
let mut sigterm = signal(SignalKind::terminate())?;

tokio::select! {
    _ = sighup.recv() => { /* reload config */ }
    _ = sigterm.recv() => { /* bắt đầu graceful shutdown */ }
}
```
Gotcha: đăng ký một handler `tokio::signal` cho một `SignalKind` sẽ thay disposition mặc định một lần, toàn cục, cho cả process.
Bạn không thể có hai listener độc lập cùng tranh làm handler SIGTERM. Hãy lấy tín hiệu duy nhất đó rồi phân phát nó tới mọi
subsystem cần phản ứng (drain connection, flush log, dừng listener) từ một chỗ.

### Thứ tự trong triển khai thật
Kubernetes gửi `SIGTERM`, chờ `terminationGracePeriodSeconds` (mặc định 30 giây), rồi gửi `SIGKILL`. Nếu graceful shutdown của
bạn (drain request đang xử lý dở, đóng upstream connection cho sạch) có thể mất lâu hơn khoảng đó dưới tải đỉnh, request vẫn bị
giết cứng. Hãy chỉnh grace period theo p99 request duration thực tế của bạn, đừng để mặc định.

## Practice
1. Viết một binary nhỏ đăng ký handler `tokio::signal` cho SIGHUP và SIGTERM, và chỉ in ra cái nào vừa bắn.
2. Gửi `kill -HUP <pid>` và `kill -TERM <pid>` bằng tay, và xác nhận cả hai đều bắt được mà không giết process.
3. Implement config reload kích hoạt bởi SIGHUP trong [`proxy`](../../proxy) theo
   [`09-architecture/03-config.md`](../09-architecture/03-config.md). Xác nhận các connection hiện có không bị ảnh hưởng bởi một
   lần reload.
4. Implement graceful shutdown kích hoạt bởi SIGTERM theo
   [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md): dừng listener, để request đang xử lý dở
   hoàn tất, rồi thoát.
5. Mô phỏng kịch bản grace period của Kubernetes: giữ một request chậm đang mở, gửi SIGTERM, và xác nhận shutdown của bạn hoặc
   hoàn tất kịp thời, hoặc bị giết một cách sạch sẽ mà không làm hỏng state.
