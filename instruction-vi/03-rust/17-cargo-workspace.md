# Cargo Workspace, Feature, và Build Script

## What to learn

### `Cargo.toml` của workspace
Một section `[workspace]` với `members = ["labs/*", "proxy"]` (đúng hình
dạng của repo này) làm mọi crate được liệt kê chia sẻ một `Cargo.lock` và
một thư mục `target/` duy nhất — version của một dependency được resolve
một lần cho cả workspace, không phải mỗi crate riêng. Đây là lý do thêm
`tokio` vào một crate `labs/NN-*` mới không kích hoạt một lượt resolve
dependency mới, độc lập; nó tái sử dụng bất kỳ version nào phần còn lại
của workspace đã chốt.

```toml
# Cargo.toml ở gốc repo
[workspace]
members = ["labs/*", "proxy"]
resolver = "2"
```
Gotcha: một *package* ở edition 2021 ngầm định `resolver = "2"` (edition
2024 ngầm định `"3"`), nhưng một workspace *virtual* như gốc repo này
không có edition riêng và âm thầm rơi về resolver 1 nếu bạn không set rõ
— đó là lý do dòng ở trên tồn tại. Nó quan trọng cho feature unification
bên dưới: resolver v1 hợp nhất feature qua dev-dependency, build-dependency,
và dependency theo target theo những cách bất ngờ hơn.

### `[workspace.dependencies]`: một version, khai báo một lần
Khai báo version của một dependency một lần dưới `[workspace.dependencies]`
và để mỗi crate thành viên tham chiếu nó bằng `dep.workspace = true` giữ
mọi crate ở cùng version mà không phải lặp lại — và chắc chắn để trôi dần
— chuỗi version qua 18 file `Cargo.toml` khác nhau.

```toml
# workspace root
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }

# labs/05-reverse-proxy/Cargo.toml
[dependencies]
tokio = { workspace = true }
```

### Feature flag là additive, và điều đó có ý nghĩa thật
Feature của Cargo được hợp nhất trên toàn bộ *build*, không phải theo
từng crate: nếu crate A bật feature `client` của `hyper` và crate B (được
build trong cùng lần build workspace) bật `server`, `hyper` được compile
sẽ có cả hai. Đây là lý do bỏ một feature khỏi `Cargo.toml` của một crate
có thể không làm binary nhỏ lại nếu một thành viên workspace khác vẫn bật
nó — và vì sao một library crate hầu như không nên bật một feature "để
phòng hờ", vì nó âm thầm trở thành bắt buộc cho mọi consumer được build
cùng nó.

Gotcha: feature phải strictly additive — bật một cái không bao giờ được
loại bỏ chức năng. Cargo enforce điều này bằng construction, không phải
như một guideline về style, chính xác vì unification nếu không sẽ làm kết
quả build phụ thuộc vào crate nào khác tình cờ được compile cùng crate
của bạn.

### `build.rs`: sinh code trước khi `rustc` chạy
Một `build.rs` ở gốc một crate được compile và chạy trước chính crate đó,
và có thể viết file vào `OUT_DIR` mà code của chính crate đó `include!`
vào. Trường hợp cụ thể handbook này cần nó: `tonic-build`/`prost-build`
compile file `.proto` thành struct Rust cho hỗ trợ gRPC
([`05-http-stack/11-grpc.md`](../05-http-stack/11-grpc.md)) — code client/server được sinh ra không tồn
tại như source bạn viết tay, nó được sinh lại mỗi lần build từ schema
`.proto`.

```rust
// build.rs
fn main() {
    tonic_build::compile_protos("proto/health.proto").unwrap();
}
```
Gotcha: một `build.rs` làm việc thật (parse một schema, gọi `protoc`)
thêm thời gian thật vào mỗi clean build và mỗi lần CI chạy — giữ nó nhanh,
và ưu tiên check in code đã sinh cho một crate mà nhiều consumer downstream
build thường xuyên, nếu build time trở nên khó chịu.

### Conditional compilation với `cfg` và feature
`#[cfg(feature = "tls")]` compile hẳn một module vào hoặc ra dựa trên một
feature flag, cho phép một crate hỗ trợ chức năng optional (một backend
TLS optional, một exporter metric Prometheus optional) mà không buộc mọi
consumer phải kéo về và compile dependency đó.

```toml
[features]
default = []
tls = ["dep:tokio-rustls"]
```
```rust
#[cfg(feature = "tls")]
mod tls_listener;
```
Gotcha: bản thân dependency phải khai báo
`tokio-rustls = { version = "...", optional = true }` — `dep:` chỉ hoạt
động với dependency optional. Thiếu `optional = true` thì mọi consumer
phải compile `tokio-rustls` dù feature tắt; còn thiếu tiền tố `dep:` thì
Cargo tự tạo thêm một feature public ngầm tên `tokio-rustls`, làm lộ tên
dependency ra feature API của crate bạn.

## Practice
1. Thêm `[workspace.dependencies]` cho `tokio` và `bytes` ở gốc workspace
   của repo này, và chuyển hai crate `labs/*` sang `dep.workspace = true`
   thay vì lặp lại version.
2. Thêm một feature flag vào một crate `labs/*` để conditionally compile
   một module optional (`#[cfg(feature = "...")]`), và xác nhận qua
   `cargo build --no-default-features` và `cargo build --features ...`
   rằng module thực sự bị loại/bao gồm.
3. Cố tình quan sát feature unification: chỉ bật một feature trong một
   dev-dependency của một crate, build cả workspace, và xác nhận (qua
   `cargo tree -e features` hoặc tương tự) một crate anh em dùng cùng
   dependency đó cũng có feature đó bật lên.
4. Nối `build.rs` với `tonic-build` trong crate xử lý bài tập
   [`05-http-stack/11-grpc.md`](../05-http-stack/11-grpc.md), để compile một file `.proto`, và xem code
   được sinh dưới `target/*/build/*/out/`.
5. Đo thời gian clean build trước và sau khi thêm bước compile schema vào
   `build.rs`, và quyết định check in code được sinh có đáng cho kích
   thước workspace này không.
