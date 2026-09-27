# Runtime Configuration and Flavors

## What to learn

### `multi_thread` vs. `current_thread`
`#[tokio::main]` mặc định dùng runtime work-stealing multi-threaded từ
[`01-tokio.md`](01-tokio.md), với số worker mặc định bằng số CPU core.
`#[tokio::main(flavor = "current_thread")]` chạy mọi thứ trên đúng một
thread gọi, hoàn toàn không có work-stealing. Một proxy hầu như luôn
muốn `multi_thread` ở production (nó rải kết nối qua các core), nhưng
`current_thread` thực sự hữu ích cho test và công cụ nơi thực thi
single-threaded, có thể đoán trước đáng giá hơn thông lượng.

```rust
#[tokio::main(worker_threads = 4)] // ghi đè tường minh; mặc định là num_cpus
async fn main() { /* ... */ }

#[tokio::test] // mặc định current_thread — khởi động nhanh, dễ suy luận hơn
async fn handles_one_request() { /* ... */ }
```

### `LocalSet` và future `!Send`
Mọi future spawn bằng `tokio::spawn` phải là `Send`, vì scheduler có thể
di chuyển nó giữa các worker thread. `tokio::task::LocalSet` (đi kèm một
runtime `current_thread`, hoặc được vào qua `LocalSet::run_until`) cho
bạn `spawn_local` một future *không* `Send` — hữu ích khi bọc một thư
viện C không thread-safe qua FFI ([`03-rust/14-ffi-and-abi.md`](../03-rust/14-ffi-and-abi.md)), hoặc tái
sử dụng một cấu trúc dựa trên `Rc<RefCell<_>>` mà không phải trả giá cho
`Arc<Mutex<_>>`.

```rust
let local = tokio::task::LocalSet::new();
local.run_until(async {
    tokio::task::spawn_local(async { /* future !Send, ổn ở đây */ }).await.unwrap();
}).await;
```
Gotcha: điều này chỉ hoạt động vì mọi thứ ở lại trên một thread. Với tới
`LocalSet` chỉ để im lặng một lỗi `Send`, mà không hiểu vì sao future đó
ban đầu không `Send`, thường có nghĩa là có một `Rc`/`RefCell` ẩn lẽ ra
nên là `Arc`/`Mutex`.

### Định cỡ blocking pool
`spawn_blocking` ([`04-runtime/01-tokio.md`](01-tokio.md)) chạy việc trên một thread pool
riêng, được định cỡ bằng `max_blocking_threads` (mặc định 512) — rộng
rãi, vì blocking thread phần lớn ngồi idle chờ I/O hoặc một mutex thay vì
đốt CPU, nên có nhiều thread như vậy là rẻ. Điều này không liên quan tới
`worker_threads`, thứ nên giữ gần với số core vì các thread đó được kỳ
vọng bận CPU; nhầm lẫn hai cái — vặn `worker_threads` vượt xa số core
"để có thêm song song" — làm overhead context-switching tệ hơn, không
tốt hơn.

### Quan sát một runtime đang chạy: tokio-console và metrics
`tokio-console` (một TUI kết nối qua crate `console-subscriber`) cho thấy
số task đang sống và thời lượng poll theo thời gian thực, và làm cho bug
gọi-blocking-trong-code-async (failure mode trung tâm của [`01-tokio.md`](01-tokio.md))
hiện rõ trực tiếp thay vì phải suy đoán từ triệu chứng.
`tokio::runtime::Handle::metrics()` (một tập con stable, nhiều hơn dưới
`tokio_unstable`) cho quyền truy cập lập trình vào số lần ăn cắp của
worker, độ sâu queue, và busy time — cùng dữ liệu mà
[`08-observability/02-metrics.md`](../08-observability/02-metrics.md) muốn export dưới dạng Prometheus gauge
cho một proxy đang chạy.

### Chọn `worker_threads` một cách có chủ đích
Ít worker hơn số core sẽ không tận dụng hết phần cứng; nhiều worker hơn
số core thêm overhead lập lịch mà không có chỗ nào cho các thread thừa
thực sự chạy song song. Ngoại lệ chủ đích phổ biến duy nhất: dành riêng
một core cho việc khác — một thread chuyên scrape metrics, hoặc chừa
khoảng trống trên một host dùng chung — bằng cách đặt `worker_threads`
bằng số core trừ một. Với lý do hình thức vì sao thêm worker ngừng có ích
— trần phần-tuần-tự của Amdahl's Law, và vì sao workload của một proxy
gần chế độ của Gustafson hơn là của Amdahl — xem [`22-theory/08-amdahls-law.md`](../22-theory/08-amdahls-law.md).

## Practice
1. Chạy [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) dưới cả `flavor = "multi_thread"` (mặc
   định) và `flavor = "current_thread"`, load-test cả hai, và đo khác
   biệt thông lượng dưới các kết nối đồng thời.
2. Xây một ví dụ nhỏ dùng `LocalSet` + `spawn_local` với trạng thái chia
   sẻ `Rc<RefCell<_>>`, và xác nhận cùng đoạn code đó không compile với
   `tokio::spawn` thuần.
3. Cài `tokio-console`, gắn `console-subscriber` vào một crate lab, và cố
   tình gọi một hàm blocking bên trong một handler async — tìm nó trong
   console qua thời lượng poll của nó.
4. In số lần ăn cắp của worker từ `tokio::runtime::Handle::metrics()` cho
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) dưới tải, và nối một đợt tăng đột biến trong
   số lần ăn cắp với việc phân phối kết nối không đều giữa các worker.
5. Giải thích bằng lời của bạn vì sao đặt `worker_threads` vượt xa số
   core sẽ làm [`proxy`](../../proxy) chậm hơn, không nhanh hơn, dưới tải bền vững.
