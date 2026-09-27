# Iterators and Closures

## What to learn

### Iterator là lazy — đó là phần zero-cost
`.iter().map(f).filter(g)` xây một chuỗi các struct adapter không làm gì
cả cho tới khi thứ gì đó drive chúng (`.collect()`, một vòng `for`, một
`.next()` trực tiếp). Compiler thường inline và fuse toàn bộ chuỗi thành
một vòng lặp không có allocation nào cho bản thân các adapter — đây chính
là ý nghĩa thực tế của "zero-cost abstraction": code khai báo, được nối
chuỗi, compile xuống gần như những gì một vòng lặp viết tay sẽ làm.

```rust
let hot_upstreams: Vec<_> = pool.iter()
    .filter(|u| u.healthy())
    .map(|u| u.addr)
    .collect(); // filter + map run in one pass; only .collect() allocates
```
Gotcha: `.collect::<Vec<_>>()` có allocate; nối chuỗi nhiều adapter và chỉ
collect một lần ở cuối vừa idiomatic hơn vừa rẻ hơn so với collect sau mỗi
bước.

### `Fn` / `FnMut` / `FnOnce` — closure là struct
Một closure được desugar thành một struct vô danh capture môi trường của
nó, cộng với một method gọi được sinh ra. `Fn` gọi được qua `&self` (lặp
lại được, capture không mutate), `FnMut` qua `&mut self` (lặp lại được,
có mutate), `FnOnce` qua `self` (tiêu thụ các giá trị capture — gọi được
đúng một lần). Vì `Fn: FnMut: FnOnce`, bất kỳ closure `Fn` nào cũng thỏa
bound `FnMut`/`FnOnce`, nhưng không ngược lại.

```rust
let counter = std::sync::atomic::AtomicUsize::new(0);
let record_hit = || counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed); // Fn: shared-ref capture, callable many times
```
Gotcha: một closure phải chạy nhiều hơn một lần (một handler trên mỗi
connection được map qua nhiều connection) cần `Fn`/`FnMut`, nhưng nếu nó
*move* một resource không `Clone` vào bên trong, nó suy biến thành
`FnOnce` và sẽ không compile ở chỗ yêu cầu `Fn` — cách sửa thông thường là
clone một `Arc` vào closure thay vì move giá trị gốc.

### `impl Trait` ở vị trí trả về so với `Box<dyn Fn>`
Trả về `impl Fn(...) -> ...` cho một closure type cụ thể, không đặt tên,
monomorphized với chi phí bằng không — nhưng mỗi hàm trả về `impl Fn` với
signature giống hệt nhau vẫn trả về type *riêng* của chính nó.
`Box<dyn Fn(...)>` xóa type đi, đây là thứ bạn cần để lưu các closure
không đồng nhất trong một collection (một `Vec` các route handler — xem
[`05-http-stack/03-router.md`](../05-http-stack/03-router.md)).

```rust
fn make_key_extractor(header: &'static str) -> impl Fn(&Request) -> Option<&str> {
    move |req| req.headers().get(header)?.to_str().ok()
}
```

### Các iterator adapter quan trọng trên hot path của một proxy
`.peekable()` để lookahead mà không tiêu thụ (hữu ích trong một parser tự
viết — `01-http-parser`), `.windows()`/`.chunks()` trên slice cho logic
framing, `.try_fold()` để tích lũy có thoát sớm dưới một `Result`,
`.zip()` để ghép cặp iterator tên/giá trị header. Tránh một
`.collect::<Vec<_>>()` ngay sau đó lại `.iter()` lần nữa — vòng lặp qua
một allocation đó thường có thể tránh được bằng cách giữ chuỗi lazy thêm
một bước nữa.

## Practice
1. Viết lại một vòng `for` viết tay có theo dõi index thủ công trong
   [`labs/01-http-parser`](../../labs/01-http-parser) (ví dụ quét tìm `\r\n`) bằng các iterator adapter
   (`.position()`, `.windows()`, `.split()`), và so sánh độ dễ đọc với
   phiên bản vòng lặp.
2. Viết một closure capture một `Arc<Mutex<Stats>>` đã clone và mutate nó
   trên mỗi lần gọi từ nhiều task được spawn; giải thích vì sao nó cần
   `Fn` thay vì `FnOnce` để dùng được theo cách đó.
3. Implement `Iterator` bằng tay cho một type tùy chỉnh (ví dụ một struct
   duyệt qua các chunk của một buffer `Bytes`) và drive nó bằng một vòng
   `for` thông thường để xác nhận việc nối dây `IntoIterator` hoạt động.
4. Trong [`labs/03-router`](../../labs/03-router), lưu các route handler dưới dạng
   `Box<dyn Fn(&Request) -> Response + Send + Sync>` trong một `Vec`, và
   giải thích vì sao `impl Fn` không thể dùng cho kiểu của field đó thay
   vào đó.
5. So sánh một chuỗi `.filter().map().collect()` với một vòng lặp viết
   tay làm cùng công việc bằng cách xem codegen `--release` (`cargo asm`,
   hoặc đơn giản là đo thời gian cả hai dưới tải) và xác nhận chúng gần
   bằng nhau.
