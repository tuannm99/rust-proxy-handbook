# Linux

Phase 2. Cơ chế cấp syscall mà proxy chạy trên đó — tokio đang làm gì bên
dưới, và kernel sẽ/sẽ không làm gì cho bạn.

## Cách đọc thư mục này

Hai cách dùng, tùy vào bạn đang ở đâu:

- **Chưa có nền tảng OS/systems:** đọc mọi file dưới đây theo thứ tự, từ
  `01-fundamentals.md` đến `11-zerocopy.md`, làm `## Practice` của mỗi
  file trước khi qua file kế tiếp. Coi cả thư mục này như một tutorial
  liên tục — nhóm Kernel mechanisms giả định bạn đã nắm nhóm Fundamentals,
  đừng nhảy cóc.
- **Đã thoải mái với process, syscall, và epoll/select:** bỏ qua hẳn nhóm
  Fundamentals và bắt đầu từ `07-epoll.md` — đó là điểm thư mục này ngừng
  là kiến thức OS chung và bắt đầu là các cơ chế cụ thể (`epoll`,
  `io_uring`, zero-copy) mà một proxy thực sự dựa vào. Nhảy giữa các file
  trong nhóm Kernel mechanisms theo thứ tự bất kỳ khớp với lỗ hổng của
  bạn; chúng không phụ thuộc chặt vào nhau như nhóm Fundamentals.

## Files

**Fundamentals** (bắt đầu từ đây nếu "syscall," "file descriptor,"
"kernel space," hay "container" chưa có nghĩa chính xác với bạn — đây là
nhóm duy nhất trong thư mục này được viết như một primer từ con số 0; mọi
file bên dưới đều giả định bạn đã nắm nhóm này):
- `01-fundamentals.md` — kernel/user-space split để làm gì, và một index tới năm file bên dưới
- `02-processes-and-threads.md` — process vs thread, vì sao thread rẻ hơn, hai scheduler chồng lên nhau (kernel + tokio)
- `03-kernel-and-syscalls.md` — ranh giới syscall và chi phí thật của nó, file descriptor
- `04-memory-basics.md` — virtual address space (một pointer trỏ vào chiều sâu của `09-memory.md`), thứ bậc cache/RAM/disk, stack vs heap
- `05-blocking-io-and-signals.md` — vì sao syscall block, vì sao event loop tồn tại, signal như một thông báo bất đồng bộ từ kernel
- `06-containers.md` — namespace và cgroup: container thật ra là gì, không phải một VM tí hon

**Kernel mechanisms:**
- `07-epoll.md` — readiness notification, level- vs edge-triggered, bug `EAGAIN` bạn nên tự dính
- `08-io_uring.md` — interface async I/O mới hơn và nó khác epoll về bản chất ở đâu
- `09-memory.md` — virtual memory, paging, RSS, thay global allocator
- `10-signals.md` — xử lý signal trong một process bất đồng bộ, quy ước `SIGTERM`/`SIGHUP`
- `11-zerocopy.md` — `sendfile`, `splice`, `mmap`, và khi nào chúng thực sự đáng dùng

## Đi tiếp theo đâu

Nhóm fundamentals trước nếu bạn cần — mọi thứ khác giả định bạn đã đọc nó.
`07-epoll.md` là nền tảng cho `04-runtime/01-tokio.md`; `10-signals.md`
là nền tảng cho `09-architecture/03-config.md` và `04-graceful-shutdown.md`;
`11-zerocopy.md` là nền tảng cho `05-http-stack/05-static.md`. Để biết
chuyện gì xảy ra *bên trong* kernel dưới các lời gọi này, xem `16-kernel/`
— đặc biệt là `01-epoll-internals.md`, `02-io_uring-internals.md`, và
`08-page-cache.md`.
