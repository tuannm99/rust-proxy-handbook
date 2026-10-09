# Kernel

Đi sâu hơn phần ứng dụng-facing của [`02-linux/14-epoll.md`](../02-linux/14-epoll.md)/[`02-linux/15-io_uring.md`](../02-linux/15-io_uring.md)
— chuyện gì thực sự xảy ra bên trong kernel, bên dưới syscall bạn gọi.

## Trạng thái: đã viết; đọc [`02-linux/14-epoll.md`](../02-linux/14-epoll.md)/[`02-linux/15-io_uring.md`](../02-linux/15-io_uring.md) trước

Mọi thứ dưới đây giả định bạn đã va vào phiên bản ứng dụng-facing của cùng
cơ chế (bug `EAGAIN` trong [`02-linux/14-epoll.md`](../02-linux/14-epoll.md), cách dùng
submit/complete cơ bản trong [`02-linux/15-io_uring.md`](../02-linux/15-io_uring.md)) và muốn biết *vì
sao* nó hoạt động như vậy. Nếu đọc thẳng từ đầu, một thứ tự hợp lý là:
[`01-epoll-internals.md`](01-epoll-internals.md) → [`02-io_uring-internals.md`](02-io_uring-internals.md) → [`03-tcp-stack.md`](03-tcp-stack.md) →
[`04-interrupt.md`](04-interrupt.md) → [`05-rss.md`](05-rss.md) → [`06-rps.md`](06-rps.md) → [`07-scheduler.md`](07-scheduler.md) → [`08-page-cache.md`](08-page-cache.md) →
[`09-ebpf.md`](09-ebpf.md) → [`10-xdp.md`](10-xdp.md) (hai file cuối phục vụ trực tiếp [`labs/17-ebpf`](../../labs/17-ebpf)
và đọc riêng lẻ cũng ổn khi tới lab đó).

## Đã viết

- [`09-ebpf.md`](09-ebpf.md) — VM eBPF, verifier, map, attach point, Aya; phục vụ [`labs/17-ebpf`](../../labs/17-ebpf)
- [`10-xdp.md`](10-xdp.md) — drop packet ngay trong NIC driver, các attach mode, vòng feedback proxy↔XDP
- [`08-page-cache.md`](08-page-cache.md) — kernel cache dữ liệu file thế nào, và nó tương tác ra sao với `mmap`/`sendfile` trong [`02-linux/18-zerocopy.md`](../02-linux/18-zerocopy.md)
- [`03-tcp-stack.md`](03-tcp-stack.md) — state machine TCP của kernel, SYN backlog vs accept backlog, TIME_WAIT
- [`01-epoll-internals.md`](01-epoll-internals.md) — epoll được implement bên trong kernel thế nào (red-black tree các fd đang theo dõi, ready list)
- [`02-io_uring-internals.md`](02-io_uring-internals.md) — cơ chế submission/completion queue bên dưới API của crate `io_uring`
- [`07-scheduler.md`](07-scheduler.md) — kiến thức cơ bản về CPU scheduling liên quan tới một proxy nhạy cảm về latency (CFS, priority, `nice`)
- [`04-interrupt.md`](04-interrupt.md) — hardware interrupt vs softirq, vì sao chúng quan trọng với workload nặng về network
- [`05-rss.md`](05-rss.md) — Receive Side Scaling, dàn interrupt của NIC ra nhiều core ở tầng hardware
- [`06-rps.md`](06-rps.md) — Receive Packet Steering, phương án phần mềm khi hardware queue không đủ
