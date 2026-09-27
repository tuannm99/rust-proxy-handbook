# Profiling
perf, flamegraph, bpftrace.

## What to learn
### perf + flamegraph cho thời gian CPU
`perf record -F 99 -p <pid> -g -- sleep 30` sample call stack 99 lần/giây;
`cargo flamegraph` (bọc `perf` + inferno) biến nó thành một flamegraph
trực quan nơi frame càng rộng = càng nhiều thời gian CPU. Với một proxy,
hãy chờ thấy thời gian chia ra giữa handshake TLS/crypto, parse HTTP, và
syscall (read/write/epoll_wait) — nếu flamegraph bị chi phối bởi các frame
của allocator (`malloc`/`free`), đó thường là dấu hiệu của việc clone
không cần thiết header/body trên mỗi request.
Build với `debug = true` dưới `[profile.release]` để symbol resolve được,
nếu không flamegraph vô dụng.

Gotcha: chỉ có symbol thôi chưa đủ — `perf -g` còn cần *unwind* stack, và
Rust ở chế độ release mặc định bỏ frame pointer, tạo ra các flamegraph chỉ
sâu một frame và vô dụng theo một cách khác. Hãy build với
`RUSTFLAGS="-C force-frame-pointers=yes"`, hoặc dùng `--call-graph dwarf`
(chính xác hơn, file perf.data lớn hơn nhiều, overhead cao hơn).

Gotcha: `perf` cần quyền. `kernel.perf_event_paranoid` thường mặc định
một giá trị chặn việc profile các process không có đặc quyền, và bên
trong container bạn thường cần `CAP_PERFMON` (hoặc `--privileged`) cùng
một kernel tương thích. Hãy giải quyết chuyện này *trước* khi có sự cố cần
một profile.

### Async làm flamegraph nói dối về causality
Đây là gotcha quan trọng nhất cho codebase này, và nó khiến gần như ai
cũng bất ngờ lần đầu tiên.

Một flamegraph cho thấy call stack tại thời điểm sample. Trong Rust async,
stack đó là `worker thread → executor loop → poll() → poll của future của
bạn`. Nó **không** phải đường đi logic của một request. Một future await
bốn thứ tuần tự sẽ xuất hiện như bốn stack `poll` không liên quan bên dưới
executor, không có quan hệ cha-con giữa chúng — cấu trúc logic bạn quan
tâm nằm trong *state machine*, không nằm trên stack.

Hai hệ quả:
- **Việc quy kết bị sai.** Bạn không thể đọc "lệnh gọi upstream này gây ra
  allocation đó" từ đồ thị, vì phần tiếp diễn của lệnh gọi upstream được
  poll từ executor, không phải từ code đã await nó.
- **Việc chờ đợi vô hình.** Profiling CPU sample các thread đang *chạy
  trên CPU*. Một request chờ 200ms trên một upstream đóng góp 0 vào
  flamegraph. Nếu proxy của bạn chậm vì nó đang chờ, flamegraph sẽ trông
  hoàn toàn khỏe mạnh và không nói cho bạn biết gì.

Hãy dùng trace ([`08-observability/03-tracing.md`](03-tracing.md)) cho việc quy kết
causality và latency; dùng flamegraph cho "cái gì đang đốt CPU." Chúng trả
lời các câu hỏi khác nhau, và nhầm lẫn giữa chúng lãng phí rất nhiều thời
gian.

### Off-CPU time và tokio-console
Vì profile CPU che giấu việc chờ đợi, bạn cần một công cụ thứ hai cho nửa
còn lại. Off-CPU profiling (qua `bpftrace` trên các tracepoint của
scheduler, hoặc `offcputime` từ bcc) đo nơi các thread *block*, chính là
nơi latency của một proxy thường nằm ở đó.

