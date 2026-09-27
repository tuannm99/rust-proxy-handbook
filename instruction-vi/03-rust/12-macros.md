# Macros

## What to learn

### Declarative macro: `macro_rules!`
`macro_rules!` pattern-match trên token stream tại call site và expand
thành một template cố định — hữu ích khi phần lặp đi lặp lại là cú pháp
(lặp lại *hình dạng* của code) chứ không phải logic (lặp lại *hành vi*,
điều mà một hàm hay generic thông thường đã xử lý được). Ưu tiên một hàm
hoặc generic trước; một macro chỉ đáng dùng khi những cái đó thực sự không
diễn đạt được thứ bạn cần.

```rust
macro_rules! metric_counter {
    ($name:ident) => {
        static $name: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    };
}
metric_counter!(REQUESTS_TOTAL);
metric_counter!(ERRORS_TOTAL);
```

### Hygiene
Các identifier được sinh bởi macro không vô tình capture hay đụng độ với
identifier tại call site — macro của Rust là hygienic, khác với `#define`
kiểu textual của C. Đây là lý do vì sao chúng an toàn hơn nhiều so với
preprocessor macro, và cũng là lý do vì sao chúng có thể khó debug hơn:
bạn không thể chỉ đơn giản thay thế văn bản trong đầu để hình dung ra kết
quả expand. `cargo expand` (một subcommand có thể cài thêm) là công cụ để
thực sự thấy một macro đã sinh ra cái gì.

### Nơi proc-macro thực sự xuất hiện ở đây
Bạn sẽ dùng derive và attribute macro liên tục mà không cần viết cái nào:
`#[derive(Serialize, Deserialize)]` (serde, config parsing —
`09-architecture/03-config.md`), `#[derive(thiserror::Error)]`
(`03-rust/08-error-handling.md`), `#[tokio::main]`/`#[tokio::test]`
(attribute macro viết lại `fn main()` thành thiết lập runtime cộng với
thân hàm của bạn). Viết một proc-macro từ đầu — một loại crate riêng,
dùng `syn`/`quote` để parse và sinh lại token stream — là kỹ năng thật
nhưng hiếm; phần lớn kỹ sư, kể cả senior, đi cả năm trời mà không viết cái
nào. Nhận ra một derive/attribute macro expand ra cái gì quan trọng hơn
nhiều trong công việc hàng ngày.

```rust
#[tokio::main] // attribute macro: expands to runtime setup + calls your async body
async fn main() { /* ... */ }
```

### Khi nào không nên dùng macro
Nếu một hàm generic, một trait, hoặc một builder
(`03-rust/13-api-design-and-modules.md`) có thể diễn đạt điều bạn muốn,
hãy ưu tiên chúng. Macro mờ đục với IDE tooling theo cách generics không
gặp phải (autocomplete yếu hơn, lỗi compiler trỏ vào chỗ expand thay vì
source của bạn), và chúng khó cho một người đọc trong tương lai — kể cả
chính bạn sau này — lần theo hơn so với một lời gọi hàm có tên.

## Practice
1. Cài `cargo expand` và chạy nó trên một struct nhỏ có
   `#[derive(Serialize, Deserialize)]`; đọc `impl` được sinh ra và xác
   định đại khái derive của serde đã tạo ra cái gì.
2. Viết một `macro_rules!` sinh ra một hàm kiểm tra boolean cho mỗi
   variant của enum (ví dụ `is_502!(status)`); rồi viết lại đúng thứ đó
   như một hàm hoặc trait method thông thường và quyết định cái nào dễ
   đọc hơn.
3. Chủ động shadow một tên biến bên trong một expansion `macro_rules!` và
   xác nhận, qua `cargo expand` hoặc một test nhanh, rằng nó không đụng
   độ với một biến cùng tên tại call site — đó là hygiene đang hoạt động.
4. Tìm một attribute macro đã có sẵn trong workspace này (`#[tokio::main]`
   trong bất kỳ `labs/*/src/main.rs` nào) và chạy `cargo expand --bin
   <name>` để thấy phần bootstrap runtime nó sinh ra.
5. Viết ra, bằng lời của bạn, vì sao bạn sẽ không dùng một proc-macro để
   giải quyết một vấn đề boilerplate trong `proxy` trước khi loại trừ một
   giải pháp bằng `macro_rules!`, generic, hoặc trait.
