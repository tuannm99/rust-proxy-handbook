# User, Permission, và Capability

Một process *là ai* trong mắt kernel, điều đó cho nó làm được gì, và làm sao một proxy phải
bind port 443 và đọc private key mà vẫn tránh chạy bằng root toàn năng. Bổ sung cho phân biệt
"kernel mode vs root" trong [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md).

## What to learn

### Danh tính: UID và GID
Với kernel, một user là một con số. Mỗi process mang một **UID** và một **GID** chính cùng các
group phụ; tên trong `/etc/passwd` và `/etc/group` chỉ là nhãn cho `ls` và `id`. UID `0` là
**root**, được đối xử đặc biệt. Mỗi process thực ra có nhiều UID: **real** (ai khởi động nó),
**effective** (thứ mà các kiểm tra permission dùng — cái quan trọng), và **saved** (để nó có
thể chuyển ngược lại). Một executable **setuid** chạy với effective UID của *chủ sở hữu nó*
(cách `sudo` và `ping` hoạt động trước đây), vì thế các binary setuid root nhạy cảm về bảo mật.
File mang một owner UID và group GID ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)); các ID *effective* của process bạn được
so với chúng. Bên trong container các con số vẫn mang cùng ý nghĩa với kernel — UID 0 trong
container là UID 0 của host trừ khi một **user namespace** ánh xạ lại nó
([`13-containers.md`](13-containers.md)).

### Permission bit: rwx cho owner, group, other
Mỗi inode có chín permission bit — read, write, execute cho **owner**, **group** và **other** —
cộng ba bit đặc biệt. Kernel kiểm tra lớp đầu tiên khớp với caller (owner nếu bạn sở hữu,
không thì group, không thì other); nó không cộng gộp.

```text
-rw-r----- 1 proxy proxy 1675 key.pem     = 0640: owner đọc/ghi, group đọc, người khác không gì
drwxr-x--- 2 proxy proxy 4096 certs/      = 0750: directory
```
Ý nghĩa khác nhau với **directory**: `r` liệt kê tên, `w` tạo/xóa/đổi tên entry, `x` cho bạn đi
*xuyên qua* directory (cần trên mọi directory trong một path). Xóa một file cần quyền write
trên **directory** của nó, không phải trên file. **`umask`** gỡ bớt bit khỏi file mới tạo
(`022` biến `666` thành `644`); một proxy ghi private key hay socket file nên set mode tường
minh (`OpenOptions::mode(0o600)` từ `std::os::unix::fs::OpenOptionsExt`) thay vì thừa kế umask.
**root bỏ qua hoàn toàn các kiểm tra này** — đúng là vấn đề.

### Vì sao không chạy luôn bằng root?
Một bug trong process root (path-traversal trong static serving, lỗi memory-safety của parser
trong code `unsafe`, một shell injection) trao cho kẻ tấn công cả cỗ máy. Proxy xử lý byte thù
địch từ internet cả ngày, nên mục tiêu thiết kế là **least privilege**: chạy bằng một user
thường, chuyên dụng, chỉ với quyền truy cập nó cần. Hai thứ thường đẩy người ta về phía root là
bind port thấp và đọc private key; cả hai đều có đáp án tốt hơn.

### Capability: root, được xẻ nhỏ
Linux chia quyền lực của root thành ~40 **capability**. Một process giữ các tập của chúng
(permitted, effective, inheritable, bounding, ambient), và một thao tác đặc quyền cần capability
cụ thể, không phải UID 0. Những cái bạn sẽ gặp:

- `CAP_NET_BIND_SERVICE` — bind port dưới 1024. Cách sửa kinh điển cho "proxy của tôi phải listen
  trên 443": cấp riêng cái này, không phải root (`setcap cap_net_bind_service=+ep ./proxy`, hoặc
  `AmbientCapabilities=` trong một systemd unit, hoặc `--cap-add` trong Docker). Hoặc hạ ngưỡng:
  `sysctl net.ipv4.ip_unprivileged_port_start=443`, hoặc để supervisor mở socket.
- `CAP_NET_RAW` — raw socket và bắt packet (`ping`, `tcpdump`).
- `CAP_NET_ADMIN` — cấu hình interface, route, firewall, `tc`.
- `CAP_SYS_ADMIN` — "root mới": một mớ hàng chục thao tác; cấp nó cho container gần như tương
  đương trao root.
- `CAP_SYS_RESOURCE`, `CAP_SYS_NICE`, `CAP_IPC_LOCK` — vượt giới hạn tài nguyên, nâng scheduling
  priority, khóa bộ nhớ ([`20-limits-and-proc.md`](20-limits-and-proc.md), [`12-cpu-scheduling.md`](12-cpu-scheduling.md)).