Với riêng async, `tokio-console` là công cụ nhắm đúng: nó cho thấy số lần
poll theo từng task, thời lượng poll, và — tín hiệu quan trọng — các task
mà mỗi lần poll mất nhiều thời gian. Một lần poll chạy nhiều mili-giây là
một task đang chặn executor thread ([`03-rust/05-async.md`](../03-rust/05-async.md)'s cooperative
scheduling), làm khựng mọi connection khác trên worker đó. Thủ phạm phổ
biến trong một proxy: một lần đọc file đồng bộ
([`05-http-stack/05-static.md`](../05-http-stack/05-static.md)), một lần nén lớn
([`05-http-stack/06-compression.md`](../05-http-stack/06-compression.md)), regex trên một body lớn
([`07-security/06-waf.md`](../07-security/06-waf.md)), hoặc một lần ghi log đồng bộ
([`08-observability/01-logging.md`](01-logging.md)).

Gotcha: một executor thread bị chặn xuất hiện như *latency trên các
request không liên quan*, đó là lý do vì sao nó khó chẩn đoán chỉ từ dữ
liệu ở mức request — request chậm và các request bị ảnh hưởng là các
request khác nhau.

### bpftrace cho các câu hỏi ở mức syscall/latency mà perf không trả lời được
`perf` cho bạn biết thời gian CPU đi đâu; `bpftrace` (xây trên eBPF) trả
lời "mỗi syscall `read()` block bao lâu" hay "có bao nhiêu lần retransmit
TCP xảy ra" mà không cần sửa binary. Ví dụ: histogram latency của
`accept()` để bắt một vấn đề backlog của listener vô hình trong metrics ở
mức ứng dụng.
```
bpftrace -e 'tracepoint:syscalls:sys_enter_read /pid == $1/ { @start[tid] = nsecs; }
             tracepoint:syscalls:sys_exit_read /@start[tid]/ { @read_ns = hist(nsecs - @start[tid]); delete(@start[tid]); }'
```

### Thời gian của một proxy Rust thực sự đi đâu
1. Syscall (epoll_wait/read/write) — xem [`02-linux/07-epoll.md`](../02-linux/07-epoll.md),
   [`02-linux/11-zerocopy.md`](../02-linux/11-zerocopy.md) để biết cách giảm những cái này.
2. Allocation — mỗi lần clone `Vec<u8>`/`String` trên hot path đều tốn chi
   phí; profile bằng `heaptrack` hoặc `dhat` (qua crate `dhat`) song song
   với profiling CPU.
3. TLS — chi phí CPU của handshake là có thật ở connection churn cao (các
   connection ngắn ngày liên tục handshake lại); session resumption
   ([`01-network/13-tls.md`](../01-network/13-tls.md)) quan trọng hơn việc micro-optimize parser.
