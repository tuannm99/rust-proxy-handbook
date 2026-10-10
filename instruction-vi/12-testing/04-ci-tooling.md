# CI & Static Tooling

Sự kiểm chứng chạy trước khi proxy xử lý một request thật nào — linting,
formatting, và phát hiện undefined behavior cho chính workspace.

## What to learn
### clippy + rustfmt như một gate bắt buộc
`cargo clippy --workspace --all-targets -- -D warnings` và `cargo fmt
--check` rẻ (vài giây) và bắt được một lớp lớn bug và sự trôi dạt về style
trước khi bất cứ thứ gì đắt hơn chạy. `-D warnings` quan trọng một cách cụ
thể — không có nó, các lint của clippy chỉ mang tính khuyến nghị và bị bỏ
qua dưới áp lực deadline; có nó, một lint thất bại làm fail build giống
hệt một lỗi compile.

### miri cho riêng phần code unsafe
[`labs/01-http-parser`](../../labs/01-http-parser) chính xác là loại crate nhiều khả năng nhất chứa
`unsafe` thật (raw pointer vào buffer) — xem [`03-rust/03-unsafe.md`](../03-rust/03-unsafe.md).
`cargo +nightly miri test` chạy test code dưới một interpreter phát hiện
undefined behavior mà phần cứng thật sẽ âm thầm bỏ qua: truy cập vượt giới
hạn, use-after-free, data race, pointer arithmetic không hợp lệ. Hãy biết
giới hạn của nó: miri không thể thực thi syscall thật, nên các lệnh gọi
`epoll_wait`/`libc` thật của bài tập raw-epoll ([`02-linux/14-epoll.md`](../02-linux/14-epoll.md))
không thể chạy trực tiếp dưới nó — miri dùng để test logic unsafe thuần
Rust (một buffer pool tự viết tay, pointer arithmetic trong parser) tách
biệt khỏi các syscall xung quanh nó.

```yaml
# .github/workflows/ci.yml (trích đoạn)
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
  miri:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: rustup +nightly component add miri
      - run: cargo +nightly miri test -p http-parser
```
Gotcha: đừng chạy `cargo miri test` trên *toàn bộ* workspace theo mặc định
— các crate mở socket thật (`tcp-server`, `reverse-proxy`, ...) sẽ fail
hoặc hang dưới miri vì những lý do chẳng liên quan gì tới sự unsafe trong
code của bạn. Giới hạn nó vào các crate mà logic unsafe thực sự đang được
kiểm chứng.

### cargo-deny / cargo-audit
Một khi workspace phụ thuộc vào các crate hướng-mạng (thư viện TLS, hyper,
tokio), việc audit ở mức dependency trở nên quan trọng: `cargo audit`
kiểm tra cây dependency với cơ sở dữ liệu advisory RustSec để tìm các lỗ
hổng đã biết; `cargo deny check` có thể còn thực thi chính sách license và
cấm các phiên bản crate trùng lặp/không mong muốn. Cả hai gần như tức thời
với một `Cargo.lock` đã resolve sẵn — không có lý do gì để bỏ qua chúng
một khi workspace có dependency thật, mà nó đã có rồi (`tokio-rustls`
trong [`proxy`](../../proxy)).

### Cái gì block một commit so với cái gì chạy theo lịch
Các kiểm tra nhanh (fmt, clippy, unit test, miri trên các crate unsafe
nhỏ) thuộc về mỗi lần push — chúng mất vài giây tới vài phút và cho phản
hồi tức thì. Các kiểm tra chậm (regression corpus fuzzing từ
[`12-testing/02-fuzzing.md`](02-fuzzing.md), load test từ [`12-testing/01-load-testing.md`](01-load-testing.md),
chaos run từ [`12-testing/03-chaos.md`](03-chaos.md)) thuộc về một lịch trình (hàng đêm,
hoặc trên các nhánh release) — chạy một load test nhiều phút trên mỗi lần
push chỉ làm chậm việc lặp lại mà không thêm tín hiệu tương xứng cho hầu
hết các commit.

## Practice
1. Thêm một workflow GitHub Actions (hoặc tương đương) chạy `cargo fmt
   --check` và `cargo clippy --workspace --all-targets -- -D warnings`
   trên mỗi lần push; sửa bất cứ thứ gì clippy gắn cờ (nên gần như bằng 0
   ở trạng thái stub, nhưng chạy lại sau mỗi bài tập bạn implement).
2. Thêm một job `miri` giới hạn cụ thể vào [`labs/01-http-parser`](../../labs/01-http-parser) (không
   phải toàn bộ workspace); xác nhận nó pass trên code stub và chạy lại
   khi bạn implement logic parsing unsafe.
3. Thêm `cargo-deny` với một `deny.toml` ít nhất cấm các advisory đã biết
   là có lỗ hổng; chạy nó một lần trên tập dependency hiện tại và sửa bất
   cứ thứ gì nó gắn cờ.
4. Chia workflow thành một job nhanh (fmt/clippy/miri, trên mỗi lần push)
   và một job riêng theo lịch/thủ công cho bất cứ thứ gì từ
   [`12-testing/01-load-testing.md`](01-load-testing.md) hoặc [`12-testing/02-fuzzing.md`](02-fuzzing.md) mất
   hơn một hai phút để chạy.
