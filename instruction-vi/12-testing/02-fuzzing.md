# Fuzzing

## What to learn
### Kiến thức cơ bản về cargo-fuzz / AFL
`cargo-fuzz` bọc libFuzzer: nó compile code của bạn với sanitizer và
instrumentation coverage, rồi mutate một input dạng byte-string, đưa nó
vào một hàm `fuzz_target!(|data: &[u8]| { ... })` bạn viết, được dẫn dắt
bởi việc mutation nào chạm tới các nhánh code mới. AFL hoạt động tương tự
nhưng như một công cụ instrumentation binary bên ngoài, hữu ích khi bạn
không dễ dàng link được libFuzzer. Với handbook này, `cargo-fuzz` là lựa
chọn idiomatic hơn vì mọi thứ đều là một Cargo workspace bình thường.

### Fuzz parser tự viết tay
[`labs/01-http-parser`](../../labs/01-http-parser) chính xác là loại code mà fuzzing được sinh ra để
dùng: input ở mức byte, một state machine không tầm thường (header,
chunked encoding, Content-Length), và một lịch sử thật về các bug bảo mật
(request smuggling) đến từ chính lớp parser này khi bất đồng với một
parser khác về input mập mờ. Bọc entry point của parser bạn trong một
`fuzz_target!(|data: &[u8]| { let _ = parse_request(data); })` và để nó
chạy — một parser không bao giờ được panic hay hang trên *bất kỳ* byte sequence nào, kể cả rác.

### Fuzzing dựa trên corpus so với property testing
Fuzzing (cargo-fuzz/AFL) khám phá byte thô được dẫn dắt bởi feedback
coverage và giỏi nhất trong việc tìm crash/panic/hang trên input sai định
dạng mà bạn không nghĩ tới. Property testing (`proptest`) thay vào đó
sinh ra các input *có cấu trúc* (ví dụ "một request line hợp lệ + 0-10
header hợp lệ + body tùy chọn") và kiểm tra một tính chất có đúng không
(ví dụ "parse(serialize(req)) == req") — tốt hơn trong việc bắt bug logic
trên input hợp khuôn dạng so với việc tìm crash của parser. Hãy dùng cả
hai: proptest cho tính đúng đắn round-trip, cargo-fuzz cho "không bao giờ
panic trên bất cứ thứ gì."

### Chạy cargo-fuzz thật sự
Những chi tiết cơ học hay làm người ta vấp lần đầu:

- **Toolchain nightly.** libFuzzer cần các flag sanitizer mà chỉ nightly
  chấp nhận, nên mọi lệnh đều là `cargo +nightly fuzz ...`. Cài tool một
  lần bằng `cargo install cargo-fuzz`.
- **Code được test phải là một library.** Fuzz target là một crate riêng
  *phụ thuộc vào* crate của bạn, và nó không import được từ `main.rs`. Nếu
  parser của bạn nằm trong `src/main.rs`, hãy chuyển nó sang `src/lib.rs`
  và giữ `main.rs` là một binary mỏng gọi tới nó.
- **Bố cục.** `cargo fuzz init` (chạy bên trong crate) tạo ra `fuzz/` với
  `Cargo.toml` riêng và `fuzz/fuzz_targets/<name>.rs`. `Cargo.toml` đó có
  một bảng `[workspace]` rỗng để nó nằm *ngoài* workspace của repo. Cứ để
  nguyên, nếu không `cargo` sẽ phàn nàn rằng fuzz crate nằm trong một
  workspace không liệt kê nó.
- **Chạy có giới hạn thời gian.** Tham số sau `--` được chuyển cho
  libFuzzer: `cargo +nightly fuzz run <target> -- -max_total_time=1800`
  chạy 30 phút. `-max_len=8192` giới hạn kích thước input, và `-timeout=5`
  khiến bất kỳ input nào chạy lâu hơn 5 giây bị tính là treo, và đó cũng
  là một phát hiện (một vòng lặp vô hạn trên một byte sequence nào đó).
- **Corpus và crash.** Các input thú vị được tích lũy trong
  `fuzz/corpus/<target>/`. Đặt các file seed tự viết vào đó trước lần chạy
  đầu tiên. Một crash ghi input vào `fuzz/artifacts/<target>/`, và
  `cargo +nightly fuzz run <target> <artifact-file>` chạy lại đúng input
  đó để bạn debug. `cargo +nightly fuzz tmin <target> <file>` thu nhỏ nó
  về input nhỏ nhất vẫn còn crash.
- **Dictionary.** Một file text chứa các token như `"Content-Length:"`,
  `"Transfer-Encoding:"`, `"chunked"`, `"\x0d\x0a"` (định dạng dictionary escape byte dưới dạng `\xNN`) truyền qua `-dict=<file>`
  cho phép mutator ghép các từ thật của protocol vào, thay vì chờ đoán
  chúng từng byte một.

Gotcha: "không panic" là property yếu nhất mà một target có thể kiểm tra.
Target cũng có thể `assert!` một bất biến trên mọi input, và fuzzer sẽ đi
săn phản ví dụ. Với một parser, một bất biến mạnh là property split-point
của [`labs/01-http-parser`](../../labs/01-http-parser): parse cả byte sequence một lần và parse nó thành
hai mảnh tại bất kỳ offset nào phải cho cùng kết quả. Fuzzer chọn cả byte sequence lẫn offset.

## Practice
1. Thêm một thư mục `fuzz/` vào [`labs/01-http-parser`](../../labs/01-http-parser) bằng `cargo fuzz
   init` và một target gọi parser của bạn trên byte thô.
2. Chạy nó vài phút và sửa bất kỳ panic nào nó tìm thấy (index vượt giới
   hạn trên input bị cắt cụt là crash kinh điển đầu tiên).
3. Seed corpus fuzz bằng các HTTP request thật đã capture (từ output
   `curl -v`) để fuzzer bắt đầu từ một cấu trúc hợp lệ thay vì byte ngẫu
   nhiên.
4. Viết một `proptest` sinh ra một request hợp lệ (method, path, một vài
   header, body Content-Length tùy chọn) và assert parser của bạn trích
   xuất lại đúng method/path/header/body.
5. Đưa vào parser của bạn hai input khác nhau về ngữ nghĩa nhưng giống
   nhau bề ngoài (ví dụ cả một header `Content-Length` lẫn
   `Transfer-Encoding: chunked`) và xác nhận nó chọn một cách xác định và
   từ chối sự mập mờ thay vì đoán — nối lại với
   [`07-security/05-request-smuggling.md`](../07-security/05-request-smuggling.md).
