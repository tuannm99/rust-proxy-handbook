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
chạy — một parser không bao giờ được panic hay hang trên *bất kỳ* chuỗi
byte nào, kể cả rác.

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
