# Plugin System

## What to learn
### Đánh đổi cốt lõi: khả năng mở rộng compile-time vs runtime
Một "hệ thống plugin" có thể có nghĩa từ "ghép một tập cố định các
`tower::Layer` lúc compile time" tới "load code tùy ý không tin cậy lúc
runtime" (dylib, Lua, WASM). Càng linh hoạt ở runtime càng nhiều rủi ro
(mismatch ABI, crash làm sập cả proxy, lỗ hổng bảo mật) và hiệu năng tệ
hơn (dynamic dispatch, biên serialization). nginx/Envoy/HAProxy đều nghiêng
về module đã compile hoặc sandbox thay vì code động tùy ý, chính vì lý do
này.

### Vì sao hầu hết proxy Rust chọn composition lúc compile-time
Các trait `tower::Service`/`Layer` của Rust cho bạn ghép hành vi xử lý
request (auth, rate limit, WAF, logging — xem
[`09-architecture/01-components.md`](01-components.md)) như các type generic, dispatch tĩnh.
Compiler inline và monomorphize cả stack, nên không có chi phí runtime cho
"plugin", và một module tồi là một lỗi compile hoặc một panic bị chặn lại,
không phải một crash ABI. Chi phí: thêm/bớt một module cần rebuild, không
phải một artifact hot-swap được.

```rust
trait Module: Send + Sync {
    fn name(&self) -> &'static str;
    async fn handle(&self, req: Request, next: Next) -> Response;
}
```

Gotcha: monomorphization có chi phí xuất hiện ở quy mô lớn hơn là trong
một benchmark. Một stack `ServiceBuilder` lồng sâu tạo ra các type cụ thể
khổng lồ — thời gian compile tăng, thông báo lỗi trở nên không đọc được,
và future được tạo ra có thể lớn tới mức việc di chuyển nó cũng đáng kể.
`BoxCloneService` ở một hoặc hai biên layer xóa type và thường tốn ít hơn
những gì nó tiết kiệm được.

### Căng thẳng: thứ tự cấu hình được đòi hỏi dynamic dispatch
Đây là ngã ba thực tế, và nó quyết định thiết kế của bạn.

Một tower stack lúc compile-time có thứ tự được đóng cứng vào type lúc
build. Nhưng một proxy thật thường muốn tập module là *theo từng route*
và *cấu hình được* ([`09-architecture/03-config.md`](03-config.md)): route này cần auth
và WAF, route kia không cần cái nào, và một operator thay đổi nó mà không
rebuild. Bạn không thể diễn đạt "thứ tự đến từ một file TOML" trong một
type đã monomorphize.

Vậy câu trả lời trung thực cho một proxy cấu hình được là một
`Vec<Arc<dyn Module>>` được xây lúc load config — dynamic dispatch, một
lệnh gọi ảo cho mỗi module cho mỗi request, được giải quyết một lần khi
config được parse thay vì mỗi request. Chi phí đó nhỏ và có thể dự đoán;
sai lầm là giả vờ bạn có thể có cả thứ tự điều khiển-bởi-config *và* full
monomorphization.

Gotcha: `async fn` trong một trait mà bạn cần dùng như `dyn Module` là
điểm gợn. `async fn` native trong trait nói chung không object-safe, nên
các trait plugin object-safe hoặc dùng `#[async_trait]` (boxes future trả
về — một allocation cho mỗi module mỗi request) hoặc trả về một
`Pin<Box<dyn Future>>` tường minh, thứ cùng chi phí viết ra bằng tay. Dự
trù cho nó, và giữ số module theo từng route ở mức khiêm tốn.

### Khi nào plugin động vẫn đáng giá
Nếu bạn cần bên thứ ba (hoặc các team không dùng Rust) ship logic mà không
rebuild proxy của bạn, hai lựa chọn thực tế:
- **WASM** (qua `wasmtime`): sandbox, an toàn bộ nhớ ngay cả khi plugin
  lỗi hoặc độc hại, nhưng bạn trả chi phí biên serialization/host-call
  cho mỗi request và bề mặt API plugin có thể gọi bị giới hạn ở những gì
  bạn phơi bày.
- **Dynamic library** (`libloading` + một ABI `extern "C"` ổn định, hoặc
  `abi_stable`): tốc độ gần-native, nhưng một crash hoặc ABI drift trong
  plugin có thể làm sập cả process — `dyn Trait`/generic của Rust không có
  ABI ổn định qua các phiên bản compiler, nên bạn bị giới hạn ở một
  interface hình-dạng-C.
Các WASM filter của Envoy là thiết kế tham chiếu nếu bạn muốn xem điều
này được làm cho một proxy thật.

### Giới hạn một plugin cư xử tồi
An toàn bộ nhớ là nửa dễ của sandboxing. Nửa khó hơn là một plugin chạy
trên đường đi request của bạn đơn giản là có thể *không trả về* — một vòng
lặp vô hạn trong một WASM guest treo worker thread đúng như bất kỳ thao
tác blocking nào khác ([`03-rust/05-async.md`](../03-rust/05-async.md)), kéo theo mọi kết nối
multiplex trên nó cùng với request đã kích hoạt nó.

`wasmtime` cung cấp hai cơ chế cho việc này, và bạn cần một trong số đó:
- **Fuel**: guest bị tính phí theo từng thao tác và trap khi hết. Tất
  định và tái tạo được, với một chi phí nhỏ theo từng instruction.
