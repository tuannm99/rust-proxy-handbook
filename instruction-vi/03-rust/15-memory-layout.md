# Memory Layout and Representation

## What to learn

### `repr(Rust)` vs. `repr(C)`: compiler có thể sắp xếp lại field của bạn
Không có một `#[repr(...)]` tường minh, compiler được tự do sắp xếp lại
field của struct để giảm padding — hai struct có cùng danh sách field có
thể có layout khác nhau, và thứ tự field trong bộ nhớ không cần khớp thứ
tự khai báo. Điều này phần lớn không quan trọng với code Rust thuần túy
(field được truy cập theo tên), nhưng quan trọng ngay khi bạn cần một
layout ổn định: FFI ([`03-rust/14-ffi-and-abi.md`](14-ffi-and-abi.md)), hoặc suy luận về việc
đóng gói cache-line cho một struct nóng ([`17-performance/01-cpu-cache.md`](../17-performance/01-cpu-cache.md)).

```rust
struct A { a: u8, b: u64, c: u8 }              // repr(Rust): compiler may pack tighter than declared order
#[repr(C)] struct B { a: u8, b: u64, c: u8 }   // fields stay in declared order, C-style padding rules
```

### Size và alignment: `size_of`, `align_of`, và padding
Mọi type đều có một kích thước và một yêu cầu alignment; tổng kích thước
của một struct thường được làm tròn lên tới bội số của alignment lớn nhất
trong các field, đây là lý do vì sao *thứ tự* field ảnh hưởng tới tổng
kích thước kể cả dưới `repr(C)` — nhóm các field cùng kích thước lại với
nhau giảm thiểu padding lãng phí.

```rust
use std::mem::size_of;
#[repr(C)] struct Bad { a: u8, b: u64, c: u8 }  // u64's 8-byte alignment forces padding around both u8s
#[repr(C)] struct Good { b: u64, a: u8, c: u8 } // both u8s pack together after the u64
assert!(size_of::<Good>() < size_of::<Bad>());
```
Với một struct được cấp phát trên mỗi connection ở quy mô lớn — hàng chục
nghìn connection còn sống trong một proxy — kiểu khác biệt padding này là
bộ nhớ thật, không phải một micro-optimization. Xem
[`17-performance/01-cpu-cache.md`](../17-performance/01-cpu-cache.md) và [`17-performance/04-memory-layout.md`](../17-performance/04-memory-layout.md)
để có phần xử lý sâu hơn về đóng gói cache-line mà file này là điều kiện
tiên quyết.

### Niche optimization: vì sao `Option<&T>` cùng kích thước với `&T`
Compiler tận dụng các bit pattern không hợp lệ của một type — một
reference không bao giờ có thể null — để biểu diễn `None` bằng chính bit
pattern-không-thể-có đó, nên `Option<&T>`, `Option<Box<T>>`, và
`Option<NonZeroU32>` đều không tốn thêm byte nào so với type không-Option.
Đây là một tối ưu hóa có tính load-bearing, không phải một điều thú vị:
đó là lý do cụ thể vì sao Rust idiomatic dùng `Option<NonZeroU32>` thay vì
`Option<u32>` cho một id không bao giờ hợp lệ là số không.

```rust
assert_eq!(size_of::<Option<&u8>>(), size_of::<&u8>()); // niche-optimized: zero extra cost
assert_eq!(size_of::<Option<std::num::NonZeroU32>>(), size_of::<u32>());
```

### Layout của enum: tag cũng tốn không gian
Một enum thông thường với các variant mang dữ liệu được định cỡ để vừa
với variant lớn nhất cộng với một tag phân biệt; một `Result<SmallOk,
HugeErr>` phải trả giá cho kích thước của `HugeErr` kể cả trên đường
thành công. Nếu variant lỗi của một `Result` trên hot path mang thứ gì đó
lớn (cả một struct request), boxing nó (`Result<T, Box<HugeErr>>`) thu
nhỏ type lại cho trường hợp phổ biến.

```rust
enum Small { A, B(u8) }              // small
enum Mixed { A, B([u8; 256]) }       // sized for the 256-byte variant even when it's A
```

### Vì sao điều này liên hệ tới buffer pool và arena
Các thiết kế arena/slab/object-pool trong [`14-memory/`](../14-memory) đều giả định bạn
biết chính xác kích thước và alignment của thứ bạn đang lưu trữ — một slab
allocator ([`14-memory/05-slab-allocator.md`](../14-memory/05-slab-allocator.md)) không thể được thiết kế mà
không biết chính xác mỗi slot cần lớn bao nhiêu, đó chính là `size_of`/
`align_of` áp dụng lên struct connection-state thực tế của bạn.

## Practice
1. Lấy một struct connection-state trên mỗi connection từ
   [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), in `std::mem::size_of::<T>()`, sắp xếp lại
   field theo kích thước giảm dần, và đo xem nó có nhỏ lại không.
2. Tự xác nhận tuyên bố về niche optimization: so sánh
   `size_of::<Option<u32>>()` với `size_of::<Option<std::num::NonZeroU32>>()`
   và với `size_of::<Option<&u32>>()`.
3. Xây một `Result<T, E>` với `E` là một struct lớn, đo
   `size_of::<Result<T, E>>()`, rồi box `E` lại và đo lần nữa.
4. Đọc [`14-memory/05-slab-allocator.md`](../14-memory/05-slab-allocator.md) và liên hệ logic định cỡ slot của
   nó với phần thảo luận `size_of`/`align_of` của file này, bằng lời của
   bạn.
5. Thêm `#[repr(C)]` vào một struct hiện đang là `repr(Rust)` và dùng
   `size_of` (hoặc `std::mem::offset_of!` nếu toolchain của bạn có) để
   kiểm tra xem layout có thực sự thay đổi hay không.
