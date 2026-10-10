# Architecture

Phase 9. Cách các mảnh từ [`05-http-stack/`](../05-http-stack), [`06-proxy/`](../06-proxy), và [`07-security/`](../07-security)
khớp lại thành một chương trình duy nhất — và chương trình đó được
reconfigure, deploy, và restart ra sao mà không rớt traffic.

## Files

- [`01-components.md`](01-components.md) — pipeline Listener → ConnMgr → Codec → Router → Modules, và vì sao thứ tự module là một quyết định bảo mật
- [`02-plugin.md`](02-plugin.md) — compile-time composition vs runtime plugin, sandbox một guest, fail-open vs fail-closed
- [`03-config.md`](03-config.md) — reload kiểu validate-rồi-swap, cái gì không hot-reload được, snapshot config theo từng request
- [`04-graceful-shutdown.md`](04-graceful-shutdown.md) — chuỗi drain, pre-stop delay, connection sống lâu, flush telemetry
- [`05-rolling-restart.md`](05-rolling-restart.md) — `SO_REUSEPORT`, chuyển giao fd, socket activation, và state mà một lần restart mất đi
- [`06-canary-deploy.md`](06-canary-deploy.md) — chia traffic theo weight, sticky routing, rollback tự động và vấn đề cỡ mẫu của nó

## Thứ tự đọc

Đọc [`01-components.md`](01-components.md) trước — đó là bản đồ mà phần còn lại treo lên, và
bảng thứ tự module của nó gom lại các ràng buộc rải rác khắp
[`05-http-stack/`](../05-http-stack) và [`07-security/`](../07-security). Rồi tới [`03-config.md`](03-config.md) và
[`04-graceful-shutdown.md`](04-graceful-shutdown.md), thứ hỗ trợ [`labs/13-hot-reload`](../../labs/13-hot-reload) và là điều kiện
tiên quyết cho [`05-rolling-restart.md`](05-rolling-restart.md). [`02-plugin.md`](02-plugin.md) hỗ trợ
[`labs/14-plugin`](../../labs/14-plugin). [`06-canary-deploy.md`](06-canary-deploy.md) đọc cuối; nó xây trên
[`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md).

Đây là phase mà [`proxy/`](../../proxy) ngừng là một tập hợp các lab và trở thành một thứ
bạn có thể chạy thật.
