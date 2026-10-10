# Fragmentation

Vì sao bộ nhớ của một proxy chạy lâu dài cứ tăng lên và không bao giờ giảm
xuống lại, dù nó không hề có leak.

## What to learn

### Triệu chứng trông giống leak nhưng không phải
Một proxy chạy trong một tuần; RSS tăng từ 200 MB lên 2 GB rồi plateau.
Mọi allocation đều có một free tương ứng — valgrind và các leak detector
đều sạch — vậy mà bộ nhớ vẫn "biến mất". Đây là fragmentation, và chẩn
đoán nó thành leak sẽ khiến bạn đi săn một bug không tồn tại.

Khác biệt quan trọng về mặt vận hành: một leak tăng vô hạn và giết chết
process; fragmentation tăng tới một mức đỉnh được thiết lập bởi traffic
pattern tệ nhất của bạn rồi dừng ở đó. Cả hai trông giống hệt nhau trên đồ
thị trong vài giờ đầu.

### External fragmentation
Bộ nhớ trống tồn tại nhưng bị chia thành các mảnh quá nhỏ để đáp ứng một
request. Allocate 10.000 × buffer 1 KB, free một nửa xen kẽ, và bạn có
khoảng 5 MB trống không thể phục vụ dù chỉ một allocation 2 KB liên tục.

Với một proxy, nguyên nhân kích hoạt là vòng đời lẫn lộn: buffer request
sống ngắn đan xen với connection state sống lâu. Connection state sống sót
sẽ "đục lỗ" xuyên qua các vùng mà lẽ ra allocator đã trả lại cho OS.

### Internal fragmentation và size class
Allocator làm tròn allocation lên một size class (8, 16, 32, 48, 64, 80,
96, 112, 128, ... byte trong các allocator kiểu jemalloc). Một allocation
65 byte chiếm một slot 80 byte; 15 byte bị lãng phí và không thể tiếp cận.

Thường là nhỏ nhặt — trừ khi struct trên hot path của bạn nằm ngay sau một
ranh giới. Một connection struct 129 byte chiếm một slot 160 byte, lãng
phí 24% ở quy mô 100 nghìn connection. Đây là lý do vì sao việc sắp xếp
lại field của struct ([`17-performance/04-memory-layout.md`](../17-performance/04-memory-layout.md)) không phải
micro-optimization ở quy mô lớn: thu nhỏ một struct xuống dưới ranh giới
size-class là một chiến thắng dạng bậc thang, không phải tuyến tính.

### Vì sao bộ nhớ không trả lại cho OS
`free()` trả bộ nhớ về cho *allocator*, không phải kernel. Allocator chỉ
trả nó về cho OS khi nó có thể giải phóng toàn bộ một vùng (qua `munmap`
hoặc `madvise(MADV_DONTNEED)`), điều này đòi hỏi vùng đó phải hoàn toàn
trống. Một object sống lâu neo giữ một vùng 4 MB giữ cả 4 MB đó ở trạng
thái resident.

Đây chính xác là vấn đề co lại của slab trong [`13-algorithms/slab.md`](../13-algorithms/slab.md),
được tổng quát hóa: bất kỳ cấu trúc nào tăng lên đỉnh rồi giữ nguyên dung
lượng đó sẽ pin luôn các vùng của allocator theo nó.

Gotcha: malloc của glibc đặc biệt miễn cưỡng trong việc trả lại bộ nhớ, và
các arena per-thread của nó nhân hiệu ứng này lên — mỗi thread có arena
riêng, nên một proxy với 16 worker thread có thể giữ 16 mức đỉnh riêng
biệt. `MALLOC_ARENA_MAX` giới hạn điều này, và chuyển sang jemalloc hay
mimalloc ([`02-linux/16-memory.md`](../02-linux/16-memory.md) nói về việc đổi `#[global_allocator]`)
thường giúp ích nhiều hơn bất kỳ việc tune glibc nào.

### Các cách sửa mang tính cấu trúc
Fragmentation là vấn đề về pattern allocate, nên cách sửa là thay đổi
pattern chứ không phải allocator:

- **Pool các object cùng kích thước.** Một buffer pool luôn trả về cùng
  buffer 8 KB; không có gì bị free, nên không có gì có thể fragment.
- **Arena-allocate theo request.** Bump-allocate mọi thứ một request cần
  vào một vùng và drop toàn bộ vùng đó ở cuối. Không có vòng đời đan xen,
  không có lỗ hổng.
- **Tách vòng đời vào các allocator riêng.** Giữ connection state sống
  lâu tách biệt khỏi dữ liệu request sống ngắn để cái trước không thể pin
  giữ các vùng thuộc về cái sau.
- **Pre-size cho đỉnh.** Nếu một cấu trúc sẽ đạt tới 100 nghìn entry, allocate dung lượng đó một lần tốt hơn là tăng trưởng dần vào nó trong khi
  fragment.

Gotcha: pooling có failure mode riêng của nó — một pool tăng để phục vụ
một đợt traffic rồi không bao giờ co lại *chính là* mức đỉnh, chỉ là giờ
nằm trong tầm kiểm soát của bạn thay vì của allocator. Đó thường là đánh
đổi tốt hơn (có giới hạn và quan sát được), nhưng chỉ khi bạn thực sự giới
hạn nó và export kích thước như một metric ([`08-observability/02-metrics.md`](../08-observability/02-metrics.md)).

### Đo lường nó
Con số cần theo dõi là tỷ lệ giữa RSS và số byte mà ứng dụng của bạn tin
là đang sống. jemalloc phơi bày cả hai trực tiếp (`stats.allocated` so
với `stats.resident`); một tỷ lệ trôi từ ~1.1 lên 2+ là fragmentation,
không phải leak. Theo dõi nó như một gauge thay vì chẩn đoán một lần, vì
toàn bộ vấn đề là nó phát triển qua nhiều ngày.

## Practice
1. Tái tạo external fragmentation: allocate 100 nghìn × buffer 1 KB, free
   một nửa xen kẽ, rồi thử allocate 10 nghìn × 2 KB. Ghi lại RSS ở mỗi
   bước và xác nhận nó không giảm sau các lần free.
2. Chạy cùng bài test dưới glibc malloc, rồi dưới jemalloc và mimalloc qua
   `#[global_allocator]`. So sánh mức đỉnh RSS.
3. Đo việc làm tròn size-class: allocate struct 64, 65, 128, và 129 byte
   100 nghìn lần mỗi loại và so sánh mức tăng RSS thực tế với con số bạn
   kỳ vọng theo tính toán.
4. Instrument [`proxy`](../../proxy) với một gauge allocated-vs-resident và chạy traffic
   của [`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md) vào nó trong thời gian dài; theo
   dõi tỷ lệ theo thời gian thay vì tại một thời điểm duy nhất.
5. Thêm một buffer pool cho buffer I/O của request, chạy lại bước 4, và so
   sánh độ trôi của tỷ lệ có và không có pooling.
