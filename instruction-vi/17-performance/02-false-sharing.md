# False Sharing

Hai atomic không liên quan trên cùng một cache line âm thầm serialize hóa
code mà bạn tin là lock-free. Đối tác đồng thời của
`17-performance/01-cpu-cache.md`.

## What to learn

### Cơ chế
Cache coherence hoạt động ở độ chi tiết cache-line (64 byte), không phải
theo từng biến. Khi core A ghi bất kỳ byte nào của một line, bản copy
*toàn bộ* line đó ở mọi core khác bị invalidate và phải được fetch lại.
Nên hai atomic nằm trên cùng một line — dù không thread nào từng chạm vào
atomic *kia* — vẫn ping-pong line đó giữa các core trên mỗi lần ghi. Hai
thread không chia sẻ dữ liệu về mặt logic, nhưng phần cứng đối xử với
chúng như đang chia sẻ, do đó gọi là false sharing.

```rust
struct Stats {
    requests: AtomicU64,  // core 1 đập liên tục vào đây
    errors:   AtomicU64,  // core 2 đập liên tục vào đây — cùng line 64 byte
}                         // mỗi lần ghi vào một cái đều invalidate cache của cái kia
```

Hai thread tăng hai counter độc lập này có thể chạy *chậm hơn* một thread
làm cả hai việc, vì chúng dành thời gian đẩy qua đẩy lại cái line đó. Đây
là cái bẫy kinh điển: bạn shard hóa state để loại bỏ tranh chấp và vô tình
tái tạo lại nó ở tầng phần cứng.

### Chỗ một proxy gặp phải nó
Counter theo từng worker hoặc từng core là nghi phạm hàng đầu — chính là
công việc metrics trong `08-observability/02-metrics.md`. Bạn tách một
counter toàn cục thành một `Vec<AtomicU64>`, một slot mỗi worker, để tránh
tranh chấp... rồi đóng gói 8 cái vào một line, nên các worker liền kề vẫn
đánh nhau vì line. LRU lock đã shard hóa (`13-algorithms/lru.md`) và
per-connection atomic state có cùng rủi ro này.

### Cách sửa: pad tới một cache line
Buộc mỗi atomic nóng nằm trên line riêng của nó. `CachePadded<T>` của
`crossbeam` làm đúng việc này, và là câu trả lời idiomatic:

```rust
use crossbeam_utils::CachePadded;
struct Stats {
    requests: CachePadded<AtomicU64>, // giờ có line riêng
    errors:   CachePadded<AtomicU64>,
}
// hoặc một mảng theo worker:
counters: Vec<CachePadded<AtomicU64>>,
```

Gotcha: padding tốn bộ nhớ — mỗi counter giờ chiếm 64+ byte thay vì 8. Đó
là đánh đổi đúng cho một nhúm counter nóng bị tranh chấp và hoàn toàn sai
cho một triệu counter lạnh. Chỉ pad số ít atomic được ghi đồng thời ở tần
suất cao; đừng pad theo phản xạ. Và lưu ý prefetcher hiện đại đôi khi kéo
theo *cặp* line (hiệu ứng chia sẻ 128 byte), đó là lý do `CachePadded` có
thể pad tới 128 trên một số target.

### Dữ liệu chủ yếu đọc không gặp vấn đề này
False sharing là vấn đề của việc *ghi*. Nhiều core đọc cùng một line vui
vẻ chia sẻ nó — coherence chỉ đánh nhau trên các lần ghi. Nên bảng config
hay routing được đọc trên mỗi request nhưng chỉ ghi khi reload
(`09-architecture/03-config.md`) hoàn toàn ổn khi đóng gói chặt; đừng pad
chúng. Chỉ dành padding cho state được *ghi* đồng thời.

### Nó vô hình nếu không đo lường
Không có gì trong source nói "hai field này chia sẻ một line." Triệu
chứng là throughput không scale theo số core, hoặc *tệ hơn* khi bạn thêm
core. Xác nhận nó trước khi sửa: `perf c2c` (cache-to-cache) được xây
dựng chính xác cho việc này và chỉ ra line đang bị tranh chấp. Như mọi thứ
khác trong thư mục này, điểm kích hoạt là một phép đo, không phải một
linh cảm — xem `08-observability/04-profiling.md`.

## Practice
1. Tái tạo nó: hai thread tăng hai `AtomicU64` đóng gói trong một struct,
   rồi cùng hai cái đó dạng `CachePadded`. Đo throughput và xác nhận phiên
   bản đã pad scale tốt trong khi phiên bản đóng gói thì không (hoặc tệ
   đi).
2. Xây mảng counter theo worker theo cả hai cách (`Vec<AtomicU64>` so với
   `Vec<CachePadded<AtomicU64>>`), chạy nó từ N thread, và vẽ đồ thị
   throughput theo N — phiên bản đóng gói ngừng scale sớm.
3. Dùng `perf c2c` để xác định line đang bị tranh chấp trong phiên bản
   đóng gói và xác nhận nó khớp với các field bạn kỳ vọng.
4. Kiểm toán metrics của `proxy` (`08-observability/02-metrics.md`) để
   tìm các counter được ghi đồng thời chia sẻ một line; chỉ pad những cái
   đó, và xác nhận bằng load test rằng nó có ích và các metric lạnh vẫn
   không bị pad.
5. Chứng minh trường hợp không-phải-vấn-đề: đóng gói chặt dữ liệu routing
   chủ yếu đọc, cho thấy các reader đồng thời không gặp vấn đề gì, và diễn
   giải vì sao pad nó chỉ lãng phí bộ nhớ.
