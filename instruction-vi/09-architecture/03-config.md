# Config Reload

## What to learn
### Vì sao "cứ restart process đi" là chưa đủ
Một L7 proxy production đang phục vụ traffic thật; một thay đổi config
(upstream mới, rate limit cập nhật) đòi hỏi restart nghĩa là rớt kết nối
cho mọi request in-flight. Hot reload nghĩa là: load config mới, validate
nó, swap nó vào nguyên tử cho các request mới, trong khi các request hiện
có tiếp tục chạy với config chúng đã bắt đầu (hoặc với config mới, nếu
field đó không ảnh hưởng tới request in-flight).

### SIGHUP như trigger reload
Quy ước Unix (nginx, hầu hết daemon) là: `SIGHUP` = "reload config,"
`SIGTERM` = "shutdown gracefully" (xem `02-linux/10-signals.md`,
`09-architecture/04-graceful-shutdown.md`). Lắng nghe nó bằng
`tokio::signal::unix::signal(SignalKind::hangup())` thay vì blocking
signal handling — điều này giữ reload async và không gây gián đoạn I/O
in-flight.

```rust
use tokio::signal::unix::{signal, SignalKind};

let mut hangup = signal(SignalKind::hangup())?;
tokio::spawn(async move {
    loop {
        hangup.recv().await;
        match load_and_validate_config().await {
            Ok(new_cfg) => config_store.swap(new_cfg),
            Err(e) => tracing::error!(error = %e, "config reload rejected, keeping old config"),
        }
    }
});
```

Gotcha: trigger thay thế — theo dõi file với `inotify`/`notify` — có một
race mà `SIGHUP` không có. Một editor hoặc deploy tool ghi file tại chỗ
kích hoạt một event khi việc ghi *bắt đầu*, nên bạn đọc một config bị cắt
cụt, ghi dở, và reject một thay đổi thực ra hoàn toàn ổn. Yêu cầu writer
làm một atomic rename (ghi vào một file tạm, rồi `rename()` đè lên đích)
sửa được nó, vì rename là nguyên tử và watcher chỉ thấy file hoàn chỉnh —
nhưng giờ bạn phụ thuộc vào mọi tool đụng vào file đó cư xử đúng.
`SIGHUP` đặt quyết định "tôi đã ghi xong" đúng chỗ nó thuộc về: người
viết.

Gotcha: trong một container, có thể không có ai để gửi `SIGHUP`. Cập nhật
Kubernetes ConfigMap xuất hiện như một lần swap symlink trong volume được
mount, nên theo dõi file (với race ghi-dở đã được xử lý — swap *là*
nguyên tử ở đó) thường là trigger duy nhất có sẵn. Hỗ trợ cả hai.

### Luôn validate trước khi swap
Không bao giờ apply một config chưa được parse và validate đầy đủ (địa chỉ
upstream resolve được, cặp cert/key TLS khớp nhau, không có route trùng) —
một lần reload tồi nên log một lỗi và tiếp tục phục vụ traffic trên config
biết-là-tốt gần nhất, không crash hay phục vụ các route hỏng. Đây là tính
chất reliability quan trọng nhất của config reload: **reload thất bại phải
là một no-op, không phải một outage.**

"Đã validate" phải có nghĩa nhiều hơn "đã parse." Các kiểm tra thực sự bắt
được sự cố thật là những cái thử các side effect:
- **Certificate và key thực sự load và khớp nhau**
  (`01-network/13-tls.md`) — một lỗi gõ đường dẫn hay một cặp không khớp
  là một outage toàn phần cho vhost đó.
- **Route không xung đột** (`05-http-stack/03-router.md`) — hai rule
  không bao giờ phân biệt được nghĩa là một endpoint âm thầm biến mất.
- **Upstream pool được tham chiếu tồn tại** — một route trỏ tới một tên
  pool chưa được định nghĩa nên fail validation, không phải 502 lúc
  request.
- **Listener mới thực sự bind được** — nếu config đổi một port, phát hiện
  nó đã bị dùng *sau khi* bạn đã đóng listener cũ là một outage bạn tự
  gây ra.

