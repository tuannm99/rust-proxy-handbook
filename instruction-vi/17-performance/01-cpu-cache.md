# CPU Cache

Vì sao data layout, chứ không phải số lượng instruction, thường quyết
định latency của hot path. Đọc file này **sau khi**
[`08-observability/04-profiling.md`](../08-observability/04-profiling.md) đã chỉ một flamegraph vào một hot path
cụ thể — không phải trước.

## What to learn

### Những con số làm nên vấn đề
Một lần load từ L1 mất ~1 ns; từ main memory mất ~100 ns. CPU không fetch
từng byte, nó fetch các cache line 64 byte. Nên câu hỏi quyết định tốc độ
hot path hiếm khi là "bao nhiêu instruction" mà là "bao nhiêu cache miss"
— một lần miss tốn ngang bằng ~100 phép toán số học. Code trông hiệu quả
(ít instruction) có thể chậm vì nó đuổi theo các con trỏ mà mỗi cái đều
miss cache, và code trông lãng phí (chạm nhiều byte hơn) có thể nhanh vì
nó stream tuần tự và mọi line đều được prefetch.

### Locality: hai loại
- **Spatial** — dữ liệu dùng cùng nhau nên nằm cùng nhau, để một lần fetch
  line mang về nhiều giá trị hữu ích. Duyệt một `Vec<Struct>` có tính chất
  này; đuổi theo một linked list các node `Box` thì không, đây là lý do
  cốt lõi vì sao [`13-algorithms/lru.md`](../13-algorithms/lru.md) và [`15-parser/03-ast.md`](../15-parser/03-ast.md) đẩy mạnh
  arena (`Vec` + index) thay vì cấu trúc con trỏ `Box`.
- **Temporal** — dữ liệu dùng bây giờ sẽ sớm được dùng lại, nên giữ nó
  nóng.

```rust
// Struct-of-Arrays: duyệt riêng `weight` chỉ chạm vào line của weight
struct Backends { weights: Vec<u32>, conns: Vec<u32>, addrs: Vec<SocketAddr> }
// so với Array-of-Structs: duyệt `weight` kéo theo cả addr+conns qua cache
struct Backend { weight: u32, conns: u32, addr: SocketAddr }
```

Với một load balancer quét weight qua hàng nghìn backend
([`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)), layout struct-of-arrays có thể nhanh hơn
nhiều lần chỉ đơn giản vì không fetch các field nó không đọc.

### Prefetching thưởng cho truy cập có thể đoán trước
Bộ prefetch phần cứng theo dõi pattern truy cập của bạn và kéo các line
tiếp theo *trước khi* bạn hỏi, nhưng chỉ khi pattern đó có thể đoán trước
— tuần tự hoặc stride cố định. Duyệt array tuần tự chạy gần tới băng
thông bộ nhớ; đuổi theo con trỏ ngẫu nhiên đánh bại hoàn toàn prefetcher
và trả giá miss đầy đủ ở mỗi bước nhảy. Đây là lý do cụ thể, đo được, vì
sao "mảng phẳng thắng cấu trúc liên kết" cứ liên tục xuất hiện.

### Đuổi theo con trỏ là khoản thuế lặp lại của proxy
Một request thường đi qua: connection → entry trong session map → route →
upstream → backend. Nếu mỗi bước là một heap object riêng, mỗi bước có
khả năng là một cache miss, và chuỗi này chạy trên mọi request. Bạn không
thể loại bỏ sự gián tiếp về logic, nhưng bạn có thể làm cho các field
*nóng* (những field được chạm trên mọi request) nằm liền kề nhau và để
field lạnh ở nơi khác — một "hot/cold split" của struct.

### Đo lường, vì trực giác ở đây thường sai
Hành vi cache vô hình trong source code. Dùng `perf stat` để đọc
`cache-misses` và `L1-dcache-load-misses`, và `perf record` /
`cachegrind` để gán chúng cho các dòng code cụ thể. Toàn bộ kỷ luật này
phản trực giác đến mức thay đổi layout mà không đo cả trước lẫn sau chỉ là
đoán mò — và cái đoán đó thường sai ngược lại.

Gotcha: đừng bao giờ làm việc này một cách suy đoán. Một thay đổi layout
cắt bớt cache miss trên code chạy 0.1% thời gian là vô hình trong
production và mãi mãi thêm phức tạp. Điểm kích hoạt là một flamegraph
([`08-observability/04-profiling.md`](../08-observability/04-profiling.md)) chỉ ra một vòng lặp hot cụ thể, dưới
tải thật ([`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)) — xem thêm
[`17-performance/02-false-sharing.md`](02-false-sharing.md) cho phiên bản đồng thời của vấn đề
này.

## Practice
1. Benchmark array-of-structs so với struct-of-arrays cho việc quét một
   field qua 100 nghìn backend (phép quét weight của
   [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)); ghi lại `perf stat cache-misses` cho
   cả hai, không chỉ wall time.
2. Xây một linked list các node `Box` và một arena (`Vec` + index) của
   cùng dữ liệu, duyệt cả hai, và so sánh số cache-miss để thấy hiệu ứng
   prefetcher.
3. Lấy một struct nóng từ đường xử lý request của [`proxy`](../../proxy) và tách nó
   thành hot/cold; đo xem benchmark của đường xử lý request có thay đổi
   chút nào không — và thành thật nếu nó không thay đổi.
4. Dùng `perf record` để gán cache miss cho các dòng cụ thể trong một hot
   path của [`proxy`](../../proxy) dưới tải, thay vì đoán truy cập nào tốn kém.
5. Đọc `L1-dcache-load-misses` trước và sau một thay đổi layout và ghi lại
   xem thay đổi đó có đáng với độ phức tạp của nó không — luyện tập việc
   từ chối những thay đổi không làm dịch chuyển con số.
