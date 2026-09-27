# eBPF

Chạy code đã được verify của chính bạn bên trong kernel, không cần module
và không cần reboot. Nền tảng cho packet filtering bằng XDP
(`16-kernel/10-xdp.md`) và cho phần lớn observability hiện đại ở tầng
kernel.

## What to learn

### eBPF thực chất là gì
Một máy ảo nhỏ kiểu RISC bên trong kernel, với 11 register, một stack
512-byte, và một tập instruction dạng bytecode. Bạn compile một bản C
(hoặc Rust) bị giới hạn thành bytecode eBPF, kernel **verify** nó,
JIT-compile nó thành instruction native, rồi attach nó vào một hook. Từ
đó trở đi nó chạy ở tốc độ native mỗi khi hook đó kích hoạt.

Điểm mấu chốt là ranh giới an toàn. Một kernel module bị crash làm sập cả
máy; một chương trình eBPF mà lẽ ra sẽ crash bị từ chối trước khi nó kịp
load. Đó là điều khiến nó triển khai được trên các host production.

### Verifier là toàn bộ câu chuyện
Trước khi load, verifier mô phỏng mọi đường thực thi có thể và từ chối bất
cứ thứ gì nó không chứng minh được là an toàn. Các quy tắc định hình cách
bạn viết eBPF:

- **Thực thi có giới hạn.** Không có vòng lặp không giới hạn. Các kernel
  đời đầu cấm hoàn toàn vòng lặp; các kernel hiện đại cho phép vòng lặp có
  giới hạn mà verifier có thể unroll, cộng với `bpf_loop()` cho một vòng
  lặp có giới hạn do helper điều khiển. Có một giới hạn cứng cho tổng số
  instruction đã verify (1M trên kernel hiện tại).
- **Mọi truy cập bộ nhớ phải chứng minh được là nằm trong giới hạn.** Đọc
  một packet đòi hỏi một kiểm tra tường minh
  `if (data + offset > data_end) return ...` *trước* khi truy cập — và
  kiểm tra đó phải verifier nhìn thấy được trên mọi đường đi, đó là lý do
  code eBPF đầy những bounds check trông như thừa với con người.
- **Không có bộ nhớ kernel tùy ý.** Truy cập đi qua các hàm helper
  (`bpf_probe_read_kernel`, thao tác map), không phải con trỏ thô.
- **Không có stack không giới hạn.** Tổng cộng 512 byte, đó là lý do bất
  cứ thứ gì lớn hơn phải nằm trong một map.

Gotcha: việc bị verifier từ chối là chi phí phát triển chiếm ưu thế, và
thông báo lỗi mô tả trạng thái của *verifier*, không phải ý định của bạn.
Nguyên nhân thường gặp là một bounds check mà verifier không thể liên kết
với truy cập — tái cấu trúc code sao cho check đứng ngay trước truy cập,
trên mọi đường đi, là cách fix chuẩn. Hãy chờ đợi đây sẽ chiếm phần lớn
thời gian debug của bạn.

### Map: cách duy nhất để giữ state
Chương trình eBPF không có state giữa các lần gọi. Mọi state bền vững, và
mọi giao tiếp với user space, đi qua **map** — các key/value store có kiểu
được kernel tạo và quản lý.

Các loại quan trọng ở đây: `HASH` (key/value tổng quát, ví dụ counter theo
từng IP), `ARRAY` (khóa theo index, nhanh), `PERCPU_HASH`/`PERCPU_ARRAY`
(một instance cho mỗi CPU, không cần atomic — lựa chọn đúng cho counter),
`LPM_TRIE` (longest-prefix match, chính xác là CIDR matching cho
`07-security/08-ip-filtering.md`), và `RINGBUF` (streaming sự kiện
kernel-tới-userspace hiệu quả).

```
// hình dạng: phía kernel tăng, user space đọc, không có syscall trong hot path
PERCPU_HASH<u32 /* src ip */, u64 /* packet count */>
```

Gotcha: ưu tiên map per-CPU cho counter. Một counter `HASH` dùng chung cần
một atomic ở mỗi packet và trở thành điểm tranh chấp ở tốc độ hàng triệu
pps; map per-CPU không cần lock trong kernel và được cộng dồn ở user space
tại thời điểm đọc. Đánh đổi là bạn không thể đọc được một tổng tức thời
chính xác từ phía kernel.

