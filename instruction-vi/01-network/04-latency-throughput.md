# Latency, Bandwidth, Throughput, RTT

File này thuộc nhóm fundamentals. Xem [`01-network/01-fundamentals.md`](01-fundamentals.md) để có mục lục đầy đủ. Đây
là bốn con số thường bị dùng lẫn lộn trong giao tiếp hằng ngày. Đừng làm vậy: nhầm chúng dẫn tới việc tối ưu sai thứ.

## What to learn

### Latency: mất bao lâu, không phải bao nhiêu
**Latency** là thời gian một mẩu dữ liệu đi từ A tới B. Nó chủ yếu do khoảng cách vật lý và số hop quyết định (xem mục
routing của [`02-addressing.md`](02-addressing.md)). Nó không phụ thuộc vào việc connection của bạn "nhanh" theo nghĩa
thông thường. Ánh sáng trong sợi quang đi với tốc độ khoảng 200.000 km/s, không phải 300.000, vì chiết suất của thủy
tinh.

Một đường truyền xuyên lục địa có latency hàng chục mili giây, dù bạn đổ bao nhiêu bandwidth vào nó. Đó là giới hạn của
tốc độ ánh sáng, không phải vấn đề congestion.

### Bandwidth: cái trần
**Bandwidth** là tốc độ tối đa mà đường truyền mang được dữ liệu, ví dụ 1 Gbps. Đó là một cái trần, không phải cam kết
rằng bạn sẽ đạt tới nó. Rất nhiều yếu tố làm tốc độ truyền thực tế thấp hơn nhiều so với mức tối đa lý thuyết.

### Throughput: cái bạn thực sự đạt được
**Throughput** là tốc độ bạn *thực sự* đạt được. Nó bị giới hạn bởi bandwidth nhưng thường thấp hơn, vì overhead của
protocol, congestion, retransmit, hoặc hành vi của ứng dụng (ví dụ code của bạn tạo ra hay tiêu thụ byte nhanh đến đâu).

Trong giao tiếp hằng ngày, "bandwidth" và "throughput" hay bị dùng lẫn. Khi chẩn đoán một vấn đề, hãy tách chúng ra.
Một đường truyền có thể thừa bandwidth mà throughput vẫn kém, nếu một thứ khác mới là nút thắt.

### RTT: con số quyết định chi phí của handshake
**RTT (round-trip time)** là thời gian để một message đi ra và phản hồi của nó quay về. Nó xấp xỉ `2 × latency` cộng
thời gian xử lý ở đầu bên kia.

Đây là con số quan trọng cho câu hỏi "việc này tốn bao nhiêu round trip". Mỗi handshake
([`03-byte-streams.md`](03-byte-streams.md)) của TCP, rồi của TLS xếp chồng lên trên, thêm một RTT chờ đợi thuần túy
trước khi byte của request thật đầu tiên được gửi đi.

Đó chính là toàn bộ lý do để reuse connection trong [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md): trả chi phí
handshake bị giới hạn bởi RTT một lần rồi reuse connection thì tốt hơn là trả chi phí đó ở mỗi request.

### Vì sao đường truyền bandwidth cao vẫn có thể chậm
Một đường truyền có thể có bandwidth rất lớn mà vẫn chậm nếu latency (và do đó RTT) cao. Đường truyền vệ tinh là ví dụ
kinh điển: bandwidth lớn, nhưng latency tệ, vì quãng đường tới quỹ đạo địa tĩnh và quay về là khoảng cách vật lý thật.

Với một cuộc trao đổi request/response nhỏ (phần lớn traffic HTTP), việc truyền dữ liệu nhanh đến mức RTT, không phải
bandwidth, chi phối tổng thời gian. Bạn đang chờ round trip, không chờ byte.

Đây đúng là lý do multiplexing của HTTP/2 ([`01-network/17-http2.md`](17-http2.md)) và 0-RTT/session resumption trong
TLS ([`01-network/19-tls.md`](19-tls.md)) tồn tại. Chúng nhắm vào *số lượng* round trip, không phải throughput.

### Bandwidth-delay product: bao nhiêu dữ liệu có thể "đang bay"
**Bandwidth-delay product** (bandwidth × RTT) là lượng byte có thể cùng lúc nằm trên đường truyền mà chưa được ack.
Congestion window của TCP ([`01-network/12-tcp.md`](12-tcp.md)) phải tăng đến xấp xỉ kích thước này thì một connection
mới dùng hết được bandwidth của đường truyền.

Trên đường truyền bandwidth cao, latency cao ("long fat network", ví dụ vệ tinh hay cáp quang xuyên lục địa), tích số
này lớn. Mọi TCP connection mới đều bắt đầu với congestion window nhỏ (qua slow start), nên cần một thời gian để tăng
tốc tới mức dùng hết bandwidth.

Đây là một lý do nữa khiến mở connection mới cho mỗi request kém hiệu quả hơn reuse connection, độc lập với chi phí
handshake-RTT ở trên: congestion window của một connection được reuse đã "ấm" sẵn.

## Practice
1. Chạy `ping example.com` (ICMP, một phép đo latency thô) và
   `curl -w "%{time_connect} %{time_appconnect} %{time_starttransfer}\n" -o /dev/null -s https://example.com`. So sánh
   latency của `ping` với `time_connect` (RTT của TCP handshake), `time_appconnect` (cộng thêm TLS) và
   `time_starttransfer` (cộng thêm byte response đầu tiên). Xác định TLS tốn thêm bao nhiêu round trip.
2. Chọn một site gần bạn về địa lý và một site ở xa (ví dụ server ở châu lục khác), rồi lặp lại phép đo `curl -w` cho
   cả hai. Xác nhận `time_connect` của site xa khớp xấp xỉ với latency theo tốc độ ánh sáng mà bạn kỳ vọng cho khoảng
   cách đó.
3. Tải một file lớn (vài trăm MB) từ một mirror nhanh và tính throughput thực tế (`size / time`). So sánh với bandwidth
   được quảng cáo của connection của bạn, và giải thích khoảng cách nếu có.
4. Tính bandwidth-delay product, theo byte, cho đường truyền 100 Mbps với RTT 150 ms (một con số xuyên lục địa khả dĩ).
   So sánh với initial congestion window mặc định của TCP (~10 segment, ~14 KB), và giải thích vì sao một connection mới
   trên đường truyền này lúc đầu chỉ dùng một phần nhỏ bandwidth khả dụng.
