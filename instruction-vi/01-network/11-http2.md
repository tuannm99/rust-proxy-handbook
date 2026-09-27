# HTTP/2

Frame, stream, HPACK, multiplexing.

## What to learn

### Frame và tầng binary framing
HTTP/2 thay thế wire format dạng văn bản của /1.1 bằng các frame nhị
phân, có kiểu, có tiền tố độ dài (`HEADERS`, `DATA`, `SETTINGS`,
`WINDOW_UPDATE`, `RST_STREAM`, `GOAWAY`, ...) được multiplex trên một kết
nối TCP duy nhất. Mỗi frame thuộc về một stream ID (0 = tầng kết nối). Một
proxy chấm dứt HTTP/2 phải nói tường minh tầng frame này — đây không phải
"HTTP với header xịn hơn", đây là một wire protocol thực sự khác biệt.

### Stream và multiplexing
Nhiều cuộc trao đổi request/response logic (stream) chạy đồng thời trên
một kết nối TCP, mỗi cái được flow-control độc lập. Đây là cái sửa nhu cầu
6 kết nối TCP song song cho mỗi origin của HTTP/1.1. Gotcha: multiplexing
giải quyết head-of-line blocking ở *tầng kết nối* nhưng không giải quyết
HOL blocking ở *tầng TCP* — một TCP segment bị mất vẫn khựng mọi stream
trên kết nối đó cho tới khi nó được retransmit (đây chính xác là thứ
HTTP/3 trên QUIC sửa, xem [`12-http3.md`](12-http3.md)).

### Nén header HPACK
Header được nén bằng HPACK: một bảng tĩnh chứa các cặp tên/giá trị header
phổ biến, một bảng động được xây dần theo từng kết nối, và mã hóa Huffman
cho các giá trị literal. Vì bảng động có trạng thái theo từng kết nối, một
proxy giải mã HPACK ở phía client-facing và mã hóa lại cho phía upstream
phải duy trì hai trạng thái HPACK độc lập — bạn không thể chỉ relay frame
thô nếu header đang bị chỉnh sửa.

### Flow control
Cả flow control ở tầng stream lẫn tầng kết nối đều dùng cơ chế
`WINDOW_UPDATE` dựa trên tín dụng — bên gửi không được gửi nhiều `DATA`
hơn window đã công bố của bên nhận. Một proxy forward một response body
lớn từ một upstream nhanh tới một client chậm phải tôn trọng window flow
control của client và áp backpressure lên việc đọc từ upstream, không
buffer vô hạn.

### Vòng đời stream và giới hạn concurrency
Một stream đi qua một state machine nhỏ: idle → open (khi có `HEADERS`) →
half-closed (một bên đã gửi `END_STREAM`) → closed (cả hai bên xong, hoặc
`RST_STREAM`). Stream ID do client khởi tạo là số lẻ, do server khởi tạo
là số chẵn, và ID phải tăng nghiêm ngặt — một stream ID không bao giờ được
tái sử dụng, nên một kết nối sống lâu cuối cùng sẽ cạn kiệt không gian
31-bit và phải được retire bằng `GOAWAY`.

`SETTINGS_MAX_CONCURRENT_STREAMS` giới hạn số stream có thể mở cùng lúc.
Đây là setting quan trọng nhất với một proxy: nó là giới hạn concurrency
theo từng kết nối, và nó ánh xạ trực tiếp tới việc một kết nối client có
thể tạo ra bao nhiêu request upstream.

