# DDoS & Giảm thiểu tấn công Volumetric

Lớp nằm dưới `07-ratelimit.md` và `06-waf.md`: các cuộc tấn công cố gắng
làm cạn kết nối hoặc băng thông trước khi bất kỳ request nào được parse.

## What to learn
### Ý tưởng nền tảng: sự bất cân xứng về chi phí
Mọi biện pháp phòng thủ trong file này là câu trả lời cho một câu hỏi —
*request này tốn của attacker bao nhiêu, và tốn của tôi bao nhiêu?* Một
cuộc tấn công có hiệu quả khi tỷ lệ đó lệch hẳn: một SYN tốn attacker một
packet và tốn bạn một socket; một gzip bomb tốn họ 10 KB upload và tốn bạn
10 GB RAM; một search query tốn họ một URL và tốn bạn một full table scan.

Đọc mọi biện pháp dưới đây như một cách khôi phục lại sự đối xứng — bằng
cách làm cho việc reject rẻ (drop ở kernel, reject trước khi parse) hoặc
làm cho con đường đắt đỏ không khả dụng cho tới khi client chứng minh được
điều gì đó. Khi bạn tự đánh giá một biện pháp phòng thủ, hãy tính chi phí
cả hai phía trước khi quyết định nó có tác dụng.

### Khác biệt với rate limiting và WAF
`07-ratelimit.md` và `06-waf.md` hoạt động trên các HTTP request đã được
parse — chúng giả định kết nối đã được accept và proxy đang đọc byte từ
đó. Một cuộc tấn công volumetric hoặc làm cạn kết nối (SYN flood, một
flood các connection attempt trông hợp lệ, tấn công slow-client) cố gắng
thắng *trước* điểm đó, bằng cách làm cạn file descriptor, bộ nhớ, hoặc CPU
ngay trên accept path. Các biện pháp phòng thủ ở đây phải rẻ hơn trên mỗi
lần thử so với một rate limiter ở tầng HTTP, vì bạn không thể parse đầy đủ
một request chỉ để reject nó.

### SYN flood thường không phải việc proxy phải giải quyết
Một SYN flood được trả lời bằng cơ chế SYN cookie của kernel
(`net.ipv4.tcp_syncookies`) hoặc bởi hạ tầng đứng trước proxy (dịch vụ
scrubbing DDoS L3/L4 của cloud provider, một anycast edge). Một process
Rust đơn lẻ không thể scale vượt một cuộc flood volumetric thật — cố xử lý
nó hoàn toàn trong code ứng dụng là chiến đấu ở lớp sai. Việc của proxy là
defense-in-depth cho những gì thực sự đến được nó dưới dạng một kết nối đã
established, không phải thay thế một lớp scrubbing.

Vẫn nên biết cơ chế này, vì nó giải thích ranh giới: SYN cookie cho phép
kernel ngừng cấp state cho các kết nối half-open bằng cách encode các
tham số kết nối ngay vào sequence number, nên SYN backlog
(`16-kernel/03-tcp-stack.md`) không thể bị làm cạn. Chi phí là các TCP
option được negotiate trong SYN bị mất một phần — đó là lý do nó là một
fallback được kích hoạt dưới áp lực chứ không phải mặc định.

### Biết trần tài nguyên thật của bạn
"Cạn kiệt" là một con số cụ thể, và bạn nên biết con số của mình trước khi
attacker tìm ra nó thay bạn. Trên mỗi kết nối, một proxy tốn: một file
descriptor (`ulimit -n`, thường vẫn là 1024 theo mặc định trong container —
kiểm tra, đừng giả định), kernel socket buffer ở cả gửi và nhận
(`16-kernel/03-tcp-stack.md`: hàng chục KB mỗi cái, và *không* được tính vào
RSS của process bạn), buffer đọc/viết của riêng bạn
(`14-memory/04-buffer-pool.md`), và một task với state machine của nó.

Ở 100k kết nối đồng thời, chỉ 64 KB kernel buffer mỗi kết nối đã là 6.4 GB
bộ nhớ kernel. Hãy tự làm phép nhân này cho config của bạn, rồi set connection
cap dưới đây thành một số bạn đã thực sự chứng minh mình giữ được — một cap
đặt cao hơn trần thật của bạn không phải là một cap.

