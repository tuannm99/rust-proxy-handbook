# API Design and Modules

## What to learn

### Module system như một ranh giới API
`mod`/`pub`/`pub(crate)`/`pub(super)` kiểm soát không chỉ tổ chức mà còn
cả những gì thực sự thuộc về hợp đồng của bạn. Bề mặt `pub` của một crate
là thứ code downstream có thể làm hỏng nếu dựa vào nó; mọi thứ khác có thể
được tái cấu trúc tự do. Mặc định chọn mức visibility thấp nhất và chỉ mở
rộng khi thứ gì đó bên ngoài module thực sự cần — thu hẹp lại visibility
sau này là một breaking change.

```rust
pub mod pool {
    pub struct ConnectionPool { conns: Vec<Conn> } // fields private by default, even in a pub struct
    impl ConnectionPool {
        pub fn acquire(&mut self) -> Option<Conn> { self.conns.pop() } // the real API
    }
}
```

### Newtype pattern
Bọc một type nguyên thủy hoặc ngoại lai trong một struct một field
(`struct RequestId(u64);`) mang lại ba thứ: một type khác biệt mà compiler
sẽ không cho bạn vô tình lẫn với một `u64` thuần túy (một bug kinh điển —
truyền một số lượng connection vào chỗ cần một port number, cả hai đều là
`usize`), khả năng implement các trait bạn không sở hữu cho một type bạn
không sở hữu (né được orphan rule từ [`03-rust/07-traits-and-generics.md`](07-traits-and-generics.md)),
và một chỗ để enforce invariant trong một constructor trong khi vẫn giữ
giá trị bên trong private.

```rust
pub struct UpstreamAddr(std::net::SocketAddr); // distinct from a raw SocketAddr used for the client's own address
```

### Builder pattern
Với một type có nhiều field tùy chọn (config theo route, một client
builder), một builder tránh được cả một constructor mười tham số theo vị
trí (khó đọc, dễ vô tình đổi chỗ hai tham số cùng type) lẫn cách dùng
struct-literal với các field `pub` (không có điểm validate nào). Mỗi
method của builder nhận/trả về `self`, và `.build()` cuối cùng validate và
tạo ra type thật.

```rust
RouteConfig::builder()
    .path("/api")
    .upstream_pool(pool)
    .timeout(std::time::Duration::from_secs(5))
    .build()?; // validates, e.g. rejects a route with no upstream configured
```

### Sealed trait
Một trait "sealed" đủ public để dùng làm bound, nhưng mang theo một
supertrait private (hoặc nằm sau một module không thể tiếp cận từ ngoài)
ngăn bất kỳ ai bên ngoài crate của bạn implement nó. Điều này cho phép bạn
thêm method vào trait sau này mà không phải một breaking change, vì không
có implementation bên ngoài nào tồn tại có thể xung đột.

```rust
mod private { pub trait Sealed {} }
pub trait Middleware: private::Sealed { fn handle(&self, req: Request) -> Response; }
```

### Kỷ luật semver và tính ổn định của API
Thêm một field `pub`, một variant mới vào một enum `pub` không có
`#[non_exhaustive]`, hay một method trait bắt buộc mới đều là breaking
change dưới semver dù chúng chỉ "thêm vào" — code hiện có match hoặc
implement đầy đủ theo hình dạng cũ sẽ ngừng compile. `#[non_exhaustive]`
trên một struct/enum bạn kỳ vọng sẽ lớn lên cho phép bạn thêm field/variant
mà không gây ra breaking change đó, đánh đổi bằng việc caller không bao
giờ có thể construct hoặc match đầy đủ nó trực tiếp.

## Practice
1. Lấy một struct trong [`labs/06-load-balancer`](../../labs/06-load-balancer) có vài field public và
   tái cấu trúc nó để giữ các field private đứng sau một builder có
   `.build()` validate.
2. Đưa một newtype wrapper quanh một `SocketAddr` hoặc `u64` id thô ở chỗ
   nào đó đang là một kiểu nguyên thủy trần trụi, và tìm (qua lỗi
   compiler) mọi nơi đang dựa vào việc nó có thể hoán đổi với type thô.
3. Đánh dấu một enum bạn sở hữu bằng `#[non_exhaustive]`, thêm một
   variant, và xác nhận các `match` đầy đủ hiện có bên ngoài module định
   nghĩa giờ ngừng compile — rồi sửa chúng bằng một nhánh wildcard.
4. Viết một sealed trait cho một abstraction kiểu `Middleware` nhỏ, và
   xác nhận từ một module riêng rằng code bên ngoài không thể implement
   nó.
5. Chọn một item `pub` trong một crate [`labs/`](../../labs) và viết ra việc thay đổi
   hình dạng của nó sẽ làm hỏng gì cho một downstream user giả định —
   quyết định xem nó có thực sự nên là `pub` không.
