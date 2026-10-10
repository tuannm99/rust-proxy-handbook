# eBPF

Chạy code đã được verify của chính bạn bên trong kernel, không cần module
và không cần reboot. Nền tảng cho packet filtering bằng XDP
([`16-kernel/10-xdp.md`](10-xdp.md)) và cho phần lớn observability hiện đại ở tầng
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
từng IP), `ARRAY` (keyed theo index, nhanh), `PERCPU_HASH`/`PERCPU_ARRAY`
(một instance cho mỗi CPU, không cần atomic — lựa chọn đúng cho counter),
`LPM_TRIE` (longest-prefix match, chính xác là CIDR matching cho
[`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md)), và `RINGBUF` (streaming event
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
  tại. Nhanh nhất, bị giới hạn nhiều nhất. Xem [`16-kernel/10-xdp.md`](10-xdp.md).
- **TC (traffic control)** — sau khi `sk_buff` được allocate; chậm hơn XDP
  nhưng thấy cả ingress lẫn egress và có thể sửa packet tự do hơn.
- **Socket filter / `SO_ATTACH_BPF`** — theo từng socket, hữu ích cho việc
  điều hướng.
- **kprobe / tracepoint / USDT** — observability hơn là filtering: attach
  vào các hàm kernel hoặc tracepoint tĩnh để đo những gì kernel đang làm
  bên dưới proxy của bạn. Đây là thứ mà `bpftrace` compile ra, và nó là
  eBPF thực dụng nhất ngay lập tức cho [`08-observability/04-profiling.md`](../08-observability/04-profiling.md).

### Câu chuyện với Rust
Có hai lựa chọn thực sự. **Aya** là Rust thuần cho cả chương trình phía
kernel lẫn loader phía user-space, không phụ thuộc libbpf/clang — lựa chọn
dễ chịu hơn, và là thứ [`labs/17-ebpf`](../../labs/17-ebpf) nhắm tới. **libbpf-rs** binding thư
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

### Dựng một project Aya
Một project Aya không phải một crate. Nó là ba crate, vì chương trình kernel
được biên dịch cho một target khác (`bpfel-unknown-none`, không có `std`,
không có allocator) so với loader chạy như một process bình thường:

| Crate | Target | Chứa |
|---|---|---|
| `<name>-ebpf` | `bpfel-unknown-none` | chương trình XDP (`#[xdp]` từ `aya-ebpf`) và định nghĩa các map |
| `<name>-common` | cả hai | các kiểu `#[repr(C)]` dùng chung cho hai phía, như key và value của map |
| `<name>` | máy của bạn | loader: load chương trình đã biên dịch, attach nó vào một interface, đọc ghi map, in output của `aya-log` |

Toolchain, cài một lần:
- `rustup toolchain install nightly --component rust-src`. Crate eBPF tự
  build `core` cho target BPF, việc này cần nightly và source của standard
  library.
- `cargo install bpf-linker`, linker biến output LLVM của Rust thành BPF
  bytecode. Trên Linux x86_64 nó cài được ngay.
- `cargo install cargo-generate`, rồi
  `cargo generate https://github.com/aya-rs/aya-template`. Chọn loại chương
  trình `xdp`. Bạn nhận được ba crate đã nối sẵn với nhau, kèm một build
  script biên dịch crate eBPF mỗi khi bạn build loader.

Template là một Cargo workspace riêng, và một workspace không thể nằm bên
trong workspace khác. Với [`labs/17-ebpf`](../../labs/17-ebpf): bỏ `"labs/17-ebpf"` khỏi danh
sách `members` trong `Cargo.toml` ở gốc, thêm `exclude = ["labs/17-ebpf"]`
vào bảng `[workspace]` của nó, và generate project vào thư mục đó thay cho
stub. Build và chạy nó từ bên trong `labs/17-ebpf`.

Load chương trình cần root (`CAP_BPF` + `CAP_NET_ADMIN`), nên lệnh chạy
của template đi qua `sudo`:
`RUST_LOG=info cargo run --config 'target."cfg(all())".runner="sudo -E"' -- --iface veth0`.

**Một mạng test riêng.** Đừng attach một chương trình drop đang thử nghiệm
vào interface thật của bạn. Một network namespace nối bằng một cặp `veth`
cho bạn một "host" thứ hai với IP riêng:

```sh
sudo ip netns add attacker
sudo ip link add veth0 type veth peer name veth1
sudo ip link set veth1 netns attacker
sudo ip addr add 10.10.0.1/24 dev veth0 && sudo ip link set veth0 up
sudo ip netns exec attacker ip addr add 10.10.0.2/24 dev veth1
sudo ip netns exec attacker ip link set veth1 up
sudo ip netns exec attacker ping -c1 10.10.0.1     # phải thành công trước khi có XDP
```

Attach chương trình vào `veth0`. Traffic từ `10.10.0.2` (bất cứ thứ gì chạy
dưới `ip netns exec attacker`, như `curl http://10.10.0.1:8080/`) là thứ nó
nhìn thấy. Chạy proxy của bạn bind vào `10.10.0.1`. `sudo ip netns del
attacker` dọn sạch tất cả.

Kiểm tra attach mode không cần `bpftool`: `ip -details link show veth0` in
ra `xdp` cho native mode và `xdpgeneric` cho chế độ dự phòng
([`16-kernel/10-xdp.md`](10-xdp.md)). `veth` hỗ trợ native XDP. Gotcha cho WSL2: các gói
`linux-tools`/`bpftool` của distro được build cho kernel của distro, không
phải của Microsoft, và từ chối chạy. Nếu bạn muốn `bpftool`, hãy build nó
từ `github.com/libbpf/bpftool`. Bản thân kernel WSL2 có sẵn hỗ trợ BPF, BTF
và veth, nên Aya chạy được ở đó.

### Khi nào nó đáng làm
Filtering bằng eBPF đáng làm khi bạn cần drop traffic *trước khi* nó tốn
bất cứ chi phí nào ([`07-security/09-ddos.md`](../07-security/09-ddos.md)), hoặc quan sát kernel mà
không cần instrument ứng dụng. Nó không phải một thứ thay thế cho logic
ứng dụng: nó không thể parse HTTP một cách có ý nghĩa, không thể ra quyết
định cần state ở user-space, và mọi rule đều bị giới hạn bởi verifier.
Dùng nó như lớp lọc đầu tiên rẻ tiền, với proxy xử lý mọi thứ còn sống sót
qua đó.

## Practice
1. Viết một chương trình Aya tối giản trong [`labs/17-ebpf`](../../labs/17-ebpf) đếm số packet
   nhận được trong một `PERCPU_ARRAY` và một loader user-space in ra tổng
   cộng dồn mỗi giây.
2. Cố tình kích hoạt một lần bị verifier từ chối: đọc một byte packet mà
   không có bounds check `data_end` đứng trước. Đọc lỗi, rồi fix nó — đây
   là vòng lặp bạn sẽ dành phần lớn thời gian eBPF của mình trong đó.
3. Thay counter per-CPU bằng một `HASH` dùng chung và benchmark cả hai
   dưới tải; đo chi phí tranh chấp.
4. Xây một map `LPM_TRIE` chứa các CIDR bị block, điền nó từ user space, và
   tra cứu địa chỉ nguồn với nó từ phía kernel — cùng loại matching mà
   [`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md) làm trong proxy.
5. Stream event tới user space bằng `RINGBUF` và so sánh throughput của
   nó với việc tra cứu map theo từng event.
6. Dùng `bpftrace` (không cần viết code) để vẽ histogram latency của
   `tcp_sendmsg` trong khi proxy của bạn phục vụ tải, và đối chiếu nó với
   metric từ [`08-observability/02-metrics.md`](../08-observability/02-metrics.md).
