# Async Trait Method và Dyn-Compatibility

## What to learn

### Vì sao `trait Foo { async fn bar(&self); }` không tự nhiên hoạt động như `dyn Foo`
`async fn` gốc trong trait (stable từ Rust 1.75) desugar thành một method
trả về một type `impl Future` mờ, do compiler sinh ra — và type mờ đó
khác nhau cho mỗi implementor. Một trait object (`dyn Foo`) cần một layout
vtable cố định, chia sẻ bởi mọi implementor, nhưng "một concrete future
type khác nhau cho mỗi impl" chính xác là thứ một vtable không thể tự
động erase đi được. Vậy `async fn` trong một trait compile ổn cho static
dispatch (`impl Foo`, generic) nhưng trait đó không dùng được như
`dyn Foo` mà không cần thêm việc.

```rust
trait Middleware {
    async fn handle(&self, req: Request) -> Response; // ổn với `impl Middleware`, không ổn với `dyn Middleware`
}
```

### Cách sửa tay: trả về một boxed future rõ ràng
Thay vì `async fn`, viết method trả về trực tiếp
`Pin<Box<dyn Future<Output = Response> + Send + '_>>`, và implement nó
bằng một block `async move` bọc trong `Box::pin`. Cách này làm return type
cụ thể và đồng nhất qua mọi implementor, đúng thứ một vtable cần.

```rust
trait Middleware: Send + Sync {
    fn handle<'a>(&'a self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>>;
}

impl Middleware for LoggingMiddleware {
    fn handle<'a>(&'a self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            tracing::info!("request in");
            self.inner.handle(req).await
        })
    }
}
```
Đây chính xác là hình dạng `Pin<Box<dyn Future>>` của
`03-rust/10-smart-pointers-and-interior-mutability.md` và lý do `Pin` tồn
tại của `03-rust/06-pin.md`, áp dụng ở một trait boundary thay vì bên
trong một executor viết tay.

### Crate `async-trait`: cùng cách sửa, qua macro
`#[async_trait]` trên một trait (và trên mỗi `impl`) viết lại các method
`async fn` thành đúng hình dạng boxed-future trên, tự động. Dùng nó thay
vì viết tay signature đã box khi một trait có nhiều method async — phiên
bản viết tay có signature dài dòng đến mức lặp lại nó bằng tay qua năm
method còn tệ hơn chấp nhận một allocation mỗi lần gọi do macro sinh ra.
Điều này nối với `03-rust/12-macros.md`: biết một derive/attribute macro
expand ra gì là thứ khiến việc dùng `async-trait` là một lựa chọn có chủ
đích thay vì "câu trả lời ai đó paste từ Stack Overflow".

### Chi phí thật: một allocation mỗi lần gọi
Mỗi lần gọi qua một trait method boxed-future allocate một `Box` cho
future của lần gọi đó, ngay cả khi caller chỉ bao giờ dùng một implementor
cụ thể và không hề cần dynamic dispatch ở call site cụ thể đó. Với một
middleware/plugin chain (`09-architecture/02-plugin.md`) được gọi mỗi
request, đây là một chi phí thật, đo được ở tốc độ request cao, không
phải lỗi làm tròn — profile nó (`08-observability/04-profiling.md`) trước
khi giả định nó ổn, và xem xét một enum các middleware đã biết dispatch
qua `match` (static dispatch) thay vì `Vec<Box<dyn Middleware>>` nếu tập
plugin thực ra cố định lúc compile.

### Khi bạn không cần `dyn` chút nào
Nếu mọi implementor của một trait được biết lúc compile (một tập cố định
các chiến lược load-balancing chọn từ config, không load như plugin), một
hàm generic hoặc một enum dispatch qua `match` tránh hoàn toàn vấn đề này
— `async fn` trong một trait hoạt động ổn ở đó, vì không có gì cần một
trait object `dyn`. Chỉ dùng pattern boxed-future khi bạn thực sự cần
runtime polymorphism (một plugin load từ config, một `Vec` các handler
không đồng nhất) — cùng quyết định static-vs-dynamic-dispatch như
`03-rust/07-traits-and-generics.md`, chỉ thêm async vào trên đó.

## Practice
1. Viết một trait với một method `async fn` gốc, implement nó cho hai
   type, và xác nhận `dyn YourTrait` fail compile với lỗi object-safety —
   đọc kỹ message lỗi.
2. Viết lại tay cùng trait đó để trả về
   `Pin<Box<dyn Future<Output = T> + Send + '_>>`, implement cho cùng hai
   type, và xác nhận `Box<dyn YourTrait>` giờ compile và hoạt động.
3. Làm lại cùng trait với `#[async_trait]` thay vào đó, và dùng
   `cargo expand` (từ `03-rust/12-macros.md`) để so sánh signature macro
   sinh ra với thứ bạn viết tay.
4. Trong `labs/14-plugin`, thiết kế trait middleware theo cả hai cách —
   như `Vec<Box<dyn Middleware>>` với boxed future, và như một enum cố
   định các middleware đã biết dispatch qua `match` — và benchmark (phần
   `criterion` của `03-rust/16-testing-idioms.md`) một call path nặng
   allocation so với cái còn lại.
5. Quyết định, và viết ra, việc chọn strategy của
   `06-proxy/02-load-balancer.md` trong `labs/06-load-balancer` có thực
   sự cần `dyn LoadBalancer` (cấu hình được lúc runtime) hay sẽ đúng
   tương tự, và nhanh hơn, như một enum lúc compile-time — giải thích câu
   trả lời của bạn bằng những gì format config thực sự cho phép.
