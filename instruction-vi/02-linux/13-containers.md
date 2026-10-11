# Container: Namespace và cgroup

File này thuộc nhóm fundamentals. Xem [`02-linux/01-fundamentals.md`](01-fundamentals.md) để có mục lục đầy đủ. Kubernetes, pod
và container được nhắc liên tục từ [`09-architecture/`](../09-architecture) trở đi, và trong nhiều gotcha ở
[`07-security/`](../07-security) và [`08-observability/`](../08-observability). File này là nơi trả lời "container thực sự *là*
gì", vì không chỗ nào khác trong handbook dừng lại để định nghĩa nó.

## What to learn

### Container không phải một máy ảo thu nhỏ
Một máy ảo (virtual machine) ảo hóa *phần cứng*. Nó chạy một kernel đầy đủ của riêng nó, tin rằng nó có CPU, bộ nhớ và thiết bị
riêng, và chạy trên một hypervisor chặn và giả lập phần cứng thật.

Một **container** không làm điều nào trong số đó. Nó là một process bình thường (hoặc một nhóm process) chạy dưới *cùng* kernel
với mọi thứ khác trên host. Kernel làm cho nó *tin* rằng nó đang ở một mình trên máy, bằng hai cơ chế riêng biệt. Không có kernel
thứ hai, không có hypervisor, không có giả lập phần cứng.

Đó là lý do container khởi động trong mili giây còn VM mất vài giây. Cũng là lý do `uname -r` trong container báo phiên bản kernel
của *host*, không phải của riêng nó.

### Namespace: process thấy được gì
Một **namespace** giới hạn những gì một process có thể *thấy*. Linux có vài loại, mỗi loại cách ly một nhóm state toàn cục của hệ
thống:

- **PID namespace**: process trong container thấy chính nó là PID 1 (hoặc gần như vậy), và không thể thấy hay gửi signal cho các
  process bên ngoài namespace, dù tất cả đều là process thật trên cùng một kernel của host.
- **Network namespace**: container có network interface, địa chỉ IP, routing table và port space riêng, cách ly với host. Đây
  là *lý do* hai container cùng bind được port 8080 mà không xung đột: chúng ở hai network namespace khác nhau, nên với kernel
  đó là hai "port 8080" không liên quan nhau. (Các port trong [`01-network/02-addressing.md`](../01-network/02-addressing.md)
  chỉ có nghĩa cục bộ trong một namespace, không thực sự toàn cục, một khi namespace xuất hiện.)
- **Mount namespace**: container thấy filesystem root riêng, được xếp lớp từ một image, khác với filesystem thật của host.
- **UTS namespace**: hostname riêng.

Không cái nào trong số này cần kernel thứ hai hay giả lập phần cứng. Vẫn là cùng một kernel, cùng CPU vật lý, cùng RAM vật lý. Chỉ
có góc nhìn của mỗi process về state toàn cục (danh sách process, network stack, filesystem) bị giới hạn theo namespace.

### cgroup: process được dùng bao nhiêu
Namespace giới hạn *những gì process thấy*. **cgroup** (control group) giới hạn *mức tiêu thụ*: CPU time, bộ nhớ, bandwidth I/O,
cho một nhóm process. Kernel áp giới hạn này bất kể các process nghĩ rằng chúng được làm gì.

Đây là cơ chế trực tiếp đứng sau gotcha về giới hạn bộ nhớ theo cgroup trong [`02-linux/16-memory.md`](16-memory.md), và đứng sau
mọi sự cố "bị OOM-kill trong Kubernetes". Giới hạn bộ nhớ của một container là giới hạn cgroup. Nó được áp dụng dựa trên **RSS**
(bộ nhớ thường trú, thực sự có bộ nhớ vật lý phía sau; đây là hệ quả thực tế của việc phân biệt virtual và physical trong
[`08-memory-basics.md`](08-memory-basics.md)). Nó không dựa trên lượng bộ nhớ ảo mà process chỉ *reserve*.

```
$ cat /sys/fs/cgroup/memory.max     # giới hạn, theo byte (path của cgroup v2)
$ cat /sys/fs/cgroup/memory.current # mức dùng hiện tại so với giới hạn đó
```