Cái cuối cùng là lý do một cài đặt thật có hai giai đoạn: *prepare* mọi
thứ có thể fail (parse, load cert, bind socket mới) vào một object dàn
dựng, và chỉ sau đó *commit* bằng cách swap con trỏ. Bất cứ thứ gì fail
trong lúc prepare để lại config đang chạy hoàn toàn không bị đụng tới.

Gotcha: validation chạm vào mạng (resolve DNS upstream, kết nối để kiểm
tra liveness) khiến reload fail khi một *dependency* down, đó là vấn đề
fail-static từ `06-proxy/07-service-discovery.md` mặc bộ đồ khác. Validate
cú pháp và tính nhất quán nội bộ một cách nghiêm ngặt; coi một upstream
không resolve được là vấn đề của health check, không phải lý do để reject
một config nếu không thì hợp lệ.

### Không phải mọi thứ đều hot-reload được
Hãy tường minh, ngay trong tài liệu của chính config, về field nào có
hiệu lực khi reload và field nào cần restart. Cách chia thường gặp:
- **Reload được**: danh sách upstream, route, rate limit, rule WAF, log
  level, timeout cho request mới.
- **Cần một listener mới (hoặc restart)**: bind address/port, phiên bản
  protocol TLS và cấu hình cipher, số worker thread.

Gotcha: âm thầm bỏ qua một field không-reload-được đã thay đổi còn tệ hơn
reject nó. Một operator sửa listen port, gửi `SIGHUP`, thấy "reload
thành công," và phát hiện port cũ vẫn đang phục vụ đã bị đánh lừa một
cách chủ động. Hoặc apply nó (rebind, thứ
`09-architecture/05-rolling-restart.md` xử lý đúng cách), hoặc fail reload
với một thông báo nêu tên field.

### Biểu diễn "config hiện tại" cho các reader đồng thời
Các request đang được xử lý đồng thời trong khi một reload có thể xảy ra;
dùng `arc_swap::ArcSwap<Config>` (hoặc `tokio::sync::watch`) để reader
nhận một snapshot nhất quán mà không lock ở mỗi request, và bản thân việc
swap là một cập nhật con trỏ nguyên tử duy nhất.

```rust
use arc_swap::ArcSwap;
use std::sync::Arc;

static CONFIG: once_cell::sync::Lazy<ArcSwap<Config>> =
    once_cell::sync::Lazy::new(|| ArcSwap::from_pointee(Config::default()));

// reader (hot path): CONFIG.load() -> Arc<Config>, cheap, lock-free
// writer (reload):   CONFIG.store(Arc::new(new_config))
```

Gotcha: `ArcSwap` đảm bảo mỗi `load()` trả về một snapshot nhất quán — nó
**không** đảm bảo hai lệnh gọi `load()` trả về *cùng* một snapshot. Một
request load config để chọn một upstream pool và load lại nó để đọc
timeout của pool đó có thể vắt ngang qua một lần reload và trộn các field
từ hai config khác nhau. Load **một lần** ở đầu request, giữ `Arc` cho
suốt thời gian sống của nó, và truyền nó xuống pipeline (các extension của
`09-architecture/01-components.md`). Bug này hiếm, hoàn toàn không tất
định, và về cơ bản không debug được sau khi xảy ra.

Gotcha: giữ `Arc` đó cho suốt thời gian sống của request cũng là thứ giữ
config cũ sống trong khi các request in-flight dùng nó — bộ nhớ được giải
phóng khi request cuối cùng giữ nó hoàn thành, đó chính xác là drain dẫn
dắt bởi refcount từ `06-proxy/07-service-discovery.md`. Một request
streaming sống lâu ghim một phiên bản config; điều đó đúng, và đáng biết
khi bạn thắc mắc vì sao một config cũ chưa bị drop.

