# Structured Concurrency and Cancellation

## What to learn

### `JoinHandle`: một task đã spawn không phải fire-and-forget
`tokio::spawn` trả về một `JoinHandle<T>` — drop nó *không* hủy task
(nó tiếp tục chạy tách rời), nhưng bạn mất khả năng lấy kết quả hoặc quan
sát một panic (gotcha "panic bị nuốt âm thầm" của
`03-rust/08-error-handling.md` sống ở đây). `.abort()` trên một
`JoinHandle` hủy task ở điểm `.await` tiếp theo của nó, và await handle
sau đó trả về một `JoinError` bạn có thể kiểm tra để phân biệt "đã
panic" với "đã bị hủy."

```rust
let handle = tokio::spawn(async { long_upstream_call().await });
tokio::time::sleep(std::time::Duration::from_secs(5)).await;
handle.abort();
match handle.await {
    Ok(v) => use_result(v),
    Err(e) if e.is_cancelled() => tracing::warn!("cancelled"),
    Err(e) => tracing::error!("panicked: {e}"),
}
```

### `JoinSet`: quản lý nhiều task như một đơn vị
Spawn N task và tự tay theo dõi N `JoinHandle` để biết khi nào tất cả đã
xong chính xác là lý do `tokio::task::JoinSet` tồn tại — nó sở hữu một
tập động các task đã spawn, và `.join_next().await` trả kết quả khi
chúng hoàn thành, theo thứ tự hoàn thành chứ không phải thứ tự spawn.
Drop một `JoinSet` sẽ abort mọi task còn bên trong nó, đây là thuộc tính
structured-concurrency mà các lời gọi `tokio::spawn` tách rời thuần túy
không cho bạn miễn phí: một `JoinSet` ra khỏi scope là một ranh giới hủy
thật sự.

```rust
let mut set = tokio::task::JoinSet::new();
for upstream in pool.iter() {
    set.spawn(health_check(upstream.clone()));
}
while let Some(res) = set.join_next().await {
    handle_health_result(res);
}
// nếu hàm này return sớm (ví dụ qua `?`), JoinSet bị drop ở đây và mọi
// health check còn đang chạy bị abort — không có task mồ côi nào sót lại
```
Đây là câu trả lời trực tiếp cho câu hỏi "ai hủy các health checker khi
pool bị dỡ bỏ" trong `06-proxy/03-healthcheck.md`.

### `select!` và cancellation-safety, với API thật
`03-rust/05-async.md` bao quát "drop là hủy" ở mức khái niệm; trong thực
tế câu hỏi là: một thao tác có để lại trạng thái chia sẻ nhất quán nếu bị
drop giữa chừng không? `tokio::sync::Mutex::lock().await` là
cancellation-safe — drop future trước khi nó resolve chỉ có nghĩa lock
chưa bao giờ được lấy, không có gì để dọn dẹp. Một "protocol" tự viết tay
gửi một request header, rồi một body, qua hai điểm `.await` riêng biệt
thì *không* tự động cancellation-safe: bị hủy giữa hai điểm đó để lại
phía kia đã nhận nửa message. Tài liệu của `tokio::select!` duy trì một
danh sách những future stdlib/tokio nào là cancellation-safe chính vì lý
do này — kiểm tra nó trước khi dựa vào một nhánh "thua cuộc đua, bị drop"
cho bất cứ thứ gì có tác dụng phụ nhiều bước.

### Task-local storage
`tokio::task_local!` cho mỗi task một instance riêng của một giá trị,
truy cập được mà không cần luồn nó qua mọi lời gọi hàm — tương đương
async của một thread-local, gắn theo phạm vi một task đã spawn thay vì
một OS thread (điều không có ý nghĩa ở đây, vì nhiều task chia sẻ một
thread). Ứng dụng thực tế phổ biến là một trace/span context theo từng
request (`08-observability/03-tracing.md`) hoặc request ID mà mọi dòng
log ở bất cứ đâu trong call graph của task đó nên mang theo, mà không cần
một tham số tường minh ở mọi nơi.

```rust
tokio::task_local! {
    static REQUEST_ID: u64;
}
REQUEST_ID.scope(request_id, async move {
    handle_request().await // bất cứ gì gọi từ đây đều có thể REQUEST_ID.with(|id| ...)
}).await;
```

### Graceful shutdown như một dạng structured concurrency
Bài toán cốt lõi của `09-architecture/04-graceful-shutdown.md` — ngừng
nhận kết nối mới, để các kết nối đang xử lý dở hoàn tất, rồi thoát — về
bản chất là một bài toán hủy/theo-dõi-hoàn-thành: một `JoinSet`, hoặc một
tín hiệu shutdown `tokio::sync::watch` được đua qua `select!` bên trong
vòng lặp mỗi kết nối, là cơ chế cụ thể mà mẫu thiết kế này biên dịch
thành.

## Practice
1. Spawn một task, drop `JoinHandle` của nó ngay lập tức, và xác nhận —
   qua một dòng log bên trong task — rằng nó vẫn chạy tới hoàn thành,
   tách rời chứ không bị hủy.
2. Xây lại cùng task đó với `.abort()` được gọi giữa chừng, xác nhận nó
   dừng ở điểm `.await` tiếp theo, và rẽ nhánh trên `.is_cancelled()`
   của `JoinError` kết quả.
3. Thay một `Vec<JoinHandle>` thủ công trong bài tập
   `06-proxy/03-healthcheck.md` bằng một `JoinSet`, rồi chứng minh thuộc
   tính structured-cancellation: return sớm khỏi hàm sở hữu và xác nhận
   (qua một dòng log theo-task khi drop) rằng các health check đang chạy
   bị abort.
4. Viết tay một "protocol" hai điểm-`.await` (gửi header, rồi body, mỗi
   cái đứng sau một `tokio::time::sleep` giả lập I/O thật), đua nó với
   một timeout ngắn bằng `select!`, và chứng minh phía kia kết thúc với
   một message gửi dở.
5. Gắn `tokio::task_local!` cho một request ID vào
   `labs/05-reverse-proxy`, và xác nhận mọi dòng log ở bất cứ đâu trong
   call graph của request đó có thể đọc nó mà không cần truyền như một
   tham số.