### Accept-rate limiting: tuyến phòng thủ thật sự của proxy
Điều proxy *có thể* kiểm soát rẻ: tốc độ nó accept kết nối mới và số kết
nối nó giữ đồng thời, độc lập với bất kỳ rate limit HTTP theo client nào.
Một semaphore quanh accept loop chặn số kết nối đồng thời đang xử lý; một
token bucket ngay trên `accept()` chặn tốc độ kết nối mới, reject (đóng
ngay) khi chạm một trong hai giới hạn — rẻ hơn nhiều so với đọc bất kỳ byte
nào từ client trước.

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

async fn accept_loop(listener: tokio::net::TcpListener, max_inflight: usize) {
    let permits = Arc::new(Semaphore::new(max_inflight));
    loop {
        let (socket, _addr) = match listener.accept().await {
            Ok(pair) => pair,
            Err(_) => continue,
        };
        let permits = permits.clone();
        match permits.clone().try_acquire_owned() {
            Ok(permit) => {
                tokio::spawn(async move {
                    let _permit = permit; // giữ tới khi connection task kết thúc
                    handle_connection(socket).await;
                });
            }
            Err(_) => drop(socket), // vượt capacity: reject ngay, không xếp hàng
        }
    }
}
```
Gotcha: reject bằng cách drop socket ngay là có chủ đích — xếp hàng các kết
nối bị reject (hoặc tệ hơn, parse gì đó trước khi reject) chỉ chuyển điểm
cạn kiệt từ file descriptor sang bất kỳ resource nào hàng đợi đó tiêu tốn.

Gotcha: `listener.accept()` trả về `Err` đáng được chăm sóc hơn là chỉ
`continue`. `EMFILE`/`ENFILE` (hết file descriptor) là *dai dẳng* — lần
`accept()` kế tiếp cũng fail ngay, và loop này quay ở 100% CPU mà không
tạo ra gì, biến việc cạn fd thành một cú stall CPU toàn phần. Match theo
loại lỗi: back off ngắn khi gặp `EMFILE`, và xem xét giữ sẵn một fd "hy
sinh" mà bạn có thể đóng để accept-rồi-reject-ngay một kết nối, kỹ thuật
kinh điển để degrade nhẹ nhàng thay vì quay vòng vô ích.

Gotcha: khi bạn ngừng accept, các kết nối xếp hàng trong accept backlog của
kernel rồi bị refuse — đó là hành vi đúng, không phải bug cần che đi. Đẩy
backpressure vào kernel chính là mục đích; accept các kết nối bạn không
thể serve chỉ dời chỗ thất bại vào bộ nhớ của chính bạn.

### Tấn công slow-client
Một attacker gửi một byte mỗi 10 giây có thể giữ một kết nối mở vô hạn dưới
một chính sách "không có total timeout" ngây thơ, với chi phí gần như bằng
0 cho chính họ — sự bất cân xứng chi phí tệ nhất trong file này. Họ có ba
biến thể (slow header, slow body, slow read), và biện pháp phòng thủ là
một *rate* dữ liệu tối thiểu mỗi phase thay vì một deadline tổng mà một
attacker biết tính toán chỉ cần chờ qua.

Xem `07-security/10-slowloris.md` cho ba biến thể và thiết kế rate floor.

### Layer 7: flood vào endpoint đắt đỏ
Cuộc tấn công hiệu quả nhất thường không hề volumetric — nó là tìm ra
endpoint mà một request rẻ tốn bạn nhiều nhất. Một search query không có
index, một endpoint render report, một regex trên input lớn
(`13-algorithms/regex-engine.md`), một image resize. Vài trăm request mỗi
giây — thấp hơn hẳn bất kỳ rate limit hợp lý nào — làm bão hòa upstream mà
vẫn trông như traffic bình thường.

Biện pháp phòng thủ là theo từng endpoint chứ không toàn cục: rate limit
riêng, chặt hơn, cho các route đắt đỏ (`07-ratelimit.md` key theo route,
không chỉ theo client), giới hạn concurrency theo route để một endpoint
không thể tiêu hết cả upstream pool, và — cách sửa mang tính cấu trúc — coi
"endpoint nào đắt đỏ" là thứ bạn *đo* (`08-observability/02-metrics.md`
latency theo route và thời gian upstream) thay vì đoán.

### Decompression bomb
Nếu proxy nhận `Content-Encoding: gzip` trên request body và decompress nó
để inspect (`06-waf.md`) hoặc transform, thì một upload 10 KB có thể phình
ra 10 GB. Tỷ lệ compression là leverage của attacker và nó cực lớn — đây là
sự bất cân xứng chi phí tệ nhất có ở layer 7.

Cách sửa là chặn *output*, không phải input: decompress qua một reader bị
cap ở một kích thước decompressed tối đa và abort ngay khi vượt, thay vì
decompress vào một `Vec` rồi mới check sau (lúc đó bạn đã allocate nó rồi).

```rust
// chặn thứ bạn sẵn sàng vật chất hóa, trước khi vật chất hóa nó
let mut limited = decoder.take(MAX_DECOMPRESSED_BYTES);
let n = limited.read_to_end(&mut buf)?;   // dừng ở cap, không dừng ở kích thước bomb
```
Gotcha: cũng chặn *tỷ lệ*, không chỉ kích thước tuyệt đối. Một body phình
ra 1000:1 là thù địch dù nó có nằm dưới cap của bạn, và tỷ lệ là một tín
hiệu tốt hơn nhiều so với kích thước đơn thuần để phân biệt một cuộc tấn
công với một upload lớn hợp lệ. Xem `05-http-stack/06-compression.md` cho
mặt gương phía response của vấn đề này.

### Load shedding thắng queueing
Khi công việc đến vượt capacity, xếp hàng phần dư biến một overload thành
một death spiral về latency — đến lúc một request được xếp hàng được serve,
client của nó đã timeout và retry rồi. Reject ngay, rẻ, và càng sớm càng
tốt trong pipeline là điều giữ throughput hữu ích khỏi sụp đổ.

Xem `07-security/11-load-shedding.md` cho lập luận shed-so-với-queue, các
giới hạn dựa trên thời gian, priority shedding, và adaptive concurrency
limit.

### Đẩy quyết định xuống thấp hơn trong stack
Mọi thứ ở trên chạy sau một bắt tay TCP (và thường cả TLS) mà bạn đã trả
tiền rồi. Khi đã *xác định* được một attacker, nơi rẻ để drop họ nằm thấp
hơn nhiều: một entry `nftables`/`ipset`, hoặc XDP ở driver
(`16-kernel/10-xdp.md`), nơi một packet chết trước khi một socket tồn tại.

Đây là feedback loop mà `labs/17-ebpf` xây dựng: proxy có context ứng dụng
để quyết định ai là kẻ lạm dụng, kernel có vị trí để drop họ miễn phí. Giữ
quyết định ở proxy và enforcement càng thấp càng tốt.

## Practice
Làm theo thứ tự này.

1. Tính trần của bạn trước. **Xong khi** bạn đã viết ra, cho config của
   `proxy`: `ulimit -n`, kích thước kernel socket buffer mỗi kết nối,
   allocation buffer của riêng bạn mỗi kết nối, và số kết nối tối đa suy
   ra — và đã xác minh con số đó bằng cách thực sự giữ mở đúng số kết nối
   idle đó.
2. Thêm bộ giới hạn accept-rate/concurrency, với metrics riêng khỏi 429 ở
   tầng application (`07-ratelimit.md`). **Xong khi** vượt cap đóng kết nối
   mới ngay và số lượng reject hiển thị như một metric riêng.
3. Xử lý `EMFILE` đúng cách trong accept loop. **Xong khi** hạ `ulimit -n`
   xuống một số nhỏ và làm ngập kết nối tạo ra backoff và reject sạch sẽ
   thay vì một cú spin 100% CPU — xem `top` để xác nhận.
4. Làm các bài tập trong `07-security/10-slowloris.md`. **Xong khi** cả ba
   biến thể slow-client bị shed và một client chậm nhưng hợp lệ thì không.
5. Thêm một giới hạn decompression trên request body với cả cap tuyệt đối
   và cap tỷ lệ. **Xong khi** một gzip bomb bị reject mà chỉ allocate tới
   cap của bạn (đo RSS trong lúc test để chứng minh), và một body 50 MB
   compress hợp lệ vẫn hoạt động.
6. Làm các bài tập trong `07-security/11-load-shedding.md`. **Xong khi**
   overload một route đắt đỏ trả 503 nhanh thay vì xếp hàng, và các route
   khác vẫn serve bình thường.
7. Viết ra, trong README của `proxy`, những nhóm tấn công nào `proxy` tự
   giảm thiểu so với những nhóm cần hạ tầng đứng trước nó. **Xong khi**
   ranh giới đó rõ ràng — đây là quyết định thiết kế thật sự, không phải
   một chi tiết để bỏ qua.
