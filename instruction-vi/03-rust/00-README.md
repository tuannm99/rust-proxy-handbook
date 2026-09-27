# Rust

Phase 3. Không phải một tutorial Rust — file này giả định bạn đã biết
syntax, và tập trung vào những phần thực sự quyết định code proxy có đúng
hay không, và có chịu được kiểu soi xét mà một kỹ sư systems/network/
backend ở cấp principal áp dụng hay không: ownership xuyên qua các task,
lifetime trong buffer, unsafe ở FFI boundary, shared state, async, pinning,
câu chuyện dispatch/generics của type system, error handling, iterators/
closures, smart pointers, các concurrency pattern ngoài raw lock, macro,
API design, FFI/ABI, và memory representation.

## Cách đọc thư mục này

Thư mục này vẫn giả định bạn đã biết syntax Rust (biến, struct, enum,
`match`, generic cơ bản) — nó không phải một primer "học Rust từ số 0".
Thứ nó dạy từ đầu là các phần quyết định Rust *ở mức proxy-grade* có đúng
hay không. Từ đó, có hai cách dùng:

- **Mới với async/unsafe/systems Rust:** đọc mọi file dưới đây theo thứ
  tự, từ `01-ownership.md` đến `18-async-traits.md`, làm `## Practice`
  của mỗi file trước khi qua file kế tiếp. Coi nó như một tutorial liên
  tục — mỗi nhóm được xây trực tiếp trên nhóm trước (các nhóm dưới đây nói
  rõ điều đó), và `04-runtime/` giả định bạn đã nắm hết.
- **Đã thoải mái với `Arc<Mutex<_>>`, `async`/`.await`, và `unsafe` cơ
  bản:** nhảy thẳng tới nhóm thứ ba, "Kỹ năng kỹ thuật systems/backend"
  (`11-concurrency-patterns.md` đến `18-async-traits.md`) — đó là phần
  một Rust developer đang làm việc ít có khả năng đã biết nhất, vì nó đặc
  thù cho việc xây production system chứ không phải sự thông thạo Rust
  chung. Chỉ ghé qua hai nhóm đầu cho đúng file bạn còn thiếu (ví dụ
  `06-pin.md` nếu `Pin` chưa bao giờ "click", `08-error-handling.md` nếu
  bạn chưa từng thiết kế một error type hướng ra ngoài cho một library).

## Các file

**Ngôn ngữ cốt lõi và memory model** (đọc theo thứ tự ở lần đầu — mọi thứ
khác trong thư mục này và trong `04-runtime/` đều giả định bạn đã nắm các
file này):
- `01-ownership.md` — move semantics, `Copy` vs `Clone`, quy tắc borrowing, drop order và RAII
- `02-lifetimes.md` — elision, struct giữ borrowed data, lifetime với async, HRTB
- `03-unsafe.md` — `unsafe` mở khóa cái gì, hợp đồng bạn phải giữ, giữ nó tối thiểu và sound
- `04-sync.md` — `Arc`, `Mutex` vs `RwLock`, atomics và `Ordering`, shared state vs message passing
- `05-async.md` — trait `Future`, async/await desugaring, cooperative scheduling, drop-is-cancel
- `06-pin.md` — vì sao `Pin` tồn tại, self-referential future, `Unpin`

**Type system và idiom** (vốn từ vựng hàng ngày để viết và đọc Rust ở mức proxy-grade):
- `07-traits-and-generics.md` — static vs dynamic dispatch, monomorphization, associated type, object safety
- `08-error-handling.md` — `Result`/`?`, `thiserror` vs `anyhow`, panic vs error, mutex poisoning
- `09-iterators-and-closures.md` — tính lazy, `Fn`/`FnMut`/`FnOnce`, `impl Trait` vs `Box<dyn Fn>`
- `10-smart-pointers-and-interior-mutability.md` — `Box`/`Rc`/`Arc`/`Cell`/`RefCell`/`Cow`, một dải phổ duy nhất

**Kỹ năng kỹ thuật systems/backend** (thứ phân biệt "biết syntax Rust" với "xây được production system bằng nó"):
- `11-concurrency-patterns.md` — channel (`mpsc`/`oneshot`/`broadcast`/`watch`), actor pattern so với shared state
- `12-macros.md` — `macro_rules!`, hygiene, nơi các proc-macro derive/attribute đã âm thầm chạy code cho bạn
- `13-api-design-and-modules.md` — visibility như một hợp đồng, newtype, builder, sealed trait, semver
- `14-ffi-and-abi.md` — `repr(C)`, `extern "C"`, ownership qua ranh giới FFI, vì sao plugin là `dyn Trait` chứ không phải `dylib`
- `15-memory-layout.md` — `repr(Rust)` vs `repr(C)`, size/align/padding, niche optimization, layout của enum
- `16-testing-idioms.md` — cấu trúc unit/integration/doctest, mock qua trait, table-driven test, `proptest`, `criterion`
- `17-cargo-workspace.md` — `Cargo.toml` của workspace, `[workspace.dependencies]`, feature-flag unification, codegen bằng `build.rs`
- `18-async-traits.md` — vì sao `async fn` trong một trait không tự nhiên `dyn`-compatible, cách sửa boxed-future, `async-trait`, chi phí allocation mỗi lần gọi

## Đi tiếp đến đâu

Nhóm ngôn ngữ cốt lõi là nền cho `04-runtime/`. Ngữ nghĩa cancellation của
`05-async.md` đặc biệt quay lại như một bug thật trong
`06-proxy/01-upstream.md` (một future bị drop làm bỏ qua một bước giảm
counter) và trong `08-observability/03-tracing.md` (một span guard bị giữ
qua `.await`); `04-runtime/04-structured-concurrency.md` cho ý tưởng đó
các API tokio thật (`JoinSet`, tính cancellation-safety của `select!`).
Nhóm type-system và nhóm kỹ năng kỹ thuật systems nạp trực tiếp vào
`05-http-stack/`, `06-proxy/`, và `09-architecture/` — `07-traits-and-generics.md`
là nền cho mọi thiết kế pluggable-strategy (load balancer, plugin system),
`11-concurrency-patterns.md` là nền cho connection pool trong
`06-proxy/01-upstream.md` và rate limiter trong `07-security/07-ratelimit.md`,
`15-memory-layout.md` là điều kiện tiên quyết cho các thiết kế allocator/
arena/slab trong `14-memory/` và công việc cache-layout trong
`17-performance/`. `16-testing-idioms.md` là phần tương ứng ở mức kỹ năng
Rust của công việc hạ tầng load/fuzz/chaos trong `12-testing/`;
`17-cargo-workspace.md` giải thích chính hình dạng `Cargo.toml` của repo
này và là nền cho bước codegen `build.rs` của
`05-http-stack/10-grpc.md`; `18-async-traits.md` là gotcha cụ thể đằng sau
trait middleware của `09-architecture/02-plugin.md` và trait strategy của
`06-proxy/02-load-balancer.md` ngay khi một trong hai cần vừa async vừa
`dyn`-dispatch.
