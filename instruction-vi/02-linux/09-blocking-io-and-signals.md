# Blocking I/O, Event Loop và Signal

File này thuộc nhóm fundamentals. Xem [`02-linux/01-fundamentals.md`](01-fundamentals.md) để có mục lục đầy đủ. Hai ý tưởng
được gộp lại vì cả hai cùng xoay quanh một câu hỏi: chương trình của bạn làm sao biết được thứ nó đang chờ đã xảy ra?

## What to learn

### Syscall blocking và non-blocking
Giả sử một syscall chưa có gì để làm, ví dụ `read()` trên một socket chưa có dữ liệu. Có hai cách xử lý:

- **block**: kernel tạm dừng thread gọi, và không cho nó chạy lại cho đến khi dữ liệu tới. Cách này dễ viết, nhưng thread đó
  không làm được việc nào khác trong lúc chờ.
- **non-block**: syscall trả về ngay lập tức với lỗi `EWOULDBLOCK`/`EAGAIN`, nghĩa là "chưa có gì sẵn sàng, hãy hỏi lại sau".

```rust
// blocking: dòng này không return cho đến khi có dữ liệu
let n = blocking_socket.read(&mut buf)?;

// non-blocking: luôn return ngay lập tức
match nonblocking_socket.read(&mut buf) {
    Ok(n) => { /* nhận được n byte */ }
    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => { /* chưa có gì */ }
    Err(e) => return Err(e),
}
```

### Vì sao one-thread-per-connection không scale
Dùng một thread blocking cho mỗi connection là thiết kế đơn giản nhất, và nó không scale. Hàng nghìn keep-alive connection
đang idle nghĩa là hàng nghìn OS thread phần lớn không làm gì. Mỗi thread vẫn có stack riêng trong bộ nhớ và overhead
scheduling riêng của kernel ([`03-processes-and-threads.md`](03-processes-and-threads.md)).

