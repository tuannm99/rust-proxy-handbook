# Bộ công cụ Debug

Các công cụ để trả lời "chương trình của mình thực sự đang làm gì?" mà
không phải đoán. Phần lớn thời gian tự học bị mất khi làm proxy không nằm
ở viết code — mà ở nhìn chằm chằm vào code trông đúng trong khi nó chạy
sai. Mỗi công cụ ở đây thay việc nhìn chằm chằm bằng bằng chứng. Được tham
chiếu từ quy trình "khi bị kẹt" của
[`00-introduction/03-study-loop.md`](../00-introduction/03-study-loop.md).

## What to learn

### Bắt đầu từ phương pháp, không phải công cụ
Debug là một vòng lặp: đặt giả thuyết bug nằm ở đâu, chọn công cụ rẻ nhất
để xác nhận hoặc loại bỏ nó, chạy, lặp lại. Cách làm sai là ngược lại —
sửa code ngẫu nhiên cho tới khi triệu chứng dịch chuyển. Hai thói quen
giúp vòng lặp nhanh: **tái hiện trước** (một bug bạn không kích hoạt được
theo ý thì không sửa được một cách chắc chắn), và **thu nhỏ bản tái hiện**
tới khi nó đủ nhỏ để nguyên nhân không còn chỗ trốn.

### Góc nhìn của chính chương trình: panic, log, `tracing`
`RUST_BACKTRACE=1` biến thông báo panic một dòng thành một stack trace;
đây là thứ đầu tiên nên set khi có gì đó crash. Với mọi thứ khác, log có
cấu trúc tốt hơn `println!`: `tracing` với một filter kiểu `RUST_LOG` cho
phép bật chi tiết cho một module mà không bị ngập trong phần còn lại
([`08-observability/01-logging.md`](../08-observability/01-logging.md)).

```rust
tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();
// RUST_LOG=reverse_proxy=debug,hyper=info cargo run -p reverse-proxy
```
Gotcha: một dòng log "sent response" chứng minh code của bạn đã *gọi*
write, không chứng minh byte đã tới client. Với mọi thứ liên quan tới
network, xác nhận bằng một công cụ quan sát trên dây.

### Góc nhìn trên dây: `curl -v`, `nc`, `ss`, `tcpdump`
- `curl -v` (thêm `--http2` hoặc `--http1.1` để ép version) cho thấy chính xác header request và response — việc kiểm tra đầu tiên cho mọi bug HTTP.
- `nc` (netcat) gửi byte gõ tay hoặc byte lỗi, thứ `curl` sẽ không gửi. Không thể thiếu khi test một parser với input hỏng.
- `ss -tanp` liệt kê mọi socket kèm trạng thái (`ESTAB`, `TIME_WAIT`, `CLOSE_WAIT`...) và process sở hữu. Một đống `CLOSE_WAIT` nghĩa là *code của bạn* không đóng những kết nối mà phía bên kia đã đóng.
- `tcpdump -i lo -A port 8080` (hoặc Wireshark trên bản capture) cho thấy packet thật: byte có được gửi không, theo thứ tự nào, và ai đóng trước.

```text
curl -v --http1.1 http://127.0.0.1:8080/        # server đã trả lời gì?
printf 'GET / HTTP/1.1\r\n\r\n' | nc 127.0.0.1 8080   # một request mà curl sẽ từ chối gửi
ss -tanp | grep 8080                              # ai giữ socket nào, ở trạng thái nào?
```

### Góc nhìn của kernel: `strace`
`strace -f -e trace=network,read,write -p <pid>` cho thấy mọi syscall
process thực hiện. Nó trả lời những câu mà không log nào trả lời được:
process đang block trong `epoll_wait` (rảnh, đang chờ) hay đang quay vòng
trên `accept` trả về `EMFILE` ([`07-security/09-ddos.md`](../07-security/09-ddos.md))? `write` có trả về
ít byte hơn yêu cầu không (một short write, [`01-network/08-tcp.md`](../01-network/08-tcp.md))?
`strace -c` cho bảng tổng hợp số syscall, cách nhanh nhất để thấy cái gì
chiếm ưu thế.

