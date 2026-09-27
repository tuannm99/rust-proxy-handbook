# TCP Stack của Kernel

`01-network/08-tcp.md` nói về TCP từ phía ứng dụng (handshake, byte-stream
framing). File này nói về những gì kernel đang làm bên dưới — state machine
và các buffer mà số lượng kết nối cùng traffic pattern của một proxy thực
sự gây áp lực lên.

## What to learn

### State machine, và hai backlog quan trọng với một listener
Một listening socket có hai queue tách biệt, và nhầm lẫn giữa chúng là
nguồn gốc phổ biến của bug "sao lại rớt kết nối khi tải cao":
- **SYN backlog** (các kết nối half-open, ở trạng thái `SYN_RCVD`, đang
  chờ ACK cuối cùng): kích thước quyết định bởi
  `net.ipv4.tcp_max_syn_backlog`. Đây là thứ mà một SYN flood làm cạn kiệt
  (`07-security/09-ddos.md`).
- **Accept backlog** (các kết nối đã thiết lập xong, đang chờ process bạn
  gọi `accept()`): kích thước là giá trị nhỏ hơn giữa tham số `backlog`
  của `listen()` và `net.core.somaxconn`. Một proxy có accept loop bị
  chậm lại (đang block làm việc khác) sẽ làm đầy *queue này* thay vào đó,
  và kết nối mới bị từ chối (hoặc âm thầm bị drop, tùy
  `tcp_abort_on_overflow`) dù SYN backlog vẫn ổn.

### Buffer autotuning và chi phí bộ nhớ của nó ở quy mô lớn
`net.ipv4.tcp_rmem`/`tcp_wmem` cho phép kernel tăng buffer gửi/nhận của
mỗi socket lên tới một mức tối đa khi throughput yêu cầu — tốt cho một kết
nối throughput cao đơn lẻ, nhưng nhân mức tối đa đó với số lượng kết nối:
một proxy giữ 100.000 kết nối gần-như-idle, mỗi cái chỉ 64 KB buffer
khiêm tốn, đã là 6.4 GB bộ nhớ kernel không bao giờ hiện trong RSS accounting
của chính process bạn (`02-linux/09-memory.md`). Theo dõi
`/proc/net/sockstat` và `ss -m`, không chỉ metric bộ nhớ của process bạn,
khi chẩn đoán vấn đề bộ nhớ ở số lượng kết nối cao.

### TIME_WAIT: cái giá của việc là bên đóng trước
Bên gửi `FIN` đầu tiên (active close) sẽ giữ 4-tuple của kết nối ở trạng
thái `TIME_WAIT` trong `2 * MSL` (thường ~60s trên Linux) sau khi đóng, để
phòng ngừa các packet trễ lạc từ kết nối cũ bị gán nhầm cho một kết nối
mới tái sử dụng cùng tuple. Một proxy mở và đóng kết nối upstream ngắn hạn
theo từng request sẽ tích lũy entry `TIME_WAIT` đủ nhanh để làm cạn kiệt
ephemeral port hoặc connection-tracking table. Các cách giảm thiểu, theo
thứ tự mức độ giải quyết tận gốc thay vì chỉ che đi triệu chứng: tái sử
dụng kết nối upstream (pooling ở `06-proxy/01-upstream.md`, tránh việc mở
và đóng hoàn toàn), để *upstream* là bên đóng trước khi có lựa chọn, và
chỉ dùng `SO_REUSEADDR`/tinh chỉnh `net.ipv4.tcp_tw_reuse` như phương án
cuối cùng cho phía client-facing.

### Gotcha: RST vs FIN, và những gì ứng dụng của bạn thực sự thấy
Một `RST` đột ngột (connection reset, từ việc peer crash, firewall, hoặc
một `close()` tường minh khi vẫn còn dữ liệu trong receive buffer) hiện ra
với một read/write async trong Rust dưới dạng `io::Error` với
`ErrorKind::ConnectionReset`, khác với một EOF sạch từ một lần đóng
`FIN` thông thường. Code coi mọi lỗi kết thúc kết nối như nhau sẽ mất khả
năng phân biệt "client cúp máy bình thường" với "có gì đó đang chủ động
reset kết nối của chúng ta", điều này quan trọng cho cả việc debug lẫn
việc phát hiện một số loại traffic scanning/tấn công.

## Practice
1. Theo dõi `ss -tan state time-wait | wc -l` trong khi tạo tải vào
   `labs/05-reverse-proxy` với mỗi request mở một kết nối upstream mới;
   sau đó thêm connection pooling (`06-proxy/01-upstream.md`) và so sánh
   con số.
2. Cố tình làm chậm một accept loop (sleep trước mỗi `accept()`) và làm
   tràn accept backlog bằng một đợt kết nối dồn dập; quan sát sự khác biệt
   so với việc làm tràn SYN backlog bằng một đợt flood kết nối half-open
   (ví dụ qua raw SYN, trong một môi trường lab bạn kiểm soát).
3. Theo dõi `/proc/net/sockstat` và `ss -m` trong khi giữ mở hơn 10.000
   kết nối idle với `labs/00-tcp-server`; so sánh bộ nhớ buffer phía
   kernel với những gì RSS của chính process bạn báo cáo.
4. Kích hoạt cả một lần đóng sạch lẫn một `RST` trên một kết nối code bạn
   đang giữ mở, và xác nhận bạn có thể phân biệt hai loại `io::Error` đó
   trong code xử lý của mình.
