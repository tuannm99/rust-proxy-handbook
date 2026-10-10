# Latency, Bandwidth, Throughput, RTT

Một phần của chuỗi fundamentals từ-con-số-0 — xem [`01-network/01-fundamentals.md`](01-fundamentals.md)
để có index đầy đủ. Bốn con số bị dùng lẫn lộn với nhau trong giao tiếp
thường ngày và không nên như vậy — nhầm lẫn chúng dẫn tới việc tối ưu sai
thứ.

## What to learn

### Latency: mất bao lâu, không phải bao nhiêu
**Latency** là thời gian một mẩu dữ liệu mất để đi từ A tới B. Nó chủ yếu
bị chi phối bởi khoảng cách vật lý (ánh sáng trong sợi quang di chuyển ở
tốc độ khoảng 200.000 km/s, không phải 300.000, do chiết suất của thủy
tinh) và số hop (mục routing của [`02-addressing.md`](02-addressing.md)) — không phải bởi độ
"nhanh" của kết nối bạn theo nghĩa thông thường. Một đường truyền xuyên
lục địa có độ trễ hàng chục mili-giây bất kể bạn đổ bao nhiêu bandwidth
vào nó, vì đó là một giới hạn của tốc độ ánh sáng, không phải vấn đề
congestion.

### Bandwidth: cái trần
**Bandwidth** là tốc độ tối đa một đường truyền có thể mang dữ liệu, ví dụ
1 Gbps. Đó là một cái trần, không phải một cam kết bạn sẽ đạt tới nó —
rất nhiều yếu tố khiến việc truyền dữ liệu thực tế thấp hơn hẳn mức tối đa
lý thuyết.

### Throughput: cái bạn thực sự đạt được
**Throughput** là tốc độ bạn *thực sự* đạt được, bị giới hạn bởi bandwidth
nhưng thường thấp hơn, do overhead của protocol, congestion, retransmit,
hoặc hành vi của ứng dụng (ví dụ code của bạn sản xuất hoặc tiêu thụ byte
nhanh tới đâu). "Bandwidth" và "throughput" bị dùng lẫn lộn trong giao
tiếp thường ngày; hãy giữ chúng tách biệt khi bạn thực sự đang chẩn đoán
một vấn đề — một đường truyền có thể có thừa bandwidth mà vẫn cho
throughput kém nếu thứ khác mới là nút thắt cổ chai.

### RTT: con số quyết định chi phí handshake
**RTT (round-trip time)** là thời gian để một message đi ra và phản hồi
của nó quay về — xấp xỉ `2 × latency` cộng thời gian xử lý ở đầu bên kia.
Đây là con số quan trọng cho câu hỏi "chuyện này tốn bao nhiêu round
trip": mỗi handshake ([`03-byte-streams.md`](03-byte-streams.md)) — của TCP, rồi của TLS chồng
lên trên — là thêm một RTT chờ đợi thuần túy trước khi byte request thật
đầu tiên di chuyển. Đó chính là toàn bộ luận điểm cho việc tái sử dụng kết
nối trong [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md): trả giá một handshake bị giới hạn bởi
RTT một lần và tái sử dụng kết nối tốt hơn trả giá đó ở mỗi request.

### Vì sao một đường truyền bandwidth cao vẫn có thể cảm giác chậm
Một đường truyền có thể có bandwidth khổng lồ mà vẫn cảm giác ì ạch nếu
latency (và do đó RTT) cao — một đường truyền vệ tinh là ví dụ kinh điển:
bandwidth khổng lồ, latency tệ hại (chuyến đi tới quỹ đạo địa tĩnh và quay
về là khoảng cách vật lý thật sự). Với một cuộc trao đổi request/response
nhỏ — phần lớn traffic HTTP — bản thân việc truyền dữ liệu nhanh đến mức
RTT, chứ không phải bandwidth, chi phối tổng thời gian: bạn đang chờ round
trip, không phải chờ byte. Đây chính xác là lý do multiplexing của HTTP/2
([`01-network/17-http2.md`](17-http2.md)) và 0-RTT/session resumption trong TLS
([`01-network/19-tls.md`](19-tls.md)) tồn tại — chúng tấn công vào *số lượng* round
trip, không phải throughput.

### Bandwidth-delay product: bao nhiêu có thể "đang bay"
**Bandwidth-delay product** (bandwidth × RTT) là số byte có thể đang trên
đường truyền cùng lúc, chưa được ack — congestion window của TCP
([`01-network/12-tcp.md`](12-tcp.md)) phải lớn lên tới xấp xỉ kích thước này trước khi
một kết nối duy nhất có thể dùng hết bandwidth của đường truyền. Trên một
đường truyền bandwidth cao, latency cao ("long fat network" — một đường
truyền vệ tinh, hay một tuyến cáp quang xuyên lục địa), tích số này lớn,
và một kết nối bắt đầu với congestion window nhỏ (mọi kết nối TCP mới đều
vậy, qua slow start) mất một thời gian để tăng tốc tới mức dùng hết
bandwidth khả dụng. Đây là một lý do khác khiến một kết nối mới cho mỗi
request kém hiệu quả hơn một kết nối được tái sử dụng, độc lập với chi phí
handshake-RTT ở trên: congestion window của một kết nối được tái sử dụng
đã "ấm" sẵn.

## Practice
1. Chạy `ping example.com` (ICMP, một phép đo latency thô) và
   `curl -w "%{time_connect} %{time_appconnect} %{time_starttransfer}\n"
   -o /dev/null -s https://example.com` — so sánh latency của `ping` với
   `time_connect` (RTT của TCP handshake), `time_appconnect` (cộng thêm
   TLS), và `time_starttransfer` (cộng thêm byte response đầu tiên). Xác
   định TLS tốn thêm bao nhiêu round trip.
2. Chọn một site gần bạn về mặt địa lý và một site ở xa (ví dụ một server
   ở châu lục khác) rồi lặp lại phép đo `curl -w` cho cả hai — xác nhận
   `time_connect` của site xa khớp xấp xỉ với latency tốc-độ-ánh-sáng bạn
   kỳ vọng cho khoảng cách đó.
3. Tải một file lớn (vài trăm MB) từ một mirror nhanh và tính throughput
   thực tế (`size / time`); so sánh với bandwidth được quảng cáo của kết
   nối bạn — giải thích khoảng cách nếu có.
4. Tính bandwidth-delay product cho một đường truyền 100 Mbps ở RTT 150ms
   (một con số xuyên lục địa khả dĩ) theo byte; so sánh con số đó với
   initial congestion window mặc định của TCP (~10 segment, ~14KB) và
   giải thích vì sao một kết nối mới trên đường truyền này bắt đầu chỉ
   dùng một phần nhỏ bandwidth khả dụng.
