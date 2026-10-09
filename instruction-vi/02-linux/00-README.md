# Linux

Phase 2. Cơ chế cấp syscall mà proxy chạy trên đó — tokio đang làm gì bên
dưới, và kernel sẽ/sẽ không làm gì cho bạn.

## Cách đọc thư mục này

Hai cách dùng, tùy vào bạn đang ở đâu:

- **Chưa có nền tảng OS/systems:** đọc mọi file dưới đây theo thứ tự, từ
  [`01-fundamentals.md`](01-fundamentals.md) đến [`21-systemd-and-services.md`](21-systemd-and-services.md), làm `## Practice` của mỗi
  file trước khi qua file kế tiếp. Coi cả thư mục này như một tutorial
  liên tục — nhóm Kernel mechanisms giả định bạn đã nắm nhóm Fundamentals,
  đừng nhảy cóc.
- **Đã thoải mái với process, syscall, và epoll/select:** bỏ qua hẳn nhóm
  Fundamentals và bắt đầu từ [`14-epoll.md`](14-epoll.md) — đó là điểm thư mục này ngừng
  là kiến thức OS chung và bắt đầu là các cơ chế cụ thể (`epoll`,
  `io_uring`, zero-copy) mà một proxy thực sự dựa vào. Nhảy giữa các file
  trong nhóm Kernel mechanisms theo thứ tự bất kỳ khớp với lỗ hổng của
  bạn; chúng không phụ thuộc chặt vào nhau như nhóm Fundamentals.

## Files

**Fundamentals** (bắt đầu từ đây nếu "syscall," "file descriptor,"
"kernel space," "inode," hay "container" chưa có nghĩa chính xác với bạn — đây là
nhóm trong thư mục này được viết như một primer từ con số 0; mọi
file bên dưới đều giả định bạn đã nắm nhóm này):
- [`01-fundamentals.md`](01-fundamentals.md) — kernel/user-space split để làm gì, và một index tới mười hai file bên dưới
- [`02-hardware-basics.md`](02-hardware-basics.md) — privilege level của CPU, MMU, interrupt, DMA, và một packet tới `read()` của bạn thế nào
- [`03-processes-and-threads.md`](03-processes-and-threads.md) — process vs thread, vì sao thread rẻ hơn, hai scheduler chồng lên nhau (kernel + tokio)
- [`04-process-lifecycle.md`](04-process-lifecycle.md) — `fork`/`exec`/`wait`, zombie, orphan, process group, PID 1 trong container
- [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md) — ranh giới syscall và chi phí thật của nó, file descriptor
- [`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md) — inode, path, VFS, page cache vs durability, atomic replace, `/proc`
- [`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md) — UID, permission bit, capability, hạ đặc quyền, port 443 không cần root
- [`08-memory-basics.md`](08-memory-basics.md) — virtual address space (một pointer trỏ vào chiều sâu của [`16-memory.md`](16-memory.md)), thứ bậc cache/RAM/disk, stack vs heap
- [`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md) — vì sao syscall block, vì sao event loop tồn tại, signal như một thông báo bất đồng bộ từ kernel
- [`10-ipc.md`](10-ipc.md) — pipe, Unix socket, truyền fd bằng `SCM_RIGHTS`, shared memory, `eventfd`, `futex`
- [`11-time-and-timers.md`](11-time-and-timers.md) — monotonic vs wall-clock, timer wheel, timeout và deadline
- [`12-cpu-scheduling.md`](12-cpu-scheduling.md) — run queue, CFS, nice/affinity, load average, cgroup CPU throttling
- [`13-containers.md`](13-containers.md) — namespace và cgroup: container thật ra là gì, không phải một VM tí hon

**Kernel mechanisms và vận hành:**
- [`14-epoll.md`](14-epoll.md) — readiness notification, level- vs edge-triggered, bug `EAGAIN` bạn nên tự dính
- [`15-io_uring.md`](15-io_uring.md) — interface async I/O mới hơn và nó khác epoll về bản chất ở đâu
- [`16-memory.md`](16-memory.md) — page fault, mmap, RSS/PSS, swap và reclaim, OOM killer, huge page, NUMA, thay global allocator
- [`17-signals.md`](17-signals.md) — xử lý signal trong một process bất đồng bộ, quy ước `SIGTERM`/`SIGHUP`
- [`18-zerocopy.md`](18-zerocopy.md) — `sendfile`, `splice`, `mmap`, và khi nào chúng thực sự đáng dùng
- [`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md) — netfilter hook, iptables/nftables, conntrack, NAT, transparent proxying, namespace/veth, `tc netem`
- [`20-limits-and-proc.md`](20-limits-and-proc.md) — rlimit và `EMFILE`, `/proc`, và hộp công cụ quan sát theo câu hỏi
- [`21-systemd-and-services.md`](21-systemd-and-services.md) — unit file, hợp đồng SIGTERM/restart, socket activation, hardening

**Review:**
- [`22-recall-and-review.md`](22-recall-and-review.md) — bộ khung mười lăm sự thật, câu hỏi theo từng file, hình vẽ lại từ trí nhớ, thí nghiệm dự đoán-rồi-chạy; dùng theo lịch 1/3/7/21 ngày để kiến thức nhớ lâu

## Đi tiếp theo đâu

Nhóm fundamentals trước nếu bạn cần — mọi thứ khác giả định bạn đã đọc nó.
[`14-epoll.md`](14-epoll.md) là nền tảng cho [`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md); [`17-signals.md`](17-signals.md)
là nền tảng cho [`09-architecture/03-config.md`](../09-architecture/03-config.md) và [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md);
[`18-zerocopy.md`](18-zerocopy.md) là nền tảng cho [`05-http-stack/06-static.md`](../05-http-stack/06-static.md); [`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md) là nơi [`01-network/`](../01-network) gặp OS; [`21-systemd-and-services.md`](21-systemd-and-services.md) và [`20-limits-and-proc.md`](20-limits-and-proc.md) là thứ bạn cần để triển khai và vận hành [`proxy/`](../../proxy). Để biết
chuyện gì xảy ra *bên trong* kernel dưới các lời gọi này, xem [`16-kernel/`](../16-kernel)
— đặc biệt là [`16-kernel/01-epoll-internals.md`](../16-kernel/01-epoll-internals.md), [`16-kernel/02-io_uring-internals.md`](../16-kernel/02-io_uring-internals.md), và
[`16-kernel/08-page-cache.md`](../16-kernel/08-page-cache.md).
