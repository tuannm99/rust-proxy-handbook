# Containers: Namespaces and cgroups

Một phần của chuỗi fundamentals từ-con-số-0 — xem
`02-linux/01-fundamentals.md` để có index đầy đủ. Kubernetes, pod, và
container được nhắc tới liên tục từ `09-architecture/` trở đi và trong
nhiều gotcha ở `07-security/` và `08-observability/` — file này là nơi
trả lời câu hỏi "container thực sự *là* gì," vì không có chỗ nào khác
trong handbook dừng lại để định nghĩa nó.

## What to learn

### Container không phải một VM tí hon
Một máy ảo (virtual machine) ảo hóa *hardware* — nó chạy kernel đầy đủ
của riêng nó, tin rằng nó có CPU, memory, và thiết bị riêng, chạy trên một
hypervisor chặn và giả lập hardware thật. Một **container** không làm bất
kỳ điều gì trong số đó: nó là một process bình thường (hoặc một nhóm
process), chạy dưới *cùng* kernel với mọi thứ khác trên host, mà kernel
làm cho nó *tin* nó đang một mình trên máy bằng hai cơ chế riêng biệt.
Không có kernel thứ hai, không có hypervisor, không có giả lập hardware —
đó chính xác là lý do container khởi động trong mili giây trong khi VM
mất vài giây, và vì sao `uname -r` của một container báo phiên bản kernel
của *host*, không phải của riêng nó.

### Namespace: process thấy được gì
Một **namespace** giới hạn những gì một process có thể *thấy*. Linux có
vài loại, mỗi loại cách ly một hạng mục trạng thái hệ thống toàn cục:
- **PID namespace** — process của một container thấy chính nó là PID 1
  (hoặc gần vậy) và không thể thấy hay signal các process ngoài
  namespace của nó, dù tất cả đều là process thật trên cùng một kernel
  host.
- **Network namespace** — một container có network interface, địa chỉ
  IP, routing table, và port space riêng, cách ly khỏi host — đây là *lý
  do* hai container đều có thể bind port 8080 mà không xung đột: chúng ở
  các network namespace khác nhau, nên từ góc nhìn của kernel đó là hai
  "port 8080" không liên quan (các port trong
  `01-network/02-addressing.md` là cục bộ theo namespace, không thực sự
  toàn cục, một khi namespace tham gia vào cuộc chơi).
- **Mount namespace** — một container thấy filesystem root của riêng nó,
  xếp lớp từ một image, khác với filesystem thật của host.
- **UTS namespace** — hostname riêng của nó.

Không cái nào trong số này liên quan tới một kernel thứ hai hay giả lập
hardware — vẫn là cùng một kernel, cùng CPU vật lý, cùng RAM vật lý, chỉ
là góc nhìn mỗi process nhận được về trạng thái toàn cục (danh sách
process, network stack, filesystem) bị giới hạn theo từng namespace.

### cgroup: process được phép dùng bao nhiêu
Namespace giới hạn *tầm nhìn*; **cgroup** (control group) giới hạn *mức
tiêu thụ* — CPU time, memory, băng thông I/O — cho một nhóm process, được
kernel enforce bất kể các process đó nghĩ chúng được phép làm gì. Đây là
cơ chế trực tiếp đứng sau gotcha về giới hạn memory theo cgroup trong
`02-linux/09-memory.md` và mọi sự cố "bị OOM-killed trong Kubernetes":
giới hạn memory của một container là một giới hạn cgroup, được enforce
dựa trên **RSS** (memory thường trú, thực sự được backing vật lý — hệ quả
thực tế của phân biệt virtual-vs-physical trong `04-memory-basics.md`),
không phải dựa trên bao nhiêu virtual memory process của bạn chỉ đơn
thuần *reserve*.

```
$ cat /sys/fs/cgroup/memory.max     # giới hạn, theo byte (đường dẫn cgroup v2)
$ cat /sys/fs/cgroup/memory.current # mức dùng hiện tại so với giới hạn đó
```

