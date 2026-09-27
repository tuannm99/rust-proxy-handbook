# SIMD

Xử lý nhiều byte trên mỗi instruction — chỗ nó xuất hiện trong một proxy
(scan header, tìm delimiter, checksum) và khi nào nó đáng với độ phức
tạp. Điểm cuối của con đường mà `17-performance/05-branch-prediction.md`
đã bắt đầu.

## What to learn

### Ý tưởng
SIMD (Single Instruction, Multiple Data) chạy một phép toán trên một
vector các lane cùng lúc — so sánh 16 hoặc 32 byte với một delimiter
trong một instruction duy nhất thay vì một vòng lặp 16–32 phép so sánh có
branch. Với việc scan byte mà một proxy làm liên tục (tìm CR/LF kết thúc
một dòng header, dấu `:` tách một header, một ký tự không hợp lệ), đây là
một tốc độ tăng gấp nhiều lần *và* nó loại bỏ luôn branch theo từng byte
mà `17-performance/05-branch-prediction.md` lo ngại.

```rust
// Về mặt khái niệm: load 16 byte, so sánh-bằng với b'\n' trên mọi lane,
// trích ra một bitmask các kết quả khớp, đếm số 0 ở cuối để tìm cái đầu tiên.
// Một lượt qua 16 byte, không branch theo từng byte nào.
```

### Chỗ nó thực sự đáng giá trong một proxy
- **Tìm delimiter** trong HTTP parser (`05-http-stack/01-parser.md`): tìm
  ranh giới dòng và header. Đây là những gì `memchr` làm, và đó là cách
  dùng SIMD có đòn bẩy cao nhất — bạn có được nó miễn phí chỉ bằng cách
  dùng crate `memchr` thay vì một vòng lặp tự viết.
- **Validation:** kiểm tra toàn bộ giá trị một header không chứa ký tự
  điều khiển trong một lượt quét.
- **Checksum/hashing:** CRC, và hashing đứng sau
  `13-algorithms/consistent-hash.md`/`maglev.md`, đều có implementation
  tăng tốc bằng SIMD.

Ngoài các điểm xử lý byte cụ thể này, một proxy bị chi phối bởi I/O và
syscall, không phải tính toán, nên SIMD có diện tích áp dụng nhỏ. Đừng đi
tìm chỗ để vector hóa; dùng nó ở nơi hot path thực sự là một vòng lặp chặt
chẽ trên các byte.

### Có được nó mà không cần viết intrinsic
Ba tầng, theo thứ tự bạn nên ưu tiên:

1. **Dùng một crate đã làm sẵn.** `memchr` (tìm delimiter), `simd-json`,
   `aho-corasick` (`13-algorithms/aho-corasick.md`, dùng SIMD nội bộ cho
   phép scan của WAF trong `07-security/06-waf.md`). Điều này bao phủ gần
   như mọi nhu cầu thật của một proxy mà không cần một dòng unsafe nào.
2. **Autovectorization.** Viết một vòng lặp đơn giản, branchless, thẳng
   trên một slice và để compiler tự vector hóa nó
   (`-C target-cpu=native`, hoặc `target_feature`). Kiểm tra bằng
   `cargo asm` rằng nó thực sự phát ra vector instruction — những thay
   đổi nhỏ có thể âm thầm vô hiệu hóa nó.
3. **Portable SIMD / intrinsic.** `std::simd` (portable) hoặc intrinsic
   của `std::arch` chỉ khi các cách trên chưa đủ. Intrinsic là `unsafe`
   (`03-rust/03-unsafe.md`) và đặc thù theo CPU.

### Phát hiện tính năng lúc runtime là bắt buộc
Một binary compile với AVX2 sẽ crash với SIGILL trên một CPU không có nó.
Nếu bạn tự viết SIMD, bạn phải phát hiện tính năng lúc runtime
(`is_x86_feature_detected!`) và dispatch về một fallback dạng scalar —
hoặc giới hạn `target-cpu` và chấp nhận binary sẽ không chạy trên phần
cứng cũ hơn. Các crate ở tầng 1 xử lý việc này cho bạn, đây là một lý do
nữa để ưu tiên chúng.

Gotcha: SIMD là tối ưu hóa cuối cùng, không phải một tối ưu hóa sớm. Nó
phức tạp, đặc thù theo CPU, và dễ sai một cách tinh vi (xử lý phần đuôi
khi input không phải bội số của độ rộng lane là một nguồn bug kinh điển).
Chỉ dùng nó sau khi `08-observability/04-profiling.md` cho thấy một vòng
lặp scan byte là một hot spot hàng đầu dưới tải thật
(`12-testing/01-load-testing.md`), và ngay cả khi đó hãy thử con đường
`memchr`/`aho-corasick` trước khi viết dù chỉ một intrinsic.

## Practice
1. Thay một vòng lặp tìm byte viết tay trong HTTP parser của bạn
   (`05-http-stack/01-parser.md`) bằng `memchr` và benchmark đường scan
   header; đây là thay đổi SIMD giá trị cao nhất, rủi ro thấp nhất.
2. Viết một vòng lặp validation branchless đơn giản (không có ký tự điều
   khiển trong một giá trị header), compile với `target-cpu=native`, và
   dùng `cargo asm` để xác nhận compiler đã autovectorize nó.
3. Cố tình thêm một branch vào bên trong vòng lặp đó và quan sát
   autovectorization biến mất — học xem điều gì đánh bại compiler.
4. Xác nhận `aho-corasick` (`13-algorithms/aho-corasick.md`) đang dùng
   SIMD dưới workload WAF của bạn (`07-security/06-waf.md`) và so sánh
   throughput của nó với một phép scan đa-substring ngây thơ.
5. Chỉ khi một profile vẫn đòi hỏi: viết một phép tìm delimiter bằng
   `std::simd` với một phần đuôi dạng scalar và một kiểm tra tính năng
   lúc runtime, và xác nhận nó khớp với phiên bản scalar trên input mọi
   độ dài modulo độ rộng lane — rồi suy ngẫm xem con đường dùng crate có
   lẽ ra đã đủ hay không.
