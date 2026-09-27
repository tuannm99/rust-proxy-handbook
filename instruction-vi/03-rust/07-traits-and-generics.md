# Traits and Generics

## What to learn

### Static dispatch (generics) vs dynamic dispatch (trait object)
Một hàm generic được monomorphize — compiler sinh ra một bản sao chuyên
biệt cho từng type cụ thể được dùng tại mỗi call site, với chi phí runtime
bằng không và hoàn toàn có thể inline, đánh đổi bằng code bloat (kích
thước binary tăng theo số lần instantiate). Một lời gọi `dyn Trait` đi qua
một vtable — chỉ một bản compile duy nhất bất kể có bao nhiêu concrete
type implement trait đó, nhưng lời gọi không thể được inline qua lớp
indirection của vtable, và trait phải object-safe.

```rust
trait LoadBalancer {
    fn pick(&self, key: &str) -> usize;
}

fn route_generic<T: LoadBalancer>(lb: &T, key: &str) -> usize { lb.pick(key) } // monomorphized per T
fn route_dyn(lb: &dyn LoadBalancer, key: &str) -> usize { lb.pick(key) }       // one vtable call
```
Gotcha: một plugin system ([`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md)) hoặc một chiến
lược load-balancing được chọn từ config lúc khởi động gần như luôn cần
`Box<dyn Trait>` — concrete type không được biết cho tới runtime, nên
generics không thể diễn đạt được lựa chọn đó.

### Trait bound và where clause
`T: Send + Sync + 'static` xuất hiện liên tục quanh `tokio::spawn`: một
future được spawn phải là `Send` để di chuyển giữa các worker thread, và
`'static` vì task có thể sống lâu hơn stack frame đã spawn nó. `where`
clause tồn tại thuần túy vì tính dễ đọc khi bound trở nên dài.

```rust
fn spawn_handler<F>(fut: F) where F: std::future::Future<Output = ()> + Send + 'static {
    tokio::spawn(fut);
}
```
Gotcha: một async fn giữ một giá trị không phải `Send` (một `Rc`, một
`MutexGuard` từ `std::sync::Mutex` được giữ qua một `.await`) qua một
điểm suspension sẽ sinh ra một future không phải `Send`, và lỗi chỉ hiện
ra tại call site của `tokio::spawn` — thường ở xa nguyên nhân thực sự.
Liên hệ điều này với desugaring của [`03-rust/05-async.md`](05-async.md): các field của
future được sinh ra chính xác là những gì còn sống qua mỗi `.await`, nên
chỉ một giá trị không phải `Send` bất kỳ đâu trong tập đó cũng làm "nhiễm
độc" toàn bộ future.

### Associated type vs generic parameter
Ưu tiên một associated type (`Iterator::Item`) khi có đúng một output type
hợp lý cho mỗi implementor; ưu tiên một generic parameter khi một type
thực sự implement trait theo nhiều cách khác nhau cho các type argument
khác nhau (`From<T>` cho nhiều `T`).

```rust
trait UpstreamPool {
    type Conn;
    fn acquire(&self) -> Self::Conn;
}
```

### Blanket impl và orphan rule
Orphan rule nói rằng bạn chỉ được `impl` một trait cho một type nếu bạn sở
hữu trait đó hoặc type đó — điều này ngăn hai crate viết các impl xung đột
nhau cho cùng một type ngoại lai. Cách giải quyết thực tế cho việc "thêm
method" vào một type bạn không sở hữu là pattern extension trait: định
nghĩa trait của riêng bạn, blanket-impl nó cho mọi type thỏa một bound mà
bạn *có thể* tham chiếu.

```rust
trait ResponseExt {
    fn is_upstream_error(&self) -> bool;
}
impl ResponseExt for http::Response<hyper::body::Incoming> {
    fn is_upstream_error(&self) -> bool { self.status().is_server_error() }
}
```

### Object safety
Không phải trait nào cũng có thể `dyn`-dispatch được: một method có một
generic type parameter, hoặc một method trả về `Self` theo giá trị, phá vỡ
object safety vì vtable không thể được xây mà không biết concrete type
tại call site. Các method của `dyn Trait` bị giới hạn ở receiver
`&self`/`&mut self`/`Box<Self>` không có generics — đây là một check tại
compile time, không phải một sở thích về style.

## Practice
1. Viết cả một phiên bản generic lẫn một phiên bản `dyn Trait` cho một
   hàm load-balancer `pick(&self, key: &str) -> usize`, build cả hai ở
   chế độ release, và so sánh kích thước binary (`cargo bloat` hoặc `size`
   thông thường) để thấy trực tiếp chi phí của monomorphization.
2. Chủ động viết một async fn giữ một `Rc<RefCell<_>>` qua một `.await`,
   thử `tokio::spawn` nó, và đọc lỗi compiler đủ kỹ để gọi tên chính xác
   bound nào đã fail.
3. Trong [`labs/06-load-balancer`](../../labs/06-load-balancer), định nghĩa một trait `LoadBalancer` và
   implement nó cho các chiến lược round-robin, least-conn, và
   consistent-hash; chọn chiến lược cụ thể tại runtime từ một chuỗi config
   qua `Box<dyn LoadBalancer>`.
4. Viết một extension trait cho một type bạn không sở hữu (ví dụ
   `http::HeaderMap`) và giải thích vì sao orphan rule sẽ chặn việc
   implement trực tiếp một trait ngoại lai lên nó thay vào đó.
5. Thêm một generic method vào một trait và thử dùng nó như `dyn Trait`;
   đọc lỗi object-safety và xác định chính xác nó vi phạm quy tắc nào.
