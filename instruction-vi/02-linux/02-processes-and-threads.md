# Processes and Threads

Một phần của chuỗi fundamentals từ-con-số-0 — xem
[`02-linux/01-fundamentals.md`](01-fundamentals.md) để có index đầy đủ.

## What to learn

### Process: thế giới riêng của nó
Một **process** là một chương trình đang chạy với address space riêng của
nó — góc nhìn memory riêng, cách ly khỏi mọi process khác, được enforce
bởi phần cứng quản lý memory của kernel (MMU, xem [`04-memory-basics.md`](04-memory-basics.md)).
Hai process không thể đọc hoặc làm hỏng memory của nhau một cách tình cờ.
Một process cũng sở hữu tập file descriptor đang mở của riêng nó
([`03-kernel-and-syscalls.md`](03-kernel-and-syscalls.md)), process ID riêng, và resource limit riêng.

Tạo một process mới (`fork()` trên Unix, nằm dưới `std::process::Command`
trong Rust) tương đối tốn kém: phải thiết lập một address space mới (dù
copy-on-write của Linux làm cho bản thân `fork()` *ban đầu* rẻ — chi phí
thật xuất hiện khi con ghi vào bản copy trang riêng của nó).

### Thread: chia sẻ thế giới, nhưng không chia sẻ stack
Một **thread** là một đơn vị lập lịch *bên trong* một process. Mọi thread
trong một process **chia sẻ** address space của process đó — cùng heap,
cùng biến global, cùng tập file descriptor đang mở — nhưng mỗi thread có
stack riêng và trạng thái CPU register riêng, nên kernel có thể chạy các
thread độc lập và xen kẽ hoặc chạy song song chúng trên nhiều core.

Đây là lý do thread rẻ hơn process để tạo (không cần thiết lập address
space mới, chỉ cần một stack mới và một ít sổ sách kế toán của kernel) và
vì sao chia sẻ dữ liệu giữa các thread thì dễ-nhưng-rủi ro (cùng memory,
nên data race có thể xảy ra nếu hai thread đụng vào nó mà không phối hợp)
trong khi chia sẻ dữ liệu giữa các process thì khó-nhưng-an-toàn-mặc-định
(memory riêng; bạn phải chủ động chọn chia sẻ qua IPC — pipe, shared
memory segment, socket).

```rust
// hai thread chia sẻ heap của một process qua Arc<Mutex<_>> — điều này
// chỉ compile/có ý nghĩa vì thread chia sẻ address space; tương đương
// giữa hai process riêng biệt cần IPC thật sự thay vì thế
let counter = std::sync::Arc::new(std::sync::Mutex::new(0));
let c2 = counter.clone();
std::thread::spawn(move || { *c2.lock().unwrap() += 1; });
```

Toàn bộ chủ đề của [`03-rust/04-sync.md`](../03-rust/04-sync.md) — `Arc`, `Mutex`, atomic — tồn tại
vì tokio chạy các task async của bạn trên một pool OS thread chia sẻ một
address space, và hệ thống kiểu của Rust là thứ bắt được rủi ro
shared-memory nói trên tại compile time thay vì lúc 3 giờ sáng trên
production.

### Tokio task là cả hai thứ, và không phải cả hai: rẻ hơn cả hai
Runtime của tokio ([`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md)) là một pool nhỏ các OS thread
thật, mỗi thread có khả năng chạy nhiều task `async fn` của bạn, chuyển
đổi giữa chúng một cách cooperative. Một tokio task không phải một thread
và cũng không phải một process — nó rẻ hơn cả hai (không có kernel stack,
không có một entity lập lịch riêng mà *kernel* biết đến), và rất nhiều
task như vậy chia sẻ thời gian trên một nhúm OS thread thật. Cấu trúc ba
tầng này — process chứa thread, thread (trong một chương trình async)
chạy nhiều task — đáng để nắm rõ: "concurrency" (nhiều task cùng tiến
triển theo kiểu xen kẽ) và "parallelism" (nhiều thứ chạy đúng cùng một
thời điểm, cần nhiều core) có liên quan nhưng khác nhau; runtime
multi-threaded của tokio cho bạn cả hai, một runtime single-threaded chỉ
cho bạn cái đầu tiên.

### Hai scheduler, chồng lên nhau
Với nhiều thread có thể chạy hơn số CPU core — bình thường trên bất kỳ máy
thật nào — scheduler của kernel quyết định thread nào chạy trên core nào
trong bao lâu, chuyển đổi giữa chúng (một context switch, chủ đề của
[`03-kernel-and-syscalls.md`](03-kernel-and-syscalls.md)). Tokio có scheduler *riêng* của nó ở một tầng
cao hơn, quyết định một OS thread cho trước sẽ làm task nào của *bạn*
tiếp theo. Đây thực sự là hai scheduler khác nhau, hoạt động độc lập:
kernel hoàn toàn không biết các tokio task của bạn tồn tại, và tokio không
kiểm soát worker thread của chính nó chạy trên core nào. Khi bạn đang chẩn
đoán một độ trễ bất thường ([`04-runtime/02-waker.md`](../04-runtime/02-waker.md),
[`08-observability/04-profiling.md`](../08-observability/04-profiling.md)), biết mình đang nhìn vào tầng nào
trong hai tầng đó thường là toàn bộ câu hỏi.

Gotcha: một tokio task chạy một tính toán đồng bộ dài sẽ block *OS thread*
mà nó đang nằm trên đó — và vì thread đó được scheduler của tokio dùng
chung cho có thể nhiều task, một task tệ sẽ làm nghẽn mọi task khác đang
xếp hàng trên thread đó, hoàn toàn vô hình với kernel scheduler (OS thread
trông vẫn bận rộn hoàn hảo; nó chỉ bận sai việc).

## Practice
1. Viết một chương trình nhỏ spawn 3 OS thread chia sẻ một `Vec` sau một
   `Mutex`, và một phiên bản thứ hai dùng `Rc` thuần không đồng bộ hóa —
   xác nhận phiên bản thứ hai không compile, và đọc lỗi compiler để xem
   nó thực sự phản đối điều gì.
2. Chạy `ps -eLf` (Linux) và tìm một process multi-threaded trên máy bạn
   (ví dụ browser, hoặc một chương trình `tokio` đang chạy) — đếm nó có
   bao nhiêu thread (LWP) so với một process single-threaded như shell.
3. Spawn một instance [`labs/00-tcp-server`](../../labs/00-tcp-server) và, trong khi nó đang xử lý vài
   kết nối idle, kiểm tra `ps -eLf | grep tcp-server` — xác nhận số OS
   thread nhỏ và xấp xỉ số core của bạn, không phải số kết nối.
4. Viết một chương trình nhỏ tốn 2 giây trong một vòng lặp CPU chặt bên
   trong một task `tokio::spawn` trên một runtime single-threaded
   (`#[tokio::main(flavor = "current_thread")]`) trong khi một task khác
   cố `tokio::time::sleep` 100ms đồng thời — đo xem sleep thực sự bắn
   trễ bao lâu, và giải thích tại sao dựa trên mô hình scheduler-chồng ở
   trên.
