# Unsafe Rust

## What to learn

### `unsafe` thực sự mở khóa cái gì
`unsafe` không tắt borrow checker hay type checker — nó mở khóa đúng năm
khả năng bổ sung: dereference raw pointer, gọi các `unsafe` fn (bao gồm
FFI), implement các `unsafe` trait, mutate một `static`, và truy cập field
của union. Mọi quy tắc khác của Rust vẫn áp dụng bên trong một block
`unsafe`. Viết `unsafe` là một lời hứa với compiler rằng *chính bạn* đã tự
tay kiểm chứng một invariant mà nó không thể check được.

```rust
let x = 5;
let p = &x as *const i32;
unsafe {
    println!("{}", *p); // dereferencing a raw pointer requires unsafe
}
```

### Hợp đồng của unsafe
Mỗi `unsafe fn` có các tiền điều kiện ngầm định ("safety invariant") mà
caller phải giữ đúng — những điều này thuộc về một comment `// SAFETY:`
tại call site giải thích *vì sao* invariant đó đúng, không phải code làm
gì. Một proxy được xây cho throughput sớm muộn cũng sẽ cần `unsafe`
(buffer pool tự viết, FFI vào `epoll`/`io_uring` qua `libc`) — kỷ luật
viết ra lập luận về an toàn là thứ giữ cho nó sound khi code xung quanh
thay đổi.

```rust
// SAFETY: `len` bytes were just initialized by the read_exact call above,
// and buf.capacity() >= len was checked on the line before it.
unsafe { buf.set_len(len); }
```

### Các pattern unsafe phổ biến trong code networking hiệu năng cao
- Tính toán con trỏ thô vào một byte buffer để tránh chi phí bounds-check
  trong một vòng lặp parsing nóng (thường không đáng làm trước khi
  profiling chứng minh điều đó).
- FFI vào `libc` cho `epoll_ctl`/`epoll_wait`, các socket option thô
  (`setsockopt` cho `SO_REUSEPORT`, `TCP_NODELAY`), hoặc `io_uring` — xem
  `02-linux/07-epoll.md`, `02-linux/08-io_uring.md`.
- `Vec::set_len` sau khi ghi vào spare capacity lấy được qua
  `spare_capacity_mut`, để tránh phải zero-initialize một read buffer
  trước khi một syscall `read()` điền đầy nó.
- Implement `unsafe impl Send`/`Sync` cho một wrapper type khi bạn đã tự
  tay kiểm chứng tính thread-safety mà compiler không thể suy ra (ví dụ
  một ring buffer lock-free tự viết).

Gotcha: một `unsafe impl Send` không sound trên một type thực ra không an
toàn để di chuyển giữa các thread vẫn compile được bình thường và chỉ sinh
ra data race hoặc UB dưới một timing cụ thể — đúng kiểu bug sẽ không xuất
hiện trong một test đơn luồng nhưng sẽ xuất hiện dưới tải trong production.

### Giữ unsafe tối thiểu và sound
Bọc mỗi thao tác `unsafe` trong một hàm safe nhỏ nhất có thể với một tên
và signature khiến việc dùng sai trở nên khó, và giữ phần check invariant
ngay cạnh code unsafe (một `assert!` ngay trước một block `unsafe` phụ
thuộc vào nó là một khoản bảo hiểm rẻ). Chạy `cargo miri test` trên các
module dùng nhiều unsafe — Miri bắt được một lớp lớn undefined behavior
(truy cập out-of-bounds, pointer provenance không hợp lệ, data race) vốn
vẫn compile và "chạy được" dưới `cargo test` bình thường.

## Practice
1. Viết một hàm wrapper an toàn quanh `Vec::set_len` nhận vào một closure
   ghi vào `spare_capacity_mut` và trả về `Vec` đã đúng độ dài — không bao
   giờ để `set_len` là một API public.
2. Cài và chạy `cargo miri test` trên một type buffer-pool unsafe nhỏ;
   cố tình đưa vào một lỗi ghi out-of-bounds và xác nhận Miri bắt được
   nó.
3. Trong bài tập raw-epoll echo server từ `02-linux/07-epoll.md`, xác định
   mọi lời gọi `unsafe` bạn cần (tạo socket, `epoll_ctl`, `epoll_wait` qua
   `libc`) và viết một comment `// SAFETY:` cho từng cái trước khi chạy
   code.
4. Tìm (qua docs.rs hoặc source) một `unsafe impl Send` thật trong một
   crate bạn đang phụ thuộc (tokio hoặc hyper) và giải thích bằng lời của
   bạn vì sao nó sound.