Một process allocate (reserve) bộ nhớ ảo nhiều hơn hẳn giới hạn cgroup thì vẫn ổn, cho đến khi nó thực sự *ghi* vào đủ nhiều page
khiến RSS vượt giới hạn. Lúc đó OOM killer của kernel kết thúc process một cách đột ngột, thường không có cảnh báo nào mà code
của bạn bắt được. Đây đúng là kịch bản mà [`02-linux/16-memory.md`](16-memory.md) cảnh báo cho một proxy pre-allocate các buffer
pool lớn.

### Một pod là một tập namespace dùng chung
Theo thuật ngữ Kubernetes:
- Một **container** là một nhóm process bị giới hạn bởi namespace và cgroup, chạy một image.
- Một **pod** là đơn vị triển khai của Kubernetes: một hoặc nhiều container *dùng chung* một network namespace (nên dùng chung một
  địa chỉ IP và port space), nhưng mỗi container vẫn có mount namespace riêng (filesystem riêng) và giới hạn cgroup riêng.

Đây là cơ chế mà **sidecar proxy** ([`01-network/05-proxy-taxonomy.md`](../01-network/05-proxy-taxonomy.md)) dựa vào. Sidecar và
container ứng dụng là các process khác nhau, bị cách ly ở hầu hết các mặt, nhưng dùng chung một network namespace. Nhờ vậy sidecar
chặn (intercept) được traffic của ứng dụng trên `localhost` một cách trong suốt, mà không cần mẹo networking đặc biệt nào.

### Vì sao điều này quan trọng với proxy
Một proxy chạy trong container thừa hưởng mọi giới hạn này, dù code của nó có biết hay không:
- Giới hạn fd của nó ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)) có thể bị container runtime đặt chặt hơn mặc định
  của host.
- Số CPU nó thấy có thể không khớp số core vật lý của host. Giới hạn CPU của cgroup có thể hiện ra như số core lẻ, và `nproc` bên
  trong container có thể nói sai về tài nguyên thực sự khả dụng. Điều này ảnh hưởng trực tiếp khi chọn kích thước pool worker
  thread của tokio.
- Hành vi bộ nhớ của nó do RSS so với giới hạn cgroup quyết định, như mô tả ở trên. Không phải do `ulimit` hay cách process tự tính.

Nội dung về triển khai trong [`09-architecture/`](../09-architecture) (rolling restart, thời điểm graceful shutdown so với
`terminationGracePeriodSeconds`) không thể hiểu đầy đủ nếu thiếu bức tranh namespace/cgroup bên dưới này.

## Practice
1. Nếu bạn có Docker hoặc Podman, chạy `docker run --rm -it alpine sh`. Bên trong, chạy `ps aux` (xem có bao nhiêu process, đó là
   PID namespace) và `hostname` (xem UTS namespace riêng của container). Từ terminal khác trên host, chạy `ps aux` và tìm process
   của container. Ghi nhận PID *thật* trên host khác với PID mà chính container thấy.
2. Chạy hai container, mỗi container bind port 8080 (`docker run -p 18080:8080 ...` và `docker run -p 18081:8080 ...`, dùng bất kỳ
   image HTTP server đơn giản nào) và xác nhận cả hai đều hoạt động. Dùng phần network namespace ở trên để giải thích vì sao
   không xung đột.
3. Nếu hệ thống của bạn dùng cgroup v2 (`ls /sys/fs/cgroup`), tìm thư mục cgroup của một container đang chạy và đọc `memory.max`
   cùng `memory.current`. So sánh `memory.current` với con số `docker stats` báo cho cùng container đó.
4. Đặt giới hạn bộ nhớ của một container thấp có chủ đích (`docker run -m 50m ...`) và chạy một chương trình bên trong nó
   allocate và *ghi vào* hơn 50 MB. Quan sát nó bị giết, rồi kiểm tra `dmesg` trên host để tìm dòng log của OOM killer nêu tên
   process.
5. Chạy `nproc` trên host, rồi chạy lại bên trong một container khởi động với `--cpus=1` trên cùng host đó. Ghi nhận container vẫn
   có thể báo đủ số core của host, dù cgroup của nó chỉ cho phép lượng CPU time bằng một CPU. Giải thích vì sao một tokio runtime
   tính kích thước worker pool từ `nproc` có thể tạo dư thread trong container như vậy.
