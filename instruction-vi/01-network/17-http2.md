# HTTP/2

Frame, stream, HPACK, multiplexing.

## What to learn

### Frame và lớp binary framing
HTTP/2 thay wire format dạng văn bản của /1.1 bằng các frame nhị phân, có kiểu, có tiền tố độ dài (`HEADERS`, `DATA`,
`SETTINGS`, `WINDOW_UPDATE`, `RST_STREAM`, `GOAWAY`, ...). Các frame được multiplex trên một TCP connection duy nhất.
Mỗi frame thuộc về một stream ID (0 nghĩa là mức connection).

Một proxy kết thúc HTTP/2 phải nói rõ ràng lớp frame này. Đây không phải "HTTP với header xịn hơn". Đây là một wire
protocol thật sự khác.

### Stream và multiplexing
Nhiều cuộc trao đổi request/response logic (stream) chạy đồng thời trên một TCP connection, mỗi cái có flow control độc
lập. Cách này sửa nhu cầu mở 6 TCP connection song song cho mỗi origin của HTTP/1.1.

Gotcha: multiplexing giải quyết head-of-line blocking ở *mức connection*, nhưng không giải quyết HOL blocking ở *mức TCP*.
Một TCP segment bị mất vẫn làm mọi stream trên connection đó đứng lại cho đến khi nó được retransmit. Đây chính xác là
thứ HTTP/3 trên QUIC sửa, xem [`18-http3.md`](18-http3.md).

### Nén header bằng HPACK
Header được nén bằng HPACK, gồm:
- một bảng tĩnh chứa các cặp tên/giá trị header phổ biến,
- một bảng động được xây dần theo từng connection,
- mã hóa Huffman cho các giá trị literal.

Vì bảng động có state theo từng connection, một proxy giải mã HPACK ở phía client rồi mã hóa lại cho phía upstream phải
giữ hai state HPACK độc lập. Nếu bạn sửa header, bạn không thể chỉ relay frame thô.

### Flow control
Flow control ở cả mức stream và mức connection đều dùng cơ chế `WINDOW_UPDATE` dựa trên credit: bên gửi không được gửi
nhiều `DATA` hơn window mà bên nhận đã công bố.

Một proxy forward response body lớn từ upstream nhanh tới client chậm phải tôn trọng window của client, và tạo
backpressure lên việc đọc từ upstream. Không được buffer vô hạn.

### Vòng đời stream và giới hạn concurrency
Một stream đi qua một state machine nhỏ: idle → open (khi có `HEADERS`) → half-closed (một bên đã gửi `END_STREAM`) →
closed (cả hai bên xong, hoặc `RST_STREAM`).

- Stream ID do client khởi tạo là số lẻ, do server khởi tạo là số chẵn.
- ID phải tăng nghiêm ngặt, và không bao giờ được dùng lại. Vì vậy một connection sống lâu cuối cùng sẽ cạn không gian
  31-bit, và phải được retire bằng `GOAWAY`.

`SETTINGS_MAX_CONCURRENT_STREAMS` giới hạn số stream được mở cùng lúc. Đây là setting quan trọng nhất với proxy: nó là
giới hạn concurrency trên mỗi connection, và ánh xạ trực tiếp tới số request upstream mà một client connection có thể
tạo ra.