### Góc nhìn của runtime: `tokio-console`
Với bug đặc thù async — một task không bao giờ thức dậy, một worker bị
kẹt vì code blocking — `tokio-console` hiển thị mọi task, nó đã rảnh bao
lâu, và mỗi lần poll mất bao lâu ([`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md)). Một task
có poll time rất lớn đang block một worker thread; một task rảnh mãi mà
không có hoạt động waker nào là một lost wakeup ([`04-runtime/02-waker.md`](../04-runtime/02-waker.md)).

### Chạy từng bước: `rust-gdb` / `rust-lldb`
Debugger tỏa sáng với bug logic trong code đồng bộ — một state machine
parser rẽ sai nhánh. Build không tối ưu (profile `dev` mặc định), chạy
dưới `rust-gdb target/debug/<bin>`, đặt breakpoint ở một hàm, xem biến.
Gotcha: chạy từng bước qua code async rất khổ, vì luồng thực thi nhảy giữa
các task ở mỗi `.await`; với bug async, `tracing` và `tokio-console`
thường đưa bạn tới đích nhanh hơn.

### Hiệu năng và bộ nhớ: `perf`, flamegraph, heap profiler
Khi bug là "quá chậm" hay "bộ nhớ cứ tăng", đo trước khi đổi bất cứ thứ
gì. `cargo flamegraph` (bọc `perf`) cho thấy thời gian CPU đi đâu
([`08-observability/04-profiling.md`](../08-observability/04-profiling.md)); một heap profiler như `dhat` (dạng
crate) hay `heaptrack` cho thấy ai allocate cái gì, và có bao giờ được
free không. RSS tăng dưới tải ổn định hoặc là leak hoặc là một buffer
không giới hạn — heap profile cho bạn biết là cái nào.

### Undefined behavior: Miri và sanitizer
Nếu code có `unsafe`, một test pass chứng minh được rất ít.
`cargo +nightly miri test` bắt được truy cập ngoài biên, use-after-free,
và aliasing sai mà phần cứng thật âm thầm bỏ qua
([`03-rust/03-unsafe.md`](../03-rust/03-unsafe.md), [`12-testing/04-ci-tooling.md`](04-ci-tooling.md)).

## Practice
Làm theo thứ tự này.

1. Trong [`labs/00-tcp-server`](../../labs/00-tcp-server), chạy server dưới `strace -f -e trace=network` trong khi kết nối bằng `nc`. **Xong khi** bạn chỉ ra được các syscall `accept`, `read`, và `write` cho một dòng được echo.
2. Capture một vòng echo bằng `tcpdump -i lo -A port <port>`. **Xong khi** bạn xác định được bắt tay, các packet dữ liệu, và phía nào gửi `FIN` đầu tiên.
3. Cố tình để một kết nối xử lý dở (ngừng đọc từ một client) và tìm nó bằng `ss -tanp`. **Xong khi** bạn giải thích được socket đang kẹt ở trạng thái nào.
4. Thêm `tracing` với environment filter vào [`labs/00-tcp-server`](../../labs/00-tcp-server). **Xong khi** bạn bật tắt được debug log theo từng kết nối bằng `RUST_LOG` mà không cần compile lại.
5. Chèn một `std::thread::sleep` vào một connection handler và tìm ra nó bằng `tokio-console`. **Xong khi** console chỉ ra task gây lỗi qua poll time của nó.
6. Tạo flamegraph của [`labs/00-tcp-server`](../../labs/00-tcp-server) dưới tải. **Xong khi** bạn gọi tên được hàm chiếm nhiều thời gian CPU nhất và nói được điều đó có hợp lý không.