Gotcha: `GOAWAY` mang stream ID cuối cùng bên gửi sẽ xử lý, đây chính là
thứ khiến graceful shutdown ([`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md))
khả thi — các stream dưới nó hoàn tất, các stream trên nó client có thể
an toàn retry ở nơi khác. Một proxy đóng kết nối TCP mà không có `GOAWAY`
biến một lần drain sạch thành các lỗi hiển thị với client.

### HPACK có trạng thái, và đó là mối nguy hiểm
Bảng động nghĩa là việc giải mã header phụ thuộc vào mọi header frame
trước đó trên kết nối đó. Ba hệ quả một proxy phải xử lý:

**Thứ tự là bắt buộc.** Frame `HEADERS` phải được giải mã theo đúng thứ
tự nhận được, dù các stream vốn dĩ độc lập với nhau — giải mã stream 7
trước stream 5 làm hỏng bảng cho cả hai. Một frame `HEADERS` theo sau bởi
các frame `CONTINUATION` là atomic: không gì khác được xen vào.

**Bảng là một cam kết bộ nhớ.** `SETTINGS_HEADER_TABLE_SIZE` giới hạn nó
theo từng kết nối, nhưng một proxy giữ hàng nghìn kết nối nhân con số đó
lên theo số lượng kết nối. Thực ra là hai bảng cho mỗi kết nối — trạng
thái decode cho phía client, trạng thái encode cho phía upstream.

**Decompression là một vector khuếch đại.** Một khối header đã nén nhỏ có
thể phình to khủng khiếp ("HPACK bomb"), nên hãy giới hạn kích thước header
*đã giải nén*, không chỉ kích thước frame. Đây là phiên bản HTTP/2 của
cùng kỷ luật giới hạn như [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md).

### Flow control trong một proxy: backpressure thực sự đổ vào đâu
Có hai window độc lập: theo stream và theo kết nối. Bên gửi phải tôn
trọng cả hai, nên một stream còn window vẫn không thể gửi nếu window của
kết nối đã cạn. Window bắt đầu ở 65535 byte và chỉ lớn lên qua
`WINDOW_UPDATE`.

Vấn đề đặc thù của proxy: một upstream nhanh nuôi một client chậm. Hành vi
đúng là để window chưa được bổ sung của client chặn bạn đọc thêm từ
upstream — chuỗi backpressure phải chạy từ đầu tới cuối. Failure mode là
buffer response trong bộ nhớ vì phía đọc có dữ liệu sẵn sàng, biến một
client chậm thành tăng trưởng bộ nhớ vô hạn.

Gotcha: khi chặng upstream là HTTP/1.1, không có window nào để lan
truyền. Backpressure ở đó là receive window của TCP — bạn dừng đọc socket
upstream, send buffer của nó đầy lên, và kernel ngừng ACK. Điều đó hoạt
động, nhưng chỉ khi bạn thực sự dừng gọi `read()`; một vòng lặp
đọc-hăm-hở-vào-một-`Vec` sẽ âm thầm vô hiệu hóa nó.

### Các tấn công đặc thù của HTTP/2
Chủ đề lặp lại là các frame rẻ để gửi nhưng gây ra công việc tốn kém cho
server, và một proxy phải giới hạn từng loại:

- **Rapid Reset (CVE-2023-44487)** — mở một stream và ngay lập tức
  `RST_STREAM` nó. Stream đó không còn tính vào
  `MAX_CONCURRENT_STREAMS`, nên một client có thể tạo ra *công việc* vô
  hạn mà không bao giờ vượt giới hạn concurrency. Cách giảm thiểu là theo
  dõi và rate-limit số reset theo từng kết nối, đóng các kết nối vượt quá
  ngưỡng đó.
- **Settings/ping flood** — `SETTINGS` và `PING` đòi hỏi phải được ack;
  làm ngập chúng buộc server phải sinh response vô tận.
- **Empty `DATA` frame flood** — các frame độ dài 0 tốn công parsing và
  không tốn window flow-control nào, nên window không bao giờ điều tiết
  chúng.

Pattern chung: bất kỳ frame nào rẻ với client và chưa được tính vào một
giới hạn có sẵn đều cần rate limit riêng của nó. `h2` đã sửa từng cái
trong số này, đây là một luận điểm cụ thể cho lập trường trong
[`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) — dùng implementation được bảo trì.

### Rust nằm ở đâu trong bức tranh này
`h2` (được `hyper` dùng nội bộ khi feature `http2` bật) là implementation
HTTP/2 trên thực tế trong hệ sinh thái Rust; auto server builder của
`hyper-util` trong [`labs/02-http-server`](../../labs/02-http-server)/`reverse-proxy` negotiate
HTTP/1.1 vs HTTP/2 qua ALPN (xem [`13-tls.md`](13-tls.md)) nên bạn có được điều này
"miễn phí" một khi TLS đã được cắm vào, nhưng bạn vẫn nên giải thích được
chuyện gì đang xảy ra bên dưới abstraction đó.

## Practice

1. Capture một kết nối HTTP/2 bằng Wireshark (hoặc `nghttp -v`) và xác
   định ít nhất 4 loại frame khác nhau trên đường truyền.
2. Bật feature `http2` trên hyper server trong [`labs/02-http-server`](../../labs/02-http-server) và
   xác nhận qua `curl --http2` rằng nó negotiate HTTP/2 qua TLS (ALPN).
3. Trong [`labs/08-http2`](../../labs/08-http2), gửi hai request đồng thời trên cùng một kết nối
   HTTP/2 bằng `curl --http2 -v` và xác nhận cả hai stream hoàn tất trên
   một kết nối TCP (kiểm tra bằng `ss` hoặc `lsof`); rồi thu nhỏ window
   ban đầu và quan sát stream thứ hai bị khựng lại vì flow control.
4. Đọc docs về flow control của crate `h2` và giải thích, bằng lời của
   bạn, chuyện gì xảy ra nếu kết nối phía client-facing của proxy bạn là
   HTTP/2 nhưng kết nối upstream là HTTP/1.1 — sự lệch pha flow-control
   được hấp thụ ở đâu?
5. Chứng minh backpressure từ đầu tới cuối trong [`labs/08-http2`](../../labs/08-http2): phục vụ
   một response lớn cho một client đọc chậm (hoặc ngừng đọc hẳn), và xác
   nhận bộ nhớ của proxy bạn giữ phẳng thay vì buffer toàn bộ body.
6. Đặt `SETTINGS_MAX_CONCURRENT_STREAMS` thấp (ví dụ 2), mở nhiều stream
   hơn số đó, và quan sát peer bị giữ lại thế nào; tái hiện Rapid Reset
   bằng cách mở rồi reset stream ngay lập tức trong một vòng lặp và xác
   nhận riêng giới hạn concurrency *không* chặn được bạn; rồi kích hoạt
   `GOAWAY` bằng cách tắt server giữa chừng một request và xác nhận các
   stream đang xử lý dở dưới last-stream-ID hoàn tất thay vì lỗi.
