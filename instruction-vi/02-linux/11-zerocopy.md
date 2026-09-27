# Zero-copy

sendfile, splice, mmap.

## What to learn

### Bản copy bạn đang cố tránh
Một cách "phục vụ một file qua socket" ngây thơ làm: read() copy dữ liệu
file từ page cache của kernel vào một buffer userspace, rồi write() copy
nó ngược lại từ userspace vào socket buffer của kernel. Đó là hai lần
copy và hai chuyến khứ hồi context-switch cho dữ liệu mà process của bạn
chưa bao giờ thực sự cần đụng vào. Các syscall zero-copy để kernel di
chuyển dữ liệu trực tiếp từ page-cache tới socket.

### sendfile
```rust
// libc::sendfile(out_fd, in_fd, offset, count)
// out_fd phải là một socket (hoặc tương tự); in_fd phải là một file thường.
let sent = unsafe { libc::sendfile(sock_fd, file_fd, std::ptr::null_mut(), len) };
```
Đây là primitive zero-copy kinh điển cho "phục vụ một static file" và
chính xác là thứ `05-http-stack/05-static.md` nên dùng ở happy path.
Gotcha: `sendfile` yêu cầu nguồn phải là một *file* — bạn không thể
`sendfile` socket-tới-socket, điều này quan trọng với một reverse proxy
relay response từ upstream.

### splice và vectored I/O
`splice(2)` di chuyển dữ liệu giữa hai fd *khi ít nhất một cái là một
pipe*, mà không copy qua userspace — đây là cách bạn có được relay
zero-copy socket-tới-socket (client-socket -> pipe -> upstream-socket)
cho một proxy, khác với `sendfile`. `writev`/`readv` (vectored I/O, lộ ra
trong Rust qua `IoSlice`/`IoSliceMut` và
`AsyncWrite::poly_write_vectored`) cho bạn ghi nhiều buffer không liền kề
(ví dụ một header bạn vừa xây cộng một body bạn đang stream) trong một
syscall thay vì gộp chúng vào một buffer trước.

### mmap
`mmap` một file ánh xạ các trang của nó trực tiếp vào address space của
bạn, được backing bởi page cache — các lần đọc fault trang vào một cách
lazy, và OS lo việc caching cho bạn. Hữu ích cho các static asset lớn bạn
sẽ truy cập ngẫu nhiên (không hoàn toàn tuần tự), nhưng gotcha: một file
`mmap`'d bị truncate hoặc sửa đổi ngay dưới chân bạn trong khi đang được
map có thể `SIGBUS` process của bạn ở lần truy cập tiếp theo — nguy hiểm
cho một proxy chạy dài phục vụ file do người dùng upload hoặc bị ghi đè
thường xuyên; `sendfile` không có failure mode này.

### Vì sao zero-copy khó khi có TLS
Không syscall nào trong số này biết gì về TLS — `sendfile`/`splice` di
chuyển byte *không quan tâm đến ciphertext*, nhưng TLS yêu cầu mã hóa dữ
liệu ở userspace trước khi nó lên dây, nghĩa là dữ liệu vẫn phải đi qua
một buffer userspace để mã hóa dù sao. Kernel TLS (`kTLS`, `setsockopt`
với `SOL_TLS`) đẩy chính bước encrypt/decrypt vào kernel để `sendfile` có
thể hoạt động lại ngay cả với TLS, nhưng đó là một tính năng mới hơn, hỗ
trợ hẹp hơn (cần kernel + thường cần hỗ trợ offload NIC cụ thể) và hỗ trợ
từ hệ sinh thái Rust (`ktls`, gắn với `rustls`) kém trưởng thành hơn nhiều
so với `rustls` thuần. Trong thực tế: `proxy` terminate TLS
(`01-network/13-tls.md`) sẽ làm một bản copy-và-encrypt ở userspace trên
đường response trừ khi bạn cố tình dùng kTLS, và đó là một mặc định bình
thường, chấp nhận được — đừng coi việc mất zero-copy dưới TLS là một bug.

## Practice
1. Viết một static file server nhỏ dùng `libc::sendfile` thô và benchmark
   nó so với một vòng lặp read+write ngây thơ bằng `wrk` cho một file
   lớn.
2. Implement relay socket-tới-socket qua `splice` cho một TCP proxy trần
   và xác nhận (qua `strace`) không có copy buffer userspace nào xảy ra.
3. `mmap` một file, đọc từ mapping, rồi truncate file đó từ một process
   khác và quan sát `SIGBUS`.
4. Dùng vectored write (`IoSlice`) trong `labs/04-static-server` để ghi
   một response header và body trong một syscall thay vì gộp buffer.
5. Đọc thêm về hỗ trợ kTLS trong `rustls`/crate `ktls` và viết một ghi
   chú ngắn về việc có đáng theo đuổi cho `proxy` hay không.
