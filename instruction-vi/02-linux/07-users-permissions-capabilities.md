# User, Permission và Capability

File này trả lời: kernel xem một process là ai, điều đó cho phép nó làm gì, và làm sao một proxy
vừa bind được port 443, vừa đọc được private key mà không phải chạy bằng root toàn năng. Nó bổ
sung cho phần phân biệt "kernel mode và root" trong
[`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md).

## What to learn

### Danh tính: UID và GID
Với kernel, một user chỉ là một con số. Mỗi process mang một **UID** và một **GID** chính, cùng
các group phụ. Tên trong `/etc/passwd` và `/etc/group` chỉ là nhãn để `ls` và `id` hiển thị.
UID `0` là **root**, được đối xử đặc biệt.

Mỗi process thực ra có nhiều UID:
- **real**: ai khởi động nó.
- **effective**: UID mà các kiểm tra permission dùng. Đây là cái quan trọng.
- **saved**: để nó có thể chuyển ngược lại.

Một executable **setuid** chạy với effective UID của *chủ sở hữu file*. Trước đây `sudo` và
`ping` hoạt động theo cách này. Vì thế các binary setuid root rất nhạy cảm về bảo mật.

File mang một owner UID và một group GID
([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)). Các ID *effective* của process được so
với chúng.

Trong container, các con số vẫn mang cùng ý nghĩa với kernel. UID 0 trong container chính là UID 0
của host, trừ khi một **user namespace** ánh xạ lại nó ([`13-containers.md`](13-containers.md)).

### Permission bit: rwx cho owner, group, other
Mỗi inode có chín permission bit: read, write, execute cho **owner**, **group** và **other**.
Ngoài ra còn ba bit đặc biệt. Kernel chỉ kiểm tra lớp đầu tiên khớp với caller: owner nếu bạn là
chủ, nếu không thì group, nếu không nữa thì other. Nó không cộng gộp các lớp.

```text
-rw-r----- 1 proxy proxy 1675 key.pem     = 0640: owner đọc/ghi, group đọc, người khác không có quyền gì
drwxr-x--- 2 proxy proxy 4096 certs/      = 0750: directory
```
Với **directory**, các bit có ý nghĩa khác:
- `r`: liệt kê tên các entry.
- `w`: tạo, xóa, đổi tên entry.
- `x`: cho phép đi *xuyên qua* directory. Cần quyền này trên mọi directory trong một path.

Vì vậy xóa một file cần quyền write trên **directory chứa nó**, không cần quyền trên chính file.

**`umask`** gỡ bớt bit khỏi file mới tạo (`022` biến `666` thành `644`). Một proxy ghi private key
hoặc socket file nên đặt mode tường minh (`OpenOptions::mode(0o600)` từ
`std::os::unix::fs::OpenOptionsExt`), thay vì thừa kế umask.

**root bỏ qua hoàn toàn các kiểm tra này.** Đó chính là vấn đề.

### Vì sao không chạy luôn bằng root?
Nếu process chạy bằng root có một bug (path traversal trong static serving, lỗi memory-safety của
parser trong code `unsafe`, một shell injection), kẻ tấn công chiếm được cả cỗ máy. Proxy xử lý
dữ liệu thù địch từ internet cả ngày. Vì vậy mục tiêu thiết kế là **least privilege**: chạy bằng
một user thường, chuyên dụng, chỉ có quyền truy cập đúng những gì nó cần.

Hai thứ hay đẩy người ta về phía root là bind port thấp và đọc private key. Cả hai đều có cách
giải quyết tốt hơn.

### Capability: root được chia nhỏ
Linux chia quyền lực của root thành khoảng 40 **capability**. Một process giữ các tập capability
(permitted, effective, inheritable, bounding, ambient). Một thao tác đặc quyền cần đúng capability
tương ứng, không cần UID 0. Những cái bạn sẽ gặp:

- `CAP_NET_BIND_SERVICE`: bind port dưới 1024. Đây là cách sửa kinh điển cho "proxy của tôi phải
  listen trên 443". Hãy cấp riêng capability này thay vì cấp root. Cách làm:
  `setcap cap_net_bind_service=+ep ./proxy`, hoặc `AmbientCapabilities=` trong systemd unit, hoặc
  `--cap-add` trong Docker. Hoặc hạ ngưỡng port bằng
  `sysctl net.ipv4.ip_unprivileged_port_start=443`, hoặc để supervisor mở socket sẵn.
- `CAP_NET_RAW`: raw socket và bắt packet (`ping`, `tcpdump`).
- `CAP_NET_ADMIN`: cấu hình interface, route, firewall, `tc`.
- `CAP_SYS_ADMIN`: "root phiên bản mới", gồm hàng chục thao tác gộp lại. Cấp nó cho container gần
  như tương đương với việc trao root.
- `CAP_SYS_RESOURCE`, `CAP_SYS_NICE`, `CAP_IPC_LOCK`: vượt giới hạn tài nguyên, nâng scheduling
  priority, lock bộ nhớ ([`20-limits-and-proc.md`](20-limits-and-proc.md),
  [`12-cpu-scheduling.md`](12-cpu-scheduling.md)).

`getpcaps <pid>` hoặc `grep Cap /proc/<pid>/status` hiện các tập capability của một process
(`capsh --decode=<hex>` giải mã chúng). Container bắt đầu với một tập mặc định đã được cắt bớt.
Tư thế an toàn là **bỏ hết, rồi chỉ thêm lại những gì cần**.

### Pattern drop-privileges
Khi thật sự cần root trong thời gian ngắn (cách khởi động truyền thống), hãy làm các việc cần đặc
quyền *trước*, rồi hạ đặc quyền vĩnh viễn:

1. Bind port thấp và mở các file certificate/private key.
2. Chuyển sang một user không đặc quyền, đúng thứ tự: `setgroups` (xóa group phụ), rồi `setgid`,
   rồi `setuid`. (Sau `setuid` bạn không còn `setgid` được nữa.)
3. Xác minh rằng bạn không lấy lại được root, rồi tiếp tục chạy proxy.

Một fd đã mở vẫn dùng được sau khi hạ quyền. Đó là toàn bộ mẹo: listening socket và key được lấy
khi còn đặc quyền, rồi dùng khi đã mất đặc quyền.

Trong Rust, dùng crate `nix` (`nix::unistd::setuid`) hoặc `libc`. **Kiểm tra giá trị trả về của
mọi lời gọi.** Nếu `setuid` fail mà bạn bỏ qua, bạn vẫn âm thầm chạy bằng root.

Ngày nay còn tốt hơn là để *supervisor* khởi động bạn ở dạng không đặc quyền, kèm socket đã mở
sẵn ([`21-systemd-and-services.md`](21-systemd-and-services.md): socket activation), hoặc với
`User=` và ambient capability phù hợp. Khi đó bạn khỏi phải làm cả quy trình này.

### Gia cố nhiều lớp, ngoài UID
- **`no_new_privs`** (`prctl(PR_SET_NO_NEW_PRIVS)`): process không bao giờ có thêm đặc quyền qua
  exec hay binary setuid. Hãy đặt nó, vì seccomp yêu cầu.
- **seccomp**: danh sách các syscall mà process được phép gọi. Nếu một exploit thử gọi `execve`,
  process bị giết. Đây là lớp gia cố sâu nhất, và là nơi các eBPF filter cũng hoạt động
  ([`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md)).