Gotcha: `GOAWAY` mang stream ID cuối cùng mà bên gửi sẽ xử lý. Đây là thứ làm cho graceful shutdown
([`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md)) khả thi. Các stream có ID bằng
hoặc nhỏ hơn thì được hoàn tất. Các stream có ID lớn hơn thì client có thể retry an toàn ở nơi khác. Nếu proxy đóng TCP
connection mà không gửi `GOAWAY`, một lần drain sạch sẽ biến thành lỗi mà client nhìn thấy.

### HPACK có state, và đó là mối nguy
Bảng động nghĩa là việc giải mã header phụ thuộc vào mọi header frame trước đó trên connection. Proxy phải xử lý ba hệ
quả:

**Thứ tự là bắt buộc.** Frame `HEADERS` phải được giải mã đúng thứ tự nhận được, dù các stream vốn độc lập nhau. Giải mã
stream 7 trước stream 5 sẽ làm hỏng bảng cho cả hai. Một frame `HEADERS` theo sau là các frame `CONTINUATION` là atomic:
không frame nào khác được chen vào giữa.

**Bảng chiếm bộ nhớ.** `SETTINGS_HEADER_TABLE_SIZE` giới hạn nó cho mỗi connection, nhưng proxy giữ hàng nghìn
connection, nên con số này bị nhân lên. Thực ra mỗi connection có hai bảng: state decode cho phía client, và state
encode cho phía upstream.

**Giải nén là một vector khuếch đại.** Một khối header nén nhỏ có thể phình ra rất lớn ("HPACK bomb"). Vì vậy hãy giới
hạn kích thước header *sau khi giải nén*, không chỉ kích thước frame. Đây là phiên bản HTTP/2 của cùng nguyên tắc đặt
giới hạn trong [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md).

### Flow control trong proxy: backpressure thực sự dồn về đâu
Có hai window độc lập: theo stream và theo connection. Bên gửi phải tôn trọng cả hai. Một stream còn window vẫn không
gửi được nếu window của connection đã cạn. Window bắt đầu ở 65535 byte và chỉ lớn thêm qua `WINDOW_UPDATE`.

Vấn đề đặc thù của proxy: upstream nhanh, client chậm. Cách làm đúng là để window chưa được bổ sung của client chặn bạn
đọc thêm từ upstream. Chuỗi backpressure phải chạy suốt từ đầu đến cuối. Cách làm sai là buffer response trong bộ nhớ
chỉ vì phía đọc có sẵn dữ liệu. Khi đó một client chậm biến thành bộ nhớ tăng không giới hạn.

Gotcha: khi chặng upstream là HTTP/1.1, không có window nào để chuyển tiếp. Backpressure ở đó là receive window của TCP:
bạn ngừng đọc socket upstream, send buffer của upstream đầy, kernel ngừng ACK. Cách này chạy được, nhưng chỉ khi bạn thật
sự ngừng gọi `read()`. Một vòng lặp đọc hết sức, đẩy vào một `Vec`, sẽ âm thầm vô hiệu hóa nó.

### Các tấn công đặc thù của HTTP/2
Chủ đề chung: có những frame rất rẻ để client gửi nhưng tốn công của server. Proxy phải giới hạn từng loại:

- **Rapid Reset (CVE-2023-44487)**: mở một stream rồi ngay lập tức `RST_STREAM` nó. Stream đó không còn tính vào
  `MAX_CONCURRENT_STREAMS`, nên client có thể tạo ra *công việc* vô hạn mà không bao giờ vượt giới hạn concurrency. Cách
  giảm thiểu: theo dõi và rate-limit số lần reset trên mỗi connection, và đóng connection vượt ngưỡng.
- **Settings/ping flood**: `SETTINGS` và `PING` đều phải được ack. Gửi dồn dập chúng buộc server sinh response vô tận.
- **Empty `DATA` frame flood**: frame độ dài 0 tốn công parse nhưng không tiêu window flow control nào, nên window không
  bao giờ làm chúng chậm lại.

Pattern chung: bất kỳ frame nào rẻ với client và chưa được tính vào một giới hạn sẵn có đều cần rate limit riêng. `h2`
đã sửa từng lỗi này. Đây là một lý do cụ thể cho lập trường trong
[`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md): hãy dùng implementation đang được bảo trì.

### Rust nằm ở đâu trong bức tranh này
`h2` (được `hyper` dùng bên trong khi feature `http2` được bật) là implementation HTTP/2 thực tế của hệ sinh thái Rust.
Auto server builder của `hyper-util` trong [`labs/02-http-server`](../../labs/02-http-server)/`reverse-proxy` chọn
HTTP/1.1 hay HTTP/2 qua ALPN (xem [`19-tls.md`](19-tls.md)). Vì vậy bạn có tính năng này "miễn phí" khi TLS đã được gắn
vào. Dù vậy, bạn vẫn nên giải thích được chuyện gì xảy ra bên dưới lớp trừu tượng đó.

Cấu hình nào của hyper tương ứng với khái niệm nào trong file này (stream, window, giới hạn header, Rapid Reset) nằm ở
[`05-http-stack/02-hyper.md`](../05-http-stack/02-hyper.md). Các công cụ để *nhìn thấy* frame và tạo tải multiplex
(`nghttp`, `h2load`) nằm ở [`12-testing/06-lab-environment.md`](../12-testing/06-lab-environment.md).

## Practice

1. Bắt một HTTP/2 connection bằng Wireshark (hoặc `nghttp -v`) và xác định ít nhất 4 loại frame khác nhau trên đường
   truyền.
2. Bật feature `http2` trên hyper server trong [`labs/02-http-server`](../../labs/02-http-server) và xác nhận qua
   `curl --http2` rằng nó thương lượng được HTTP/2 qua TLS (ALPN).
3. Trong [`labs/08-http2`](../../labs/08-http2), gửi hai request đồng thời trên cùng một HTTP/2 connection bằng
   `curl --http2 -v`. Xác nhận cả hai stream hoàn tất trên một TCP connection (kiểm tra bằng `ss` hoặc `lsof`). Sau đó
   thu nhỏ window ban đầu và quan sát stream thứ hai bị đứng lại vì flow control.
4. Đọc docs về flow control của crate `h2`, và giải thích bằng lời của bạn: nếu connection phía client của proxy là
   HTTP/2 nhưng connection upstream là HTTP/1.1 thì chuyện gì xảy ra? Sự lệch nhau về flow control được hấp thụ ở đâu?
5. Chứng minh backpressure đầu-cuối trong [`labs/08-http2`](../../labs/08-http2): phục vụ một response lớn cho một client
   đọc chậm (hoặc ngừng đọc hẳn), và xác nhận bộ nhớ của proxy giữ phẳng thay vì buffer toàn bộ body.
6. Đặt `SETTINGS_MAX_CONCURRENT_STREAMS` thấp (ví dụ 2), mở nhiều stream hơn số đó, và quan sát peer bị giữ lại thế nào.
   Tái hiện Rapid Reset bằng cách mở rồi reset stream ngay trong một vòng lặp, và xác nhận riêng giới hạn concurrency
   *không* chặn được bạn. Sau đó kích hoạt `GOAWAY` bằng cách tắt server giữa chừng một request, và xác nhận các stream
   đang xử lý dở dưới last-stream-ID hoàn tất thay vì báo lỗi.