### Bí mật không thuộc về file config
Config được đọc từ đĩa, được log lúc reload, dump ra ở debug output, và
thường bị commit vào một repository do vô ý. Giữ credential (auth
upstream, key ký JWT) trong biến môi trường hoặc một secrets store được
tham chiếu *theo tên* từ config, và bọc chúng trong một newtype tự redact
(`08-observability/01-logging.md`) để `{:?}` trên config không thể làm lộ
chúng.

Gotcha: log config lúc reload thực sự hữu ích để audit những gì đã thay
đổi. Log một **hash hoặc version**, cộng với một diff có cấu trúc của các
field không phải bí mật — không bao giờ log cả struct.

### Config có version cho rollback
Giữ N config hợp lệ gần nhất (hoặc ít nhất là cái gần nhất) để một
operator có thể rollback ngay lập tức nếu một config hợp cú pháp nhưng sai
về logic gây ra tỷ lệ lỗi tăng cao — gắn quyết định rollback với các metric
tỷ lệ lỗi từ `09-architecture/06-canary-deploy.md`/`08-observability/02-metrics.md`.

Gotcha: một lần reload *thành công* rồi mới làm suy giảm traffic là
trường hợp nguy hiểm, vì không gì cảnh báo cả — validation đã pass. Phát
ra một version/hash config như một metric label hoặc một gauge để
dashboard có thể tương quan "tỷ lệ lỗi tăng" với "config đổi lúc này", và
alert trên các lần reload *thất bại* như một ticket
(`08-observability/06-alerting.md`): một proxy chạy vui vẻ trên config cũ
trong khi mọi lần reload đều fail là một sự lệch pha thầm lặng, ngày càng
lớn, so với những gì operator tin là đang được deploy.

## Practice
Xây theo thứ tự.

1. Làm bước 1-5 trong `labs/13-hot-reload`, rồi lặp lại trong `proxy`.
   Định nghĩa một struct `Config` với `serde` + `toml` (upstream, route,
   rate limit, đường dẫn TLS). **Xong khi** nó load lúc khởi động đứng sau
   một `ArcSwap` và các handler đọc qua `.load()`.
2. Load config đúng một lần mỗi request và truyền `Arc` xuống pipeline.
   **Xong khi** một test reload liên tục dưới load đồng thời không thể
   tạo ra một request thấy hai phiên bản config khác nhau — đưa version
   vào span của request để chứng minh điều đó.
3. Thêm một handler `SIGHUP` với prepare/commit hai giai đoạn: parse,
   validate xung đột route, load và khớp cert/key, bind mọi listener mới
   — tất cả trước khi swap. **Xong khi** mỗi failure mode để lại config cũ
   đang phục vụ.
4. Viết các test âm tính. **Xong khi** một route trùng, một cặp cert/key
   không khớp, một route tham chiếu một upstream pool chưa định nghĩa, và
   một port đã bị dùng đều tạo ra một rejection được log và zero thay đổi
   hành vi.
5. Thêm theo dõi file như một trigger thứ hai, xử lý race ghi-dở. **Xong
   khi** một `cp` ghi tại chỗ không kích hoạt một rejection giả, và một
   atomic rename có kích hoạt một reload.
6. Phân loại mọi field là reload-được hay không, và reject thay đổi với
   các field không-reload-được kèm một thông báo nêu tên field. **Xong
   khi** sửa listen port tạo ra một failure rõ ràng thay vì một thành
   công gây hiểu lầm.
7. Chuyển bí mật ra khỏi file và bọc chúng trong một newtype tự redact.
   **Xong khi** `{:?}` trên config đã load không in ra bí mật nào và dòng
   log reload chứa một hash config cộng một diff không-bí-mật.
8. Phát ra version config như một metric và alert trên reload thất bại.
   **Xong khi** một dashboard có thể tương quan một thay đổi tỷ lệ lỗi với
   đúng lần reload gây ra nó, và các lần reload thất bại thầm lặng lặp lại
   tạo ra một ticket.
9. Giữ N config hợp lệ gần nhất với một đường rollback. **Xong khi** một
   lệnh operator revert về phiên bản trước mà không cần restart và không
   rớt request in-flight.