- **Namespace và cgroup** ([`13-containers.md`](13-containers.md)) giới hạn những gì process thấy
  và dùng được.
- **Mandatory access control** (SELinux, AppArmor) thêm policy áp dụng cả cho file của root. Một
  bất ngờ hay gặp trong production: mọi thứ trông đều được phép theo mode bit, nhưng vẫn fail với
  `EACCES` vì một nhãn SELinux cấm (xem `ausearch -m avc`, `dmesg`).

### Gotcha: secret trên đĩa và trong `/proc`
Private key và token nên có mode `0600` (hoặc `0400`), thuộc về user của service, và nằm trong một
directory mà người khác không đi xuyên qua được.

Secret truyền qua **command-line argument** hiện ra với mọi user trong `ps` và
`/proc/<pid>/cmdline`. **Environment** thì đọc được ở `/proc/<pid>/environ` bởi cùng user và root.
Hãy ưu tiên file có permission chặt, hoặc một secret mount.

Và đừng quên: core dump ([`20-limits-and-proc.md`](20-limits-and-proc.md)) của proxy chứa cả key
của nó.

## Practice

1. Chạy `id` và `ls -l /etc/shadow /etc/passwd /usr/bin/sudo`. Xác định owner, mode bit, và (với
   `sudo`) bit setuid (`s`). Giải thích bit nào cho phép `sudo` của một user thường chạy như root.
2. Tạo `d/f` với các mode do bạn chọn. Dự đoán và kiểm chứng xem bạn có `ls d`, `cat d/f` và
   `rm d/f` được không khi directory có mode `000`/`500`/`700`. Xác nhận rằng việc xóa phụ thuộc
   vào permission của directory.
3. Với một user không đặc quyền, thử `python3 -m http.server 80` (chờ `PermissionError`). Sau đó
   cấp capability: `sudo setcap cap_net_bind_service=+ep $(readlink -f $(which python3))` (gỡ bằng
   `setcap -r`), hoặc dùng [`labs/00-tcp-server`](../../labs/00-tcp-server) trên port 80 với
   capability. Kiểm tra `getpcaps` hoặc `/proc/<pid>/status` (`CapEff`).
4. Thử `sysctl net.ipv4.ip_unprivileged_port_start` trước và sau khi hạ giá trị (nhớ khôi phục
   lại).
5. Chạy server của bạn trong một container non-root với
   `--cap-drop ALL --cap-add NET_BIND_SERVICE --user 1000` và bind port 80. Sau đó thử một thao tác
   cần capability đã bỏ (`ip link add`) và đọc lỗi.
6. Viết một chương trình Rust scratch bind port 80 bằng root, rồi hạ xuống user `nobody`
   (`setgroups`, `setgid`, `setuid`, qua `nix` hoặc `libc`). Chứng minh bằng `id` hoặc
   `/proc/self/status` rằng nó không lấy lại được root (`setuid(0)` trả lỗi), và nó vẫn nhận được
   connection trên listener đã mở sẵn.