Một process đã cấp phát (reserve) virtual memory nhiều hơn hẳn giới hạn
cgroup của nó thì hoàn toàn ổn — cho tới khi nó thực sự *ghi* vào đủ số
trang để RSS vượt giới hạn, lúc đó OOM killer của kernel kết thúc process
một cách đột ngột, thường không có cảnh báo nào mà code của bạn có thể
bắt được. Đây chính xác là kịch bản `02-linux/09-memory.md` cảnh báo cho
một proxy pre-allocate các buffer pool lớn.

### Một pod là một tập namespace được chia sẻ
Theo thuật ngữ Kubernetes: một **container** là một nhóm process bị giới
hạn namespace, giới hạn cgroup, chạy một image. Một **pod** là đơn vị
triển khai của Kubernetes — một hoặc nhiều container *chia sẻ* một
network namespace (và do đó chia sẻ một địa chỉ IP và port space) trong
khi giữ mount namespace riêng (filesystem riêng) và giới hạn cgroup
riêng. Đây chính xác là cơ chế mà một **sidecar proxy**
(`01-network/05-proxy-taxonomy.md`) dựa vào: sidecar và container ứng
dụng là các process khác nhau, cách ly nhau ở hầu hết các mặt, nhưng chia
sẻ một network namespace, nên sidecar có thể chặn traffic của ứng dụng
trên `localhost` một cách trong suốt mà không cần thủ thuật networking
đặc biệt nào.

### Vì sao điều này quan trọng với một proxy
Một proxy chạy bên trong một container thừa hưởng mọi giới hạn này dù
code của chính nó có nhận thức được hay không: giới hạn fd của nó
(`03-kernel-and-syscalls.md`) có thể bị container runtime giới hạn chặt
hơn mặc định của host, số CPU nó thấy được có thể không khớp số core vật
lý của host (giới hạn CPU cgroup có thể hiện ra như core phân số —
`nproc` bên trong một container có thể nói dối về thứ thực sự khả dụng,
điều này quan trọng trực tiếp khi định cỡ pool worker thread của tokio),
và hành vi memory của nó bị chi phối bởi RSS-so-với-giới-hạn-cgroup như
mô tả ở trên, không phải bởi những gì `ulimit` hay sổ sách kế toán riêng
của process tin tưởng. Không nội dung nào trong `09-architecture/` về
triển khai (rolling restart, thời điểm graceful shutdown so với
`terminationGracePeriodSeconds`) có ý nghĩa đầy đủ nếu thiếu bức tranh
namespace/cgroup này bên dưới.

## Practice
1. Nếu bạn có Docker hoặc Podman, chạy `docker run --rm -it alpine sh`,
   rồi bên trong nó chạy `ps aux` (xem có bao nhiêu process tồn tại —
   PID namespace) và `hostname` (xem UTS namespace riêng của container).
   Từ một terminal khác trên host, chạy `ps aux` và cố tìm process của
   container — ghi nhận PID *thật* trên host khác với PID mà chính
   container thấy là của nó.
2. Khởi động hai container mỗi cái bind port 8080 (`docker run -p
   18080:8080 ...` và `docker run -p 18081:8080 ...` dùng bất kỳ image
   HTTP server đơn giản nào) và xác nhận cả hai đều hoạt động — giải
   thích, dùng phần network-namespace ở trên, vì sao điều này không xung
   đột.
3. Nếu cgroup v2 khả dụng trên hệ thống của bạn (`ls /sys/fs/cgroup`),
   tìm thư mục cgroup của một container đang chạy và đọc `memory.max` và
   `memory.current` — so sánh `memory.current` với con số `docker stats`
   báo cáo cho cùng container đó.
4. Đặt giới hạn memory của một container cố tình thấp (`docker run -m
   50m ...`) và chạy một chương trình bên trong nó cấp phát và *ghi vào*
   nhiều hơn 50MB — quan sát nó bị giết, rồi kiểm tra `dmesg` trên host
   để tìm dòng log của OOM killer nêu tên process.
5. Chạy `nproc` trên host, rồi chạy lại nó bên trong một container khởi
   động với `--cpus=1` trên cùng host đó — ghi nhận container vẫn có thể
   báo cáo đầy đủ số core của host dù cgroup của nó chỉ giới hạn nó một
   lượng thời gian CPU bằng một CPU; giải thích vì sao một tokio runtime
   định cỡ worker pool của nó từ `nproc` có thể cấp phát dư thread bên
   trong một container như vậy.
