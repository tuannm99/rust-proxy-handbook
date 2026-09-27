# Smart Pointers and Interior Mutability

## What to learn

### `Box<T>`: single ownership, trên heap
`Box<T>` là một vùng cấp phát trên heap với cùng quy tắc move/borrow như
bất kỳ giá trị owned nào, chỉ khác là được backing bởi bộ nhớ heap thay vì
stack. Nó xuất hiện cho: các giá trị quá lớn để di chuyển rẻ, các type đệ
quy (một type không thể chứa chính nó theo giá trị, nhưng có thể chứa một
`Box` của chính nó), và trait object (`Box<dyn Trait>`), vì `dyn Trait`
không có kích thước biết trước tại compile time.

```rust
struct Recursive { child: Box<Recursive> } // needs Box: without it, the type has infinite size
```

### `Rc<T>` vs `Arc<T>`: shared ownership, đơn luồng vs. đa luồng
`Rc<T>` là shared ownership dùng reference count cho code đơn luồng —
bản thân count không phải atomic, nên rẻ hơn nhưng không phải `Send`/
`Sync`. `Arc<T>` là cùng ý tưởng đó với một count atomic, an toàn để chia
sẻ xuyên thread. Trong một proxy async, gần như mọi thứ đều đi qua ranh
giới thread qua `tokio::spawn`, nên `Rc` hiếm khi xuất hiện — chủ yếu bên
trong một future đơn luồng có chủ đích trên một runtime `current_thread`
hoặc `LocalSet` (`04-runtime/03-runtime-config.md`). Mặc định dùng `Arc`
và chỉ hạ xuống `Rc` khi một giá trị được cố tình ghim vào một thread duy
nhất là thói quen an toàn hơn.

### `Cell<T>` và `RefCell<T>`: interior mutability, đơn luồng
Cả hai đều cho phép bạn mutate qua một shared reference (`&T`), điều mà
borrow checker thường cấm. `Cell<T>` hoạt động với các type `Copy` qua
`get`/`set` — không cần sổ sách gì tại runtime, chỉ là một phép hoán đổi
bộ nhớ thông thường. `RefCell<T>` hoạt động với mọi thứ qua
`.borrow()`/`.borrow_mut()`, enforce quy tắc aliasing của Rust (một
mutable *xor* nhiều immutable borrow) tại runtime thay vì compile time,
panic khi vi phạm thay vì fail compile.

```rust
use std::cell::RefCell;
let cache: RefCell<std::collections::HashMap<String, String>> = RefCell::new(Default::default());
cache.borrow_mut().insert("k".into(), "v".into()); // fine
// let a = cache.borrow(); let b = cache.borrow_mut(); // panics at runtime: already borrowed
```
Gotcha: giữ một borrow `RefCell` qua một điểm `.await` là một nguồn phổ
biến của panic "already borrowed" trong code async, vì một task khác có
thể chạy — và cũng thử borrow — trong khi task đầu tiên đang bị suspend.
Đây chính xác là hiểm họa mà `Mutex`/`Arc<Mutex<_>>` (`03-rust/04-sync.md`)
được thiết kế để làm cho không-thể-xảy-ra thay vì panic tại runtime.

### `Mutex<T>`/`RwLock<T>`: cùng một dải phổ, xuyên thread
Nơi `RefCell` enforce aliasing tại runtime cho code đơn luồng,
`Mutex<T>`/`RwLock<T>` enforce điều tương đương cho đa luồng, blocking
thay vì panic khi có contention — `03-rust/04-sync.md` bao quát chúng đầy
đủ; điểm của file này là cả bốn type đều nằm trên một dải phổ: được check
tại compile time (`&`/`&mut` thuần túy), được check tại runtime cho đơn
luồng (`Cell`/`RefCell`), và blocking cho đa luồng (`Mutex`/`RwLock`).

### `Cow<'a, T>`: tránh clone cho tới khi thực sự cần
`Cow` ("clone on write") giữ hoặc một borrowed reference hoặc một giá trị
owned, chỉ clone khi mutation thực sự cần thiết. Trên một đường
header-normalization (`07-security/04-normalization.md`), phần lớn request
không cần sửa đổi gì — trả về `Cow::Borrowed` cho trường hợp phổ biến và
chỉ allocate (`Cow::Owned`) khi một header thực sự cần được viết lại giúp
tránh một allocation trên hot path cho tuyệt đại đa số request.

```rust
fn normalize_header(v: &str) -> std::borrow::Cow<str> {
    if v.bytes().all(|b| !b.is_ascii_uppercase()) {
        std::borrow::Cow::Borrowed(v) // already normalized, zero allocation
    } else {
        std::borrow::Cow::Owned(v.to_ascii_lowercase())
    }
}
```

## Practice
1. Xây một `enum`/`struct` đệ quy nhỏ (ví dụ một config AST — xem
   `15-parser/03-ast.md`) cần `Box` để compile được; bỏ `Box` đi và đọc
   lỗi "infinite size" của compiler.
2. Thử chia sẻ một `Rc<RefCell<T>>` qua ranh giới `tokio::spawn` và đọc
   lỗi `Send` kết quả; sửa nó bằng cách chuyển sang `Arc<Mutex<T>>` (hoặc
   `Arc<parking_lot::Mutex<T>>`).
3. Chủ động giữ một borrow `RefCell` qua một `.await` trong một ví dụ
   async nhỏ để tái tạo panic "already borrowed", rồi giải thích vì sao
   phiên bản `Mutex` tương đương sẽ deadlock thay vì panic — và vì sao
   điều đó không thực sự tốt hơn. Liên hệ với `03-rust/04-sync.md`.
4. Viết một hàm header-normalization trả về `Cow<str>` (trong
   `labs/01-http-parser` hoặc bài tập normalization của `07-security`),
   và assert rằng đường đã-được-normalize không bao giờ allocate (ví dụ
   `matches!(result, Cow::Borrowed(_))`).
5. Đọc docs của `Arc::get_mut` và giải thích khi nào nó thành công (nó
   cần quyền truy cập độc quyền) — liên hệ điều này với việc vì sao nó
   hiếm khi được dùng trong một proxy giữ các `Arc` được chia sẻ qua nhiều
   task.