Gotcha: profile một build debug gần như vô nghĩa — luôn profile
`--release`, và profile dưới tải đồng thời thực tế (xem
[`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)), không phải một request curl đơn lẻ.

Gotcha: allocation hiện ra trong một profile CPU dưới dạng frame
`malloc`/`free`, nhưng *cái giá* thật của một pattern allocation tệ thường
là fragmentation và RSS tăng dần qua nhiều ngày
([`14-memory/06-fragmentation.md`](../14-memory/06-fragmentation.md)), điều mà không profile 30 giây nào thấy
được. Hãy theo dõi allocated-vs-resident như một gauge song song.

### Continuous profiling tốt hơn profiling trong lúc sự cố
Một profile chụp trong một load test phản ánh traffic mix của load
generator của bạn, không phải của production — và tới lúc bạn SSH vào một
máy để chạy `perf`, cửa sổ thú vị có thể đã trôi qua. Continuous profiler
(`pprof-rs` expose một endpoint `/debug/pprof`, được scrape bởi Parca hay
Pyroscope) sample ở tần suất thấp mọi lúc, để bạn có thể xem một profile
từ *thứ Ba tuần trước lúc 03:14* khi sự cố thực sự xảy ra.

Gotcha: expose endpoint đó trên listener nội bộ, không phải listener công
khai (cùng lý do như `/metrics` trong [`08-observability/02-metrics.md`](02-metrics.md)) —
một endpoint profiling vừa là một rò rỉ thông tin vừa là một chi phí CPU
mà bất kỳ ai cũng có thể kích hoạt.

### Đọc một flamegraph mà không tự lừa mình
Rộng ≠ chậm-mỗi-lần-gọi, rộng = tổng thời gian *toàn bộ* qua mọi sample —
một hàm nhanh được gọi liên tục có thể trông giống hệt một hàm chậm được
gọi hiếm khi. Hãy đối chiếu với các bộ đếm `perf stat` (instructions,
cache-misses, context-switches) trước khi kết luận "hàm này là bottleneck."

Gotcha: cũng kiểm tra xem bạn có thực sự bị giới hạn bởi CPU hay không
trước khi tối ưu CPU. Nếu proxy đang ở 20% CPU và latency vẫn tệ, frame
rộng nhất trong flamegraph không liên quan gì — câu trả lời nằm ở off-CPU
time, lock contention, hoặc một upstream. Các chỉ số IPC và context-switch
của `perf stat`, cộng với utilization tổng thể, cho bạn biết bạn đang ở
chế độ nào.

## Practice
Xây dựng theo thứ tự sau.

1. Cấu hình build để profile (`debug = true` trong release,
   `force-frame-pointers`) và xác nhận quyền. **Xong khi** `cargo
   flamegraph` nhắm vào `proxy` dưới tải tạo ra một đồ thị với các frame
   Rust sâu, có symbol — không phải một đồ thị phẳng.
2. Profile dưới tải đồng thời thực tế từ [`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md).
   **Xong khi** bạn có thể nêu tên ba frame rộng nhất và phân loại mỗi cái
   là syscall (đúng dự kiến), allocation (có thể sửa), hoặc logic (đúng
   dự kiến).
3. Đưa vào một `header.clone()` cố ý trên mỗi request ở hot path và
   profile lại. **Xong khi** bạn thấy nó xuất hiện — điều này giúp hiệu
   chỉnh mức độ dễ thấy của một regression cỡ đó trong thực tế.
4. Chứng minh vấn đề quy kết của async. **Xong khi** bạn có thể cho thấy
   một upstream chậm (tiêm 200ms độ trễ) không làm rộng bất cứ thứ gì
   trong flamegraph, và giải thích chỉ từ profile vì sao lại như vậy.
5. Chạy `tokio-console` nhắm vào [`proxy`](../../proxy). **Xong khi** bạn có thể xác định
   task có thời lượng poll đơn lẻ dài nhất — rồi thêm một thao tác đồng
   bộ 10ms bên trong một handler và xem nó trở thành thủ phạm tệ nhất.
6. Đo thiệt hại của một lần poll bị block. **Xong khi** bạn có thể cho
   thấy p99 latency tăng lên cho các request đồng thời *khác* trong khi
   một handler đang block, và giảm trở lại khi bạn chuyển công việc đó
   sang `spawn_blocking`.
7. Dùng `bpftrace` để lập histogram latency từ `accept()` tới `read()`
   đầu tiên dưới tải. **Xong khi** bạn có thể so sánh nó với
   `proxy_request_duration_seconds` và nói xem chúng có khớp nhau không —
   một khoảng chênh nghĩa là thời gian đang bị tiêu tốn trước khi
   instrumentation của bạn bắt đầu.
8. So sánh profile TLS với profile plaintext. **Xong khi** bạn có thể định
   lượng chi phí CPU của handshake trên mỗi connection và cho thấy nó
   giảm xuống khi session resumption được bật ([`01-network/13-tls.md`](../01-network/13-tls.md)).
9. (Mở rộng) Expose `pprof-rs` trên listener nội bộ và chụp profile liên
   tục trong một chaos test ([`12-testing/03-chaos.md`](../12-testing/03-chaos.md)). **Xong khi** bạn
   có thể lấy lại profile từ đúng phút một fault được tiêm vào, sau khi
   sự việc đã xảy ra.