`getpcaps <pid>` hoặc `grep Cap /proc/<pid>/status` hiện các tập của một process
(`capsh --decode=<hex>` giải mã chúng). Container khởi đầu với một tập mặc định đã được cắt bớt;
tư thế an toàn là **bỏ hết rồi thêm lại** chỉ những gì cần.

### Pattern drop-privileges
Khi thực sự cần root trong thời gian ngắn (một cách khởi động truyền thống), hãy làm việc đặc
quyền *trước*, rồi hạ đặc quyền vĩnh viễn:

1. Bind port thấp và mở các file certificate/private key.
2. Chuyển sang một user không đặc quyền — đúng thứ tự: `setgroups` (xóa group phụ), rồi `setgid`,
   rồi `setuid` (sau `setuid` bạn không `setgid` được nữa).
3. Xác minh bạn không thể lấy lại root, và tiếp tục chạy proxy.

Một fd đã mở vẫn hợp lệ sau khi hạ quyền — đó là toàn bộ mẹo: listening socket và key material
được lấy khi còn đặc quyền, rồi dùng khi không đặc quyền. Rust: crate `nix`
(`nix::unistd::setuid`) hoặc `libc`; **kiểm tra mọi giá trị trả về** — một `setuid` fail bị bỏ
qua khiến bạn vẫn âm thầm là root. Ngày nay còn tốt hơn là để *supervisor* khởi động bạn không
đặc quyền với socket mở sẵn ([`21-systemd-and-services.md`](21-systemd-and-services.md): socket activation) hoặc với `User=` và
ambient capability phù hợp, và bỏ qua màn vũ điệu này.

### Gia cố nhiều lớp ngoài UID
- **`no_new_privs`** (`prctl(PR_SET_NO_NEW_PRIVS)`): process không bao giờ có thêm đặc quyền qua
  exec/binary setuid — hãy set nó, vì seccomp đòi hỏi.
- **seccomp**: danh sách cho phép các syscall mà process được gọi; một exploit lạc lối thử `execve`
  sẽ bị giết. Lớp gia cố sâu nhất, và là nơi các eBPF filter cũng sống
  ([`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md)).
- **Namespace và cgroup** ([`13-containers.md`](13-containers.md)) giới hạn thứ process thấy và dùng.
- **Mandatory access control** (SELinux, AppArmor) thêm policy áp dụng cả cho file của root. Một
  bất ngờ production phổ biến: mọi thứ trông được cho phép theo mode bit mà vẫn fail với `EACCES`
  vì một nhãn SELinux cấm (`ausearch -m avc`, `dmesg`).

### Gotcha: secret trên đĩa và trong `/proc`
Private key và token nên có mode `0600` (hoặc `0400`) thuộc user của service, nằm trong một
directory người khác không đi xuyên qua được. Secret truyền qua **command-line argument** hiện
ra với mọi user trong `ps` và `/proc/<pid>/cmdline`; **environment** đọc được ở
`/proc/<pid>/environ` bởi cùng user và root. Ưu tiên file với permission chặt, hoặc một secret
mount. Và nhớ rằng một core dump ([`20-limits-and-proc.md`](20-limits-and-proc.md)) của proxy chứa key của nó.

## Practice

1. Chạy `id`, `ls -l /etc/shadow /etc/passwd /usr/bin/sudo`; xác định owner, mode bit và (với
   `sudo`) bit setuid (`s`), và giải thích bit nào cho phép `sudo` của một user thường chạy như
   root.
2. Tạo `d/f` với các mode bạn chọn, rồi dự đoán và kiểm chứng bạn có `ls d`, `cat d/f`, và
   `rm d/f` được không với mode `000`/`500`/`700` trên directory — xác nhận việc xóa phụ thuộc
   permission của directory.
3. Với một user không đặc quyền thử `python3 -m http.server 80` (chờ `PermissionError`), rồi cấp
   capability: `sudo setcap cap_net_bind_service=+ep $(readlink -f $(which python3))` (gỡ bằng
   `setcap -r`) hoặc dùng [`labs/00-tcp-server`](../../labs/00-tcp-server) trên port 80 với capability, và kiểm tra
   `getpcaps`/`/proc/<pid>/status` (`CapEff`).
4. Thử `sysctl net.ipv4.ip_unprivileged_port_start` trước và sau khi hạ nó (và khôi phục).
5. Chạy server của bạn trong một container non-root với `--cap-drop ALL --cap-add NET_BIND_SERVICE
   --user 1000` và bind port 80; rồi thử một thao tác cần capability đã bỏ (`ip link add`) và đọc
   lỗi.
6. Viết một chương trình Rust scratch bind port 80 bằng root, rồi hạ xuống `nobody` (`setgroups`,
   `setgid`, `setuid`, qua `nix`/`libc`), chứng minh bằng `id`/`/proc/self/status` rằng nó không
   thể lấy lại root (`setuid(0)` trả lỗi), và vẫn nhận connection trên listener đã mở sẵn.
