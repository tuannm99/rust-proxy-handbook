# FFI and ABI

## What to learn

### `repr(C)` và vì sao layout quan trọng ở ranh giới FFI
Layout mặc định của Rust (`repr(Rust)`) cố tình không được đặc tả —
compiler được tự do sắp xếp lại field để đóng gói tốt hơn, và hai struct
có cùng field không được đảm bảo cùng layout. `#[repr(C)]` ghim layout
theo quy tắc của C (field theo đúng thứ tự khai báo, alignment và padding
tiêu chuẩn), điều này bắt buộc bất cứ khi nào một struct đi qua ranh giới
FFI — thiếu nó, hai phía có thể âm thầm bất đồng về việc mỗi field nằm ở
đâu.

```rust
#[repr(C)]
struct SockAddrLike { family: u16, port: u16, addr: u32 } // layout now matches C's expectations
```
Đây chính xác là lý do vì sao các lời gọi `libc` (`epoll_ctl`,
`setsockopt`) trong [`03-rust/03-unsafe.md`](03-unsafe.md) hoạt động được: mọi struct
`libc` đưa cho bạn đều là `#[repr(C)]`.

### Gọi vào C: `extern "C"` và crate `libc`
`extern "C"` trên một hàm khai báo (hoặc, khi export cho C, định nghĩa)
calling convention của C. Crate `libc` phần lớn là một tập lớn các khai
báo `extern "C"` cộng với các struct `#[repr(C)]` khớp với header của nền
tảng — bạn hiếm khi tự viết những thứ này cho các syscall phổ biến vì
`libc` đã có sẵn; bạn chỉ tự viết khi bind một thư viện mà `libc` chưa bao
phủ.

```rust
extern "C" { fn getpid() -> i32; } // hand-written binding, though libc::getpid() already exists
unsafe { let pid = getpid(); }
```
Gotcha: mọi lời gọi FFI đều là `unsafe`, vì compiler không thể kiểm chứng
hợp đồng của hàm ngoại lai — nó có mong đợi một chuỗi kết thúc bằng null
không? nó có lấy ownership của một con trỏ bạn truyền vào không? nó có
thread-safe để gọi từ bất kỳ thread nào không? Kỷ luật comment
`// SAFETY:` từ [`03-rust/03-unsafe.md`](03-unsafe.md) càng quan trọng hơn ở đây, không
kém đi, vì invariant nằm trong tài liệu của người khác, không có borrow
checker nào để đối chiếu chéo.

### Ownership qua ranh giới FFI
Câu hỏi khó nhất của FFI luôn là "ai giải phóng cái này, và khi nào." Nếu
một hàm C trả về một con trỏ heap, nó có mong đợi bạn gọi hàm free tương
ứng của nó không, hay giờ Rust sở hữu nó? Sai chỗ này là một double-free
hoặc một leak, và cả hai đều không hiện ra cho tới khi chúng hiện ra. Cách
sửa chuẩn là một wrapper type của Rust có `Drop` impl gọi đúng hàm free
của C đúng một lần, được enforce bởi type system ở phía Rust dù phía C
không có sự enforce nào như vậy.

```rust
struct CBuf(*mut u8);
impl Drop for CBuf {
    fn drop(&mut self) {
        // SAFETY: self.0 was allocated by this library and never freed elsewhere.
        unsafe { free_from_c_lib(self.0); }
    }
}
```

### Sinh binding: `bindgen` và `cbindgen`
`bindgen` sinh các khai báo `extern "C"` của Rust từ một header C (Rust
gọi vào C); `cbindgen` sinh một header C từ code Rust `#[repr(C)]`/
`extern "C"` (C gọi vào Rust — liên quan nếu [`proxy`](../../proxy) từng để lộ một bề
mặt plugin C-ABI thay vì một bề mặt `dyn Trait` thuần Rust,
[`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md)). Cả hai tồn tại vì việc tự tay duy trì
binding đồng bộ với một header hay API đang thay đổi chính xác là công
việc tẻ nhạt, dễ lỗi mà một công cụ nên đảm nhận.

### Tính ổn định ABI: vì sao plugin Rust thường là `dyn Trait`, không phải `dylib`
Rust không có một ABI ổn định xuyên các phiên bản compiler — một `dylib`
được build với một phiên bản rustc không được đảm bảo load được bởi một
binary được build với phiên bản khác. Đây là lý do cụ thể vì sao hệ thống
plugin trong-process của [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md) compile các
plugin vào cùng một binary dưới dạng các object `dyn Trait` (hoặc để lộ
một ranh giới C-ABI `#[repr(C)]` nếu việc load động thực sự cần thiết)
thay vì `dlopen` một `.so` Rust bất kỳ — cách sau chỉ hoạt động đáng tin
cậy nếu mọi plugin và host dùng chung chính xác cùng một bản build rustc,
một yêu cầu vận hành mong manh.

## Practice
1. Viết một hàm C nhỏ, gọi nó từ Rust qua một block `extern "C"` viết tay,
   xác nhận nó hoạt động, rồi làm hỏng nó bằng cách sai kiểu tham số và
   quan sát rằng đây là undefined behavior, không phải một lỗi compile.
2. Thêm `#[repr(C)]` vào một struct dùng qua một ranh giới FFI trong một
   ví dụ nhỏ, gỡ nó ra, và dùng `std::mem::size_of` để xác nhận layout
   thực sự có thể khác đi mà không có nó.
3. Viết một wrapper dựa trên `Drop` quanh một resource được cấp phát bởi
   C (một hàm "C" giả lập cũng được) và xác nhận double-free/use-after-free
   được ngăn chặn bởi cấu trúc.
4. Đọc source của crate `libc` cho một binding syscall bạn đã dùng bằng
   tay (`epoll_ctl`, từ bài tập [`02-linux/14-epoll.md`](../02-linux/14-epoll.md)) và xác định các
   định nghĩa struct `#[repr(C)]` của nó.
5. Giải thích, có trích dẫn [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md), vì sao một hệ
   sinh thái ổn định ABI như C khiến plugin kiểu `dlopen` khả thi trong
   khi việc Rust thiếu ABI ổn định đẩy bạn về hướng compile plugin vào
   cùng một binary thay vào đó.
