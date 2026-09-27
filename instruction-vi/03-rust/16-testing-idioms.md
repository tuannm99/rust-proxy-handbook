# Testing Idioms

## What to learn

### Unit test: `#[cfg(test)] mod tests`
Idiom chuẩn là một `mod tests` ở cuối cùng file, gắn `#[cfg(test)]` để nó
chỉ compile khi `cargo test`, với quyền truy cập vào các item private
trong module đó — điều bên ngoài crate không làm được. Đây là lý do unit
test của Rust sống ngay cạnh code, không phải trong một cây file test đối
xứng như nhiều ngôn ngữ khác.

```rust
fn parse_content_length(s: &str) -> Option<u64> { s.trim().parse().ok() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_negative() {
        assert_eq!(parse_content_length("-1"), None);
    }
}
```

### Integration test: thư mục `tests/`
Mỗi file trực tiếp dưới `tests/` compile như crate riêng của nó, chỉ link
với `pub` API của library — nó thực thi crate theo cách một consumer bên
ngoài sẽ làm, bắt được những lỗ hổng kiểu "compile ổn bên trong crate,
nhưng public API không thể làm được điều này" mà unit test bỏ lỡ. Gotcha:
code setup chia sẻ phải nằm trong một submodule (`tests/common/mod.rs`),
không phải `tests/common.rs` ở top-level, không thì cargo coi nó là một
test binary riêng không có test nào và cảnh báo.

### Doctest: ví dụ phải luôn compile được
Một fenced block Rust bên trong doc comment `///` được compile và chạy như
một test bởi `cargo test`. Đây là loại test Rust duy nhất đảm bảo các ví
dụ trong documentation của bạn không bao giờ âm thầm mục nát — một bug
thật rất phổ biến ở các ngôn ngữ khác đơn giản là không thể xảy ra ở đây
vì nó được compile-check.

````rust
/// Parses a CIDR prefix length.
///
/// ```
/// assert_eq!(my_crate::prefix_len("/24"), Some(24));
/// ```
pub fn prefix_len(s: &str) -> Option<u8> { s.strip_prefix('/')?.parse().ok() }
````
Gotcha: một doctest "chỉ chạy" mà không assert gì tạo ra sự tự tin giả —
viết doctest thực sự check một giá trị, không phải loại chỉ chứng minh
code không panic.

### Mock qua trait, không cần một mocking framework
Mock idiomatic của Rust là một implementation thứ hai, chỉ dùng cho test,
của cùng trait mà code production đã phụ thuộc vào — không cần framework,
vì trait boundary đã sẵn là đường nối. Thiết kế cho testability nghĩa là
code đang test phụ thuộc vào `dyn UpstreamPool` (hoặc một generic
`P: UpstreamPool`), không phải một `HyperUpstreamPool` cụ thể — cùng quyết
định dispatch như [`03-rust/07-traits-and-generics.md`](07-traits-and-generics.md), chỉ áp dụng cho một
lý do khác.

```rust
trait Clock { fn now(&self) -> std::time::Instant; }
struct FixedClock(std::time::Instant);
impl Clock for FixedClock { fn now(&self) -> std::time::Instant { self.0 } }
// code production nhận `impl Clock` / `Arc<dyn Clock>`; test inject FixedClock
```
Gotcha: đừng thêm một trait chỉ để làm cái gì đó mockable nếu nó chỉ có
đúng một implementation thật và sẽ không bao giờ có cái thứ hai — đó là
abstraction quá sớm. Dùng nó khi thứ bạn mock vốn không xác định trong
test (thời gian, network, ngẫu nhiên), không phải như một pattern mặc
định.

### Table-driven test
Một danh sách cặp `(input, expected)` được loop qua trong một hàm
`#[test]` giữ một danh sách case lớn dễ đọc và biến việc thêm một case mới
thành một diff một dòng — cách thay thế idiomatic của Rust cho việc viết N
hàm test gần giống nhau.

```rust
#[test]
fn parses_hop_by_hop_headers() {
    let cases = [("Connection", true), ("Content-Type", false)];
    for (header, expected) in cases {
        assert_eq!(is_hop_by_hop(header), expected, "header={header}");
    }
}
```
Gotcha: luôn kèm input thật của dòng bị fail vào message assertion
(`"header={header}"` ở trên) — không có nó, một table-driven test fail chỉ
cho bạn biết *có* thứ gì đó hỏng, không cho biết *dòng nào*.

### Property-based testing với `proptest`
Thay vì tự chọn tay các input mẫu, `proptest` sinh hàng trăm input ngẫu
nhiên khớp một hình dạng bạn mô tả và assert một invariant giữ đúng cho
tất cả, tự động shrink một case fail xuống input nhỏ nhất vẫn còn fail.
Điều này quan trọng nhất cho parser và encoder
([`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md), [`labs/01-http-parser`](../../labs/01-http-parser)), nơi các bug thú vị
nằm ở những input không ai nghĩ ra để viết tay.

```rust
proptest::proptest! {
    #[test]
    fn roundtrips_through_encode_decode(n in 0u64..u64::MAX) {
        let encoded = encode_varint(n);
        prop_assert_eq!(decode_varint(&encoded), Some(n));
    }
}
```

### Benchmark với `criterion`
`#[bench]` chỉ chạy trên nightly không phải mặc định thực tế; `criterion`
chạy một benchmark đủ nhiều lần để có một đo lường có ý nghĩa thống kê và
theo dõi regression so với lần chạy trước — đó là thứ bạn dùng trước khi
tuyên bố một thay đổi "nhanh hơn". Một timing tay bằng `Instant::now()`
quanh một loop quá nhiễu để tin được ngoài một sanity check thô.

## Practice
1. Thêm một `#[cfg(test)] mod tests` vào một hàm parse private trong
   [`labs/01-http-parser`](../../labs/01-http-parser) và viết một test gọi trực tiếp một helper private
   — xác nhận nó sẽ không compile từ ngoài crate.
2. Viết một integration test trong `tests/` cho [`labs/03-router`](../../labs/03-router) chỉ dùng
   `pub` API của crate, và cố tình chạm vào một field private từ đó để
   xác nhận compiler chặn bạn.
3. Viết một doctest cho một hàm public có `assert_eq!` thật trong đó, rồi
   phá hàm đó và xác nhận `cargo test` fail ở doctest.
4. Định nghĩa một trait `Clock` (hoặc dependency non-deterministic tương
   tự) trong [`labs/11-rate-limit`](../../labs/11-rate-limit), inject một `FixedClock` trong test, và
   viết một test sẽ flaky nếu không có nó.
5. Viết một test round-trip bằng `proptest` cho một cặp encode/decode
   trong workspace của bạn (một hàm normalize header value, một varint
   encoder) và để nó tìm ra một edge case bạn không nghĩ ra bằng tay.
6. Cài `criterion` cho một hàm hot-path (một lượt tra routing, một lần
   parse header) và ghi lại baseline trước khi thực hiện một thay đổi
   performance, để bạn có thể chứng minh thay đổi đó thực sự có hiệu quả.
