# Error Handling

## What to learn

### `Result`, `Option`, và toán tử `?`
`Result<T, E>` mô hình hóa một lỗi có thể phục hồi, `Option<T>` mô hình
hóa một giá trị có thể vắng mặt. `?` lan truyền `Err`/`None` sớm và convert
error type qua `From` trong lúc đó — đây là lý do vì sao error type của
một hàm thường là một enum tự viết với `impl From<std::io::Error> for
MyError` thay vì `io::Error` thô: nó hợp nhất mọi nguồn lỗi nội bộ thành
một type duy nhất tại API boundary.

```rust
#[derive(Debug)]
enum ProxyError { Io(std::io::Error), BadUpstream(String) }
impl From<std::io::Error> for ProxyError {
    fn from(e: std::io::Error) -> Self { ProxyError::Io(e) }
}
fn connect() -> Result<std::net::TcpStream, ProxyError> {
    let s = std::net::TcpStream::connect("10.0.0.1:80")?; // io::Error auto-converted via From
    Ok(s)
}
```

### `thiserror` vs `anyhow`
`thiserror` derive `Display`/`Error` cho một enum cụ thể — dùng nó ở nơi
caller cần match trên một variant cụ thể và quyết định hành vi (retry so
với `502` so với `503`, theo [`01-network/15-http.md`](../01-network/15-http.md)). `anyhow::Error` là
một hộp "bất kỳ lỗi nào" bị xóa type, có context-chaining — dùng nó trong
code glue/binary (`main.rs`, thiết lập CLI) nơi bạn chỉ muốn log hoặc bail
mà không cần caller match một variant. Đặt `anyhow` vào public API của
một thư viện buộc mọi downstream caller cũng phải phụ thuộc vào `anyhow`
và mất khả năng match variant — đây là sự lạm dụng phổ biến nhất của hai
crate này.

```rust
// library-ish: concrete, matchable
#[derive(thiserror::Error, Debug)]
enum UpstreamError {
    #[error("upstream unreachable: {0}")]
    Unreachable(#[from] std::io::Error),
    #[error("upstream timed out")]
    Timeout,
}

// binary glue: just propagate + add context
fn load_config(path: &str) -> anyhow::Result<Config> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading config at {path}"))?;
    Ok(toml::from_str(&text)?)
}
```

### Panic là bug, không phải một lỗi được dự tính trước
Một panic nghĩa là một invariant mà code giả định là đúng, hóa ra không
đúng — một index ngoài phạm vi, một `.unwrap()` trên một `None` mà "lẽ ra
không bao giờ xảy ra," một tràn số nguyên ở debug mode. Một proxy không
bao giờ được để input bị lỗi dạng, do attacker kiểm soát, chạm tới một
`.unwrap()`/`.expect()` — đó là một DoS có thể kích hoạt từ xa. Dành panic
cho lỗi lập trình viên mà bạn muốn có stack trace lúc phát triển; trả về
`Result` cho bất cứ thứ gì bắt nguồn từ input mạng.

Gotcha: một panic bên trong một task được `tokio::spawn` không làm sập
process — nó bị bắt lại và hiện ra như một `Err` trên `JoinHandle` của
task đó — nhưng nếu không có gì bao giờ await handle đó, panic bị nuốt
im lặng và connection chỉ đơn giản biến mất mà không có dòng log nào.
Phần thảo luận về spawn trong [`03-rust/04-sync.md`](04-sync.md) và
[`08-observability/01-logging.md`](../08-observability/01-logging.md) đều quay lại điểm này: luôn quan sát
đường panic của một task được spawn.

### Mutex poisoning và `catch_unwind`
Một `std::sync::Mutex` trở nên "poisoned" nếu một thread panic trong lúc
đang giữ lock — mọi `.lock()` sau đó trả về một `Err` thay vì âm thầm tiếp
tục với state có khả năng đã hỏng. `parking_lot::Mutex` (một lựa chọn thay
thế phổ biến trong code proxy) không bao giờ poison, đánh đổi lấy việc
không phải xử lý đường lỗi poisoned-lock ở khắp mọi nơi, điều mà nhiều
codebase quyết định là lựa chọn thực dụng một khi các critical section
nhỏ và đã được audit. `std::panic::catch_unwind` có thể bắt một panic tại
một FFI boundary hoặc bên trong một executor tự viết, nhưng nó không phải
là một công cụ xử lý lỗi tổng quát — phần lớn code nên để ranh giới panic
theo từng task của `tokio::spawn` làm việc này thay vì rắc `catch_unwind`
khắp nơi.

## Practice
1. Thiết kế các enum `ProxyError`/`UpstreamError` với `thiserror`, và
   trong [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) ánh xạ mỗi variant sang đúng HTTP status
   (`502`/`503`/`504`) theo [`01-network/15-http.md`](../01-network/15-http.md).
2. Viết một `main.rs` nhỏ dùng `anyhow::Result` + `.context()` cho việc
   load config, và so sánh chuỗi lỗi được in ra với một phiên bản
   `Result<_, io::Error>` thô.
3. Chủ động panic bên trong một task được `tokio::spawn`, xác nhận process
   vẫn chạy tiếp, rồi thêm code await `JoinHandle` và log panic đó thay vì
   để nó biến mất âm thầm.
4. Thay một `std::sync::Mutex` bằng `parking_lot::Mutex` trong một ví dụ
   nhỏ; panic trong lúc đang giữ lock ở phiên bản std và quan sát `Err`
   poisoned-lock, rồi để ý `parking_lot` không có cơ chế tương đương nào
   cả.
5. Tìm một `.unwrap()` trong code [`labs/01-http-parser`](../../labs/01-http-parser) của chính bạn
   chạy trên input do attacker kiểm soát, và chuyển nó thành một đường
   `Result` trả về một lỗi parse thay vì panic.