### Các attach point liên quan tới một proxy
- **XDP** — sớm nhất có thể, trong NIC driver trước khi một `sk_buff` tồn
  tại. Nhanh nhất, bị giới hạn nhiều nhất. Xem `16-kernel/10-xdp.md`.
- **TC (traffic control)** — sau khi `sk_buff` được cấp phát; chậm hơn XDP
  nhưng thấy cả ingress lẫn egress và có thể sửa packet tự do hơn.
- **Socket filter / `SO_ATTACH_BPF`** — theo từng socket, hữu ích cho việc
  điều hướng.
- **kprobe / tracepoint / USDT** — observability hơn là filtering: attach
  vào các hàm kernel hoặc tracepoint tĩnh để đo những gì kernel đang làm
  bên dưới proxy của bạn. Đây là thứ mà `bpftrace` compile ra, và nó là
  eBPF thực dụng nhất ngay lập tức cho `08-observability/04-profiling.md`.

### Câu chuyện với Rust
Có hai lựa chọn thực sự. **Aya** là Rust thuần cho cả chương trình phía
kernel lẫn loader phía user-space, không phụ thuộc libbpf/clang — lựa chọn
dễ chịu hơn, và là thứ `labs/17-ebpf` nhắm tới. **libbpf-rs** binding thư
viện C libbpf và kế thừa sự trưởng thành cùng hỗ trợ CO-RE của nó.

**CO-RE** (Compile Once, Run Everywhere) là cơ chế portability đáng biết:
layout struct của kernel khác nhau giữa các phiên bản, nên một chương
trình hardcode offset field sẽ hỏng trên một kernel khác. CO-RE phát ra
các relocation được giải quyết tại thời điểm load dựa trên thông tin kiểu
BTF của kernel đang chạy, nên một binary chạy được trên nhiều kernel.

Gotcha: eBPF cần root hoặc `CAP_BPF`/`CAP_NET_ADMIN`. Một proxy drop
privilege sau khi bind port của nó sẽ không thể load chương trình eBPF sau
đó — load lúc khởi động trong khi còn có privilege, hoặc tách việc load ra
một helper có privilege riêng. Ràng buộc về thứ tự này phải được thiết kế
sẵn, không phải chắp vá sau.

### Khi nào nó đáng làm
Filtering bằng eBPF đáng làm khi bạn cần drop traffic *trước khi* nó tốn
bất cứ chi phí nào (`07-security/09-ddos.md`), hoặc quan sát kernel mà
không cần instrument ứng dụng. Nó không phải một thứ thay thế cho logic
ứng dụng: nó không thể parse HTTP một cách có ý nghĩa, không thể ra quyết
định cần state ở user-space, và mọi rule đều bị giới hạn bởi verifier.
Dùng nó như lớp lọc đầu tiên rẻ tiền, với proxy xử lý mọi thứ còn sống sót
qua đó.

## Practice
1. Viết một chương trình Aya tối giản trong `labs/17-ebpf` đếm số packet
   nhận được trong một `PERCPU_ARRAY` và một loader user-space in ra tổng
   cộng dồn mỗi giây.
2. Cố tình kích hoạt một lần bị verifier từ chối: đọc một byte packet mà
   không có bounds check `data_end` đứng trước. Đọc lỗi, rồi fix nó — đây
   là vòng lặp bạn sẽ dành phần lớn thời gian eBPF của mình trong đó.
3. Thay counter per-CPU bằng một `HASH` dùng chung và benchmark cả hai
   dưới tải; đo chi phí tranh chấp.
4. Xây một map `LPM_TRIE` chứa các CIDR bị chặn, điền nó từ user space, và
   tra cứu địa chỉ nguồn với nó từ phía kernel — cùng loại matching mà
   `07-security/08-ip-filtering.md` làm trong proxy.
5. Stream sự kiện tới user space bằng `RINGBUF` và so sánh throughput của
   nó với việc tra cứu map theo từng sự kiện.
6. Dùng `bpftrace` (không cần viết code) để vẽ histogram latency của
   `tcp_sendmsg` trong khi proxy của bạn phục vụ tải, và đối chiếu nó với
   metric từ `08-observability/02-metrics.md`.