- **Epoch interruption**: một thread nền tăng một epoch counter và guest
  trap ở lần kiểm tra tiếp theo. Rẻ hơn lúc runtime, dựa trên wall-clock
  thay vì tất định.

Cũng giới hạn bộ nhớ của guest (`StoreLimits`), và quyết định một trap
nghĩa là gì — đó là quyết định fail-open/fail-closed bên dưới, giờ có
thêm hương vị plugin-độc-hại.

Gotcha: vòng đời instance là một quyết định hiệu năng thật sự. Tạo một
`Instance` mới cho mỗi request là cách cô lập sạch nhất và đắt nhất; pool
instance (pooling allocator của wasmtime) nhanh hơn nhiều nhưng nghĩa là
state có thể rò rỉ giữa các request trừ khi bạn reset nó — cùng kỷ luật
như [`14-memory/03-object-pool.md`](../14-memory/03-object-pool.md), với một hệ quả bảo mật nếu bạn làm sai.

### Luồng dữ liệu và điều khiển qua biên plugin
Dù bạn chọn cơ chế nào, hãy quyết định tường minh: một plugin có thể thấy
toàn bộ body request/response, hay chỉ header? Nó có thể short-circuit
(trả một response mà không gọi upstream) không? Nó có thể fail open (cho
qua) hay phải fail closed (reject) khi plugin lỗi? Đây là các quyết định
liên quan tới bảo mật (xem [`07-security/06-waf.md`](../07-security/06-waf.md)), không chỉ là kiến
trúc.

Hai điều nữa gây hại về sau nếu để ngầm định:
- **Một plugin có thể mutate những gì các module trước đã thiết lập
  không?** Nếu một plugin có thể ghi đè header identity mà auth đã đặt
  ([`07-security/01-auth.md`](../07-security/01-auth.md)), nó có thể leo thang đặc quyền. Phơi bày một
  view *chỉ đọc* của context tin cậy và một kênh riêng, hạn chế cho các
  mutation bạn thực sự định cho phép.
- **Truy cập body ép buffer.** Cấp quyền truy cập body âm thầm tắt
  streaming cho mọi route dùng plugin đó
  ([`05-http-stack/10-grpc.md`](../05-http-stack/10-grpc.md), [`05-http-stack/09-websocket.md`](../05-http-stack/09-websocket.md)), với mọi
  hệ quả về giới hạn kích thước của [`07-security/06-waf.md`](../07-security/06-waf.md). Làm nó
  opt-in theo từng plugin và hiển thị trong config, không phải một khả
  năng mọi thứ đều có.

Gotcha: một plugin trên đường đi request là một phần trong ngân sách
latency của bạn, và một plugin bên thứ ba là latency bạn không kiểm soát.
Đo thời gian mỗi plugin riêng biệt như metric của chính nó
([`08-observability/02-metrics.md`](../08-observability/02-metrics.md)) và một span
([`08-observability/03-tracing.md`](../08-observability/03-tracing.md)) — "proxy bị chậm" nên quy được về một
plugin cụ thể mà không cần chia đôi config để tìm.

## Practice
Xây theo thứ tự.

1. Trong [`labs/14-plugin`](../../labs/14-plugin), định nghĩa một trait `Module` và cài đặt hai
   module (logging + rate limiting) đứng sau nó. **Xong khi** mỗi cái
   được unit-test độc lập và không cái nào biết cái kia tồn tại.
2. Xây stack từ config thay vì từ một type. **Xong khi** thay đổi thứ tự
   module trong một file TOML thay đổi thứ tự thực thi mà không cần
   rebuild, và các quy tắc thứ tự từ
   [`09-architecture/01-components.md`](01-components.md) được validate lúc load (từ chối một
   config đặt auth sau WAF).
3. Đo chi phí của dynamic dispatch. **Xong khi** bạn có các con số chi phí
   theo từng request cho một stack monomorphize so với
   `Vec<Arc<dyn Module>>` với `#[async_trait]` — và có thể nói liệu nó có
   quan trọng ở request rate mục tiêu của bạn không.
4. Định nghĩa và ghi lại fail-open vs fail-closed theo từng module. **Xong
   khi** một module WAF panic reject request và một module metrics panic
   thì không — xác minh bằng các test cố ý panic từng cái.
5. Cho plugin một view chỉ-đọc của context tin cậy. **Xong khi** một
   plugin cố ghi đè header identity đã thiết lập bởi auth không thể, và
   một test chứng minh điều đó.
6. Thêm metrics và span đo thời gian theo từng plugin. **Xong khi** một
   plugin cố ý chậm có thể nhận diện được theo tên chỉ từ dashboard.
7. (Stretch) Load một module như một WASM guest với `wasmtime` — bắt đầu
   với việc mutate header. **Xong khi** nó chạy được end-to-end và bạn đã
   đo được latency thêm vào theo từng request so với phiên bản đã compile.
8. Thêm fuel hoặc epoch interruption cộng với giới hạn bộ nhớ vào WASM
   host. **Xong khi** một guest chứa `loop {}` trap và trả về response
   thất bại đã cấu hình của bạn, trong khi các request đồng thời khác
   không thấy ảnh hưởng latency — đây là test chứng minh sandbox là thật.
9. So sánh phần plumbing. **Xong khi** bạn có thể nói ra bao nhiêu dòng
   host function và serialization phiên bản WASM cần so với `Module` đã
   compile, cho cùng một hành vi.