Cách thay thế: dùng non-blocking socket, cộng một cơ chế để hỏi kernel trong một lời gọi duy nhất "trong một nghìn fd này,
cái nào thực sự có gì sẵn sàng", thay vì tự poll từng cái. Cơ chế đó là `epoll`
([`02-linux/14-epoll.md`](14-epoll.md)), và nó là toàn bộ nền tảng của reactor trong tokio
([`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md)).

Đây là lý do tiêu chí hoàn thành của [`labs/00-tcp-server`](../../labs/00-tcp-server) đòi xử lý 500+ connection mà không dùng
500+ thread. Nó buộc bạn tự cảm nhận sự khác biệt mà phần này mô tả, thay vì chỉ đọc về nó.

### Mẫu event loop, cao hơn epoll một tầng
Dù bạn tự viết tay (bài tập trong [`02-linux/14-epoll.md`](14-epoll.md)) hay để tokio làm hộ, hình dạng luôn giống nhau:

1. Đăng ký quan tâm tới một tập fd.
2. Block *một lần*, chờ "báo cho tôi khi bất kỳ fd nào trong số này sẵn sàng", thay vì block riêng từng fd.
3. Khi lần chờ trả về, chuyển sự kiện tới logic sở hữu fd đó.

Một thread (hoặc một pool nhỏ) phục vụ được hàng nghìn connection theo cách này, vì nó không bao giờ bị block để chờ một
connection cụ thể nào. Nó chỉ block tối đa là để chờ *bất cứ thứ gì* sẵn sàng.

### Signal: kernel ngắt process của bạn
Một **signal** là thông báo mà kernel gửi tới một process một cách bất đồng bộ. Nó có thể đến giữa hai instruction bất kỳ, và
ngắt bất cứ việc gì process đang làm. Khác với syscall, nơi code của bạn chủ động khởi xướng. `kill -TERM <pid>` và Ctrl-C đều
hoạt động bằng cách gửi signal.

Đây là một cơ chế giao hoàn toàn khác so với giá trị trả về của hàm hay kết quả của syscall. Chương trình của bạn không hỏi "có
signal nào cho tôi không?". Kernel đơn giản là preempt nó.

### Vì sao xử lý signal ngây thơ rất nguy hiểm
Chính tính bất đồng bộ đó khiến xử lý signal mong manh nếu làm ngây thơ. Một handler chạy "bất cứ lúc nào, bất kể process đang
làm gì" nên không thể an toàn làm hầu hết các việc bình thường (allocate bộ nhớ, lock một mutex). Lý do: nó có thể vừa ngắt đúng
code đang làm chính việc đó. Ví dụ, nếu handler cố lock một mutex mà code bị ngắt đang giữ, process tự deadlock với chính nó.

Tập các thao tác an toàn để làm bên trong một raw signal handler gọi là **async-signal-safe**. Đó là một danh sách ngắn, loại
trừ hầu hết những gì cảm giác như "code bình thường".

Cách sửa chuẩn, và là điều mọi async runtime nghiêm túc đều làm: raw handler chỉ ghi một byte vào pipe/eventfd (hoặc tăng một
atomic; cả hai đều nằm trong danh sách async-signal-safe). Logic reload/shutdown thật chạy sau đó, trên một thread bình thường,
được wake up bởi lần ghi đó thông qua cơ chế event loop mô tả ở trên. `tokio::signal` implement đúng mẫu này cho bạn.
[`02-linux/17-signals.md`](17-signals.md) nói về các signal cụ thể (`SIGHUP`, `SIGTERM`) mà một proxy quan tâm và API tương ứng.

### Sợi chỉ chung
Readiness của blocking I/O và việc giao signal đều là biến thể của cùng một nhu cầu: "chương trình của tôi cần phản ứng với một
thứ bên ngoài, vào lúc nó không kiểm soát được". Khác nhau ở cơ chế:
- Với I/O, bạn thử lại một syscall khi đã sẵn sàng.
- Với signal, sự gián đoạn tự tìm đến bạn bất kể bạn đang ở đâu.

Trong một chương trình async được xây tốt, cả hai cuối cùng đi qua cùng một event loop: epoll readiness event và việc ghi vào
pipe/eventfd do signal tạo ra đều được dispatch bởi cùng một cơ chế. Đó là lý do `tokio::select!` chờ được cả một socket read và
một signal trong cùng một biểu thức, mà bên dưới không có gì đặc biệt xảy ra.

## Practice
1. Viết một chương trình block trên `std::net::TcpStream::read` khi không có dữ liệu tới. Ở terminal khác, xác nhận (qua `ps`
   hoặc `top`) rằng thread đang ngồi idle chứ không phải quay vòng (spin). Sau đó giải thích vì sao 500 thread như vậy vẫn là một
   chi phí tài nguyên thật, dù không thread nào dùng CPU.
2. Trong một scratch project, đặt một socket ở chế độ non-blocking (`set_nonblocking(true)`) và gọi `read()` khi không có dữ liệu
   sẵn sàng. Xác nhận bạn nhận `WouldBlock` ngay, thay vì lời gọi bị treo.
3. Viết một binary nhỏ đăng ký `tokio::signal` handler cho `SIGHUP` và `SIGTERM`, chỉ in ra cái nào vừa bắn. Gửi cả hai bằng
   `kill -HUP <pid>` / `kill -TERM <pid>` và xác nhận cả hai đều bắt được mà không giết process. Sau đó so sánh với việc gửi
   `SIGKILL`, thứ bạn hoàn toàn không bắt được.
4. Đọc danh sách các hàm async-signal-safe (`man 7 signal-safety`) và xác định ít nhất hai thao tác trông rất bình thường (ví dụ
   `malloc`, `printf`) mà *không* nằm trong danh sách. Dựa trên ví dụ mutex ở trên, giải thích vì sao gọi một trong hai từ raw
   handler là nguy hiểm.

---

<!-- nav -->
← Trước: [`02-linux/08-memory-basics.md`](08-memory-basics.md) · [Mục lục](00-README.md) · Tiếp theo: [`02-linux/10-ipc.md`](10-ipc.md) →
