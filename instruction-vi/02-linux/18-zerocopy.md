# Zero-copy

sendfile, splice, mmap.

## What to learn

### Những lần copy mà bạn muốn tránh
Cách ngây thơ để "phục vụ một file qua socket" làm như sau:
1. `read()` copy dữ liệu file từ page cache của kernel vào một buffer trong userspace.
2. `write()` copy nó ngược lại từ userspace vào socket buffer của kernel.

Đó là hai lần copy và hai chuyến đi-về giữa user và kernel, cho dữ liệu mà process của bạn thực ra chưa bao giờ cần chạm vào. Các
syscall zero-copy để kernel chuyển dữ liệu thẳng từ page cache tới socket.

### sendfile
```rust
// libc::sendfile(out_fd, in_fd, offset, count)
// out_fd phải là một socket (hoặc tương tự); in_fd phải là một file thường.
let sent = unsafe { libc::sendfile(sock_fd, file_fd, std::ptr::null_mut(), len) };
```
Đây là primitive zero-copy kinh điển cho "phục vụ một static file", và đúng là thứ mà
[`05-http-stack/06-static.md`](../05-http-stack/06-static.md) nên dùng ở happy path.

Gotcha: `sendfile` yêu cầu nguồn phải là một *file*. Bạn không `sendfile` từ socket sang socket được. Điều này quan trọng với
reverse proxy khi relay response từ upstream.

### splice và vectored I/O
`splice(2)` chuyển dữ liệu giữa hai fd, *với điều kiện ít nhất một trong hai là pipe*, mà không copy qua userspace. Đây là cách bạn
có relay zero-copy từ socket sang socket cho proxy (client-socket → pipe → upstream-socket), điều `sendfile` không làm được.

`writev`/`readv` (vectored I/O, trong Rust là `IoSlice`/`IoSliceMut` và `AsyncWrite::poll_write_vectored`) cho phép bạn ghi nhiều
buffer không liền kề nhau (ví dụ một header vừa dựng cộng một body đang stream) trong một syscall, thay vì gộp chúng vào một
buffer trước.

### mmap
`mmap` một file là ánh xạ các page của nó thẳng vào address space của bạn, dựa trên page cache. Khi đọc, các page được nạp lười
(fault vào khi cần), và OS lo việc caching cho bạn. Cách này hữu ích với các static asset lớn mà bạn truy cập ngẫu nhiên (không
hoàn toàn tuần tự).

Gotcha: nếu một file đã `mmap` bị truncate hoặc sửa ngay dưới chân bạn trong khi đang được map, process của bạn có thể bị
`SIGBUS` ở lần truy cập tiếp theo. Điều này nguy hiểm với một proxy chạy lâu, phục vụ các file do người dùng upload hoặc bị ghi đè
thường xuyên. `sendfile` không có kiểu lỗi này.

### Vì sao zero-copy khó khi có TLS
Không syscall nào ở trên biết gì về TLS. `sendfile`/`splice` chuyển byte *mà không quan tâm đó là ciphertext*. Nhưng TLS đòi hỏi
mã hóa dữ liệu trong userspace trước khi nó lên dây, nghĩa là dữ liệu vẫn phải đi qua một buffer userspace để mã hóa.

Kernel TLS (`kTLS`, `setsockopt` với `SOL_TLS`) đẩy chính bước mã hóa/giải mã vào kernel, để `sendfile` hoạt động trở lại cả với
TLS. Nhưng đây là một tính năng mới hơn, hỗ trợ hẹp hơn (cần kernel phù hợp, và thường cần NIC hỗ trợ offload cụ thể). Hỗ trợ từ
hệ sinh thái Rust (`ktls`, kết hợp với `rustls`) cũng kém trưởng thành hơn nhiều so với `rustls` thuần.

Trong thực tế: [`proxy`](../../proxy) kết thúc TLS ([`01-network/19-tls.md`](../01-network/19-tls.md)) sẽ copy và mã hóa trong
userspace ở đường response, trừ khi bạn chủ động dùng kTLS. Đó là một mặc định bình thường, chấp nhận được. Đừng coi việc mất
zero-copy khi có TLS là bug.

## Practice
1. Viết một static file server nhỏ dùng `libc::sendfile` thô, và benchmark nó với một vòng lặp read+write ngây thơ bằng `wrk` trên
   một file lớn.
2. Implement relay socket-sang-socket qua `splice` cho một TCP proxy trần, và xác nhận (qua `strace`) không có copy buffer
   userspace nào xảy ra.
3. `mmap` một file, đọc từ mapping, rồi truncate file đó từ một process khác và quan sát `SIGBUS`.
4. Dùng vectored write (`IoSlice`) trong [`labs/04-static-server`](../../labs/04-static-server) để ghi response header và body
   trong một syscall, thay vì gộp buffer.
5. Đọc thêm về hỗ trợ kTLS trong `rustls`/crate `ktls`, và viết một ghi chú ngắn về việc có đáng theo đuổi cho [`proxy`](../../proxy)
   hay không.

---

<!-- nav -->
← Trước: [`02-linux/17-signals.md`](17-signals.md) · [Mục lục](00-README.md) · Tiếp theo: [`02-linux/19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md) →
