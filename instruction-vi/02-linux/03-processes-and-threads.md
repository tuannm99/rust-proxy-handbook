# Process và Thread

File này thuộc nhóm fundamentals. Xem [`02-linux/01-fundamentals.md`](01-fundamentals.md) để có mục lục đầy đủ.

## What to learn

### Process: thế giới riêng của nó
Một **process** là một chương trình đang chạy, có address space riêng. Nó có góc nhìn bộ nhớ riêng, bị cách ly khỏi mọi
process khác. Việc cách ly này do phần cứng quản lý bộ nhớ của kernel đảm bảo (MMU, xem
[`08-memory-basics.md`](08-memory-basics.md)). Hai process không thể vô tình đọc hoặc làm hỏng bộ nhớ của nhau.

Một process cũng sở hữu riêng: tập file descriptor đang mở ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)),
process ID, và resource limit.

Tạo một process mới (`fork()` trên Unix, nằm bên dưới `std::process::Command` trong Rust) tương đối tốn kém, vì phải
thiết lập một address space mới. Copy-on-write của Linux làm cho bản thân `fork()` *lúc đầu* rẻ. Chi phí thật xuất hiện
khi process con ghi vào bản copy page riêng của nó.

### Thread: chia sẻ thế giới, nhưng không chia sẻ stack
Một **thread** là một đơn vị scheduling *bên trong* một process. Mọi thread trong một process **chia sẻ** address space của
process đó: cùng heap, cùng biến global, cùng tập file descriptor đang mở. Nhưng mỗi thread có stack riêng và state CPU
register riêng. Nhờ vậy kernel chạy được các thread một cách độc lập, xen kẽ hoặc song song trên nhiều core.

Vì thế:
- Thread rẻ hơn process để tạo. Không cần thiết lập address space mới, chỉ cần một stack mới và một ít sổ sách của kernel.
- Chia sẻ dữ liệu giữa các thread thì dễ nhưng rủi ro: cùng một bộ nhớ, nên có thể xảy ra data race nếu hai thread chạm vào
  nó mà không phối hợp.
- Chia sẻ dữ liệu giữa các process thì khó hơn nhưng an toàn theo mặc định: bộ nhớ riêng, bạn phải chủ động chia sẻ qua
  IPC (pipe, shared memory segment, socket).

```rust
// hai thread chia sẻ heap của một process qua Arc<Mutex<_>>. Cách này chỉ có nghĩa
// vì các thread chia sẻ address space. Với hai process riêng biệt, bạn cần IPC thật sự
let counter = std::sync::Arc::new(std::sync::Mutex::new(0));
let c2 = counter.clone();
std::thread::spawn(move || { *c2.lock().unwrap() += 1; });
```

Toàn bộ chủ đề của [`03-rust/04-sync.md`](../03-rust/04-sync.md) (`Arc`, `Mutex`, atomic) tồn tại vì tokio chạy các async
task của bạn trên một pool OS thread dùng chung address space. Hệ thống kiểu của Rust bắt rủi ro shared memory đó ngay khi
compile, thay vì lúc 3 giờ sáng trên production.

### Tokio task không phải thread, cũng không phải process: nó rẻ hơn cả hai
Runtime của tokio ([`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md)) là một pool nhỏ các OS thread thật. Mỗi thread có
thể chạy nhiều task `async fn` của bạn, chuyển qua lại giữa chúng theo kiểu cooperative.

Một tokio task không phải thread và cũng không phải process. Nó rẻ hơn cả hai: không có kernel stack, và kernel không biết
nó là một đối tượng scheduling riêng. Rất nhiều task như vậy chia nhau thời gian chạy trên một nhóm nhỏ OS thread thật.

Cấu trúc ba tầng này đáng nắm rõ: process chứa thread, và thread (trong chương trình async) chạy nhiều task. Cũng cần phân
biệt hai khái niệm có liên quan nhưng khác nhau:
- **Concurrency**: nhiều task cùng tiến triển theo kiểu xen kẽ.
- **Parallelism**: nhiều việc chạy đúng cùng một lúc, cần nhiều core.

Runtime multi-threaded của tokio cho bạn cả hai. Runtime single-threaded chỉ cho bạn cái đầu.

### Hai scheduler chồng lên nhau
Số thread có thể chạy thường nhiều hơn số CPU core. Điều này bình thường trên mọi máy thật. Scheduler của kernel quyết định
thread nào chạy trên core nào trong bao lâu, và chuyển qua lại giữa chúng (một context switch, xem
[`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)).

Tokio có scheduler *riêng*, ở tầng cao hơn. Nó quyết định một OS thread nhất định sẽ chạy task *nào của bạn* tiếp theo.

Đây là hai scheduler khác nhau, hoạt động độc lập. Kernel hoàn toàn không biết các tokio task của bạn tồn tại, và tokio
không kiểm soát worker thread của nó chạy trên core nào. Khi chẩn đoán một latency bất thường
([`04-runtime/02-waker.md`](../04-runtime/02-waker.md), [`08-observability/04-profiling.md`](../08-observability/04-profiling.md)),
việc biết mình đang nhìn vào tầng nào trong hai tầng đó thường chính là toàn bộ câu hỏi.

Gotcha: nếu một tokio task chạy một phép tính đồng bộ dài, nó block *OS thread* mà nó đang nằm trên. Thread đó được scheduler
của tokio dùng chung cho nhiều task, nên một task xấu làm nghẽn mọi task khác đang xếp hàng trên thread đó. Kernel scheduler
không thấy gì bất thường: OS thread trông vẫn bận hoàn toàn bình thường. Nó chỉ đang bận sai việc.

## Practice
1. Viết một chương trình nhỏ spawn 3 OS thread cùng chia sẻ một `Vec` sau một `Mutex`. Viết phiên bản thứ hai dùng `Rc`
   thuần, không đồng bộ hóa. Xác nhận phiên bản thứ hai không compile, và đọc lỗi của compiler xem nó thực sự phản đối điều
   gì.
2. Chạy `ps -eLf` (Linux) và tìm một process multi-threaded trên máy bạn (ví dụ browser, hoặc một chương trình `tokio` đang
   chạy). Đếm nó có bao nhiêu thread (LWP), và so với một process single-threaded như shell.
3. Chạy một instance [`labs/00-tcp-server`](../../labs/00-tcp-server). Trong lúc nó đang xử lý vài connection idle, kiểm tra
   `ps -eLf | grep tcp-server`. Xác nhận số OS thread nhỏ và xấp xỉ số core của bạn, không phải số connection.
4. Viết một chương trình nhỏ chạy một vòng lặp CPU liên tục trong 2 giây bên trong một task `tokio::spawn`, trên một runtime
   single-threaded (`#[tokio::main(flavor = "current_thread")]`). Đồng thời một task khác cố `tokio::time::sleep` 100 ms. Đo
   xem sleep thực sự bị trễ bao lâu, và giải thích vì sao dựa trên mô hình hai scheduler chồng lên nhau ở trên.

---

<!-- nav -->
← Trước: [`02-linux/02-hardware-basics.md`](02-hardware-basics.md) · [Mục lục](00-README.md) · Tiếp theo: [`02-linux/04-process-lifecycle.md`](04-process-lifecycle.md) →
