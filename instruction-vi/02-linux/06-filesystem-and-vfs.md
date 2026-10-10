# File và Filesystem: Inode, VFS, Page Cache, và Durability

Một "file" thực sự là gì trên Linux, một path resolve ra cái gì, và `write()` hứa và không
hứa những gì. Cần cho log file, static serving, config reload, và để hiểu vì sao "mọi thứ
là file" cho phép một `epoll` theo dõi socket và pipe như nhau
([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)).

## What to learn

### Inode: file không phải là tên của nó
Filesystem lưu mỗi file dưới dạng một **inode**: một record giữ loại file, kích thước, chủ
sở hữu, permission ([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)), timestamp và vị trí các data block của
nó — nhưng **không giữ tên**. Tên nằm trong **directory**, vốn chỉ là các file ánh xạ
`tên -> inode number`. Một tên là một **hard link** tới một inode; một inode có thể có nhiều
tên, và inode (cùng data) chỉ được giải phóng khi **link count** về 0 *và* không process nào
còn mở nó. Những hệ quả bạn sẽ gặp:

- `rm` một file mà một process vẫn đang mở sẽ gỡ tên, nhưng data vẫn sống và tiếp tục chiếm
  đĩa cho tới khi fd cuối đóng (`df` bảo đầy, `du` bảo không — `lsof +L1` tìm ra các file như
  vậy; kinh điển sau khi rotate log mà không báo cho process).
- **Symlink** là thứ khác: một file nhỏ có nội dung là một path, được resolve lúc dùng (nó có
  thể dangle, và có thể trỏ tới bất cứ đâu).
- Rename trong cùng một filesystem chỉ là sửa directory — atomic, và không có data di chuyển.

```text
$ ls -li file hardlink symlink      # -i in inode number
1048590 -rw-r--r-- 2 u u  5 file
1048590 -rw-r--r-- 2 u u  5 hardlink   <- cùng inode, link count 2
1048600 lrwxrwxrwx 1 u u  4 symlink -> file
```

### Path resolution và VFS
Mở `/var/log/proxy/access.log` khiến kernel đi dọc path: bắt đầu từ root directory, tra
`var` trong đó, rồi `log`, `proxy`, `access.log`, kiểm tra permission trên mỗi directory dọc
đường (cần quyền **execute/search** trên directory, không chỉ read). Kernel cache các lần tra
đó trong **dentry cache**. **VFS** (virtual filesystem) là layer làm mọi thứ đồng nhất: lời
gọi `open`/`read`/`write` của bạn đi tới VFS, cái này chuyển cho filesystem cụ thể — ext4,
xfs, tmpfs (RAM), overlayfs (container, [`13-containers.md`](13-containers.md)), NFS (mạng), `/proc` (state
của kernel được phơi ra dưới dạng file). **Mount** ghép một filesystem vào một directory của
filesystem khác; một path cross mount point mà không thấy. Đó cũng là lý do một path chỉ
là một *yêu cầu*: giữa `stat` và `open` của bạn, ai đó có thể tráo một symlink — race
**TOCTOU** (time-of-check to time-of-use) đằng sau nhiều lỗ hổng file-serving
([`05-http-stack/06-static.md`](../05-http-stack/06-static.md), [`07-security/04-normalization.md`](../07-security/04-normalization.md)). Phòng thủ bằng cách mở trước, rồi
kiểm tra *fd đã mở* (`fstat`), không bao giờ kiểm tra path.

### File descriptor vs open file description
`open()` trả về một fd — một index trong bảng của process — trỏ tới một **open file
description** trong kernel giữ **offset hiện tại** và các flag, cái này trỏ tới inode.
`dup()`/`fork()` copy *fd*, không phải description, nên các bản sao **chia sẻ một offset**:
parent và child cùng write vào một fd thừa kế sẽ xen kẽ đúng, và hai `read` trên các fd được
dup sẽ tiêu thụ dữ liệu nối tiếp nhau. Mở cùng một path hai lần tạo hai description độc lập
với offset riêng. `O_APPEND` làm mỗi `write` seek về cuối một cách atomic, đó là lý do nhiều
process có thể append an toàn các dòng nguyên vẹn vào một file log. `pread`/`pwrite` nhận
offset tường minh và không đụng offset chung — thứ mà thread pool và các helper file của tokio
dùng.

### Buffered, cached, và durable là ba thứ khác nhau
Một `write()` vào file thường trả về ngay khi data được copy vào **page cache** của kernel
([`16-kernel/08-page-cache.md`](../16-kernel/08-page-cache.md)); kernel ghi nó xuống đĩa sau (**write-back**, trong vòng vài
giây). Vì thế một `write` thành công nghĩa là "process khác đọc được nó" nhưng **không** phải
"nó đã nằm trên đĩa": mất điện hay kernel crash có thể làm mất nó. `fsync(fd)` block cho tới
khi data (và metadata) của file đó tới được bộ lưu trữ ổn định — chậm (mili giây trên SSD) và
là giá của durability. Có một buffer thứ hai ở trên nữa trong user space (`BufWriter` của
Rust, `stdio` của libc): data ở đó thậm chí chưa vào kernel, nên crash hoặc `process::exit` làm
mất nó trừ khi flush.

Read chạm page cache trước, đó là lý do lần đọc thứ hai của một file nóng nhanh như bộ nhớ và
vì sao `sendfile` có thể stream static asset mà không đụng userspace
([`18-zerocopy.md`](18-zerocopy.md), [`16-memory.md`](16-memory.md)). Đọc file nguội thì block ở đĩa — không như socket, file
thường **luôn "ready"** với `epoll`, nên bạn không thể làm disk I/O non-blocking bằng `epoll`.
tokio chạy các thao tác file trên một blocking thread pool chính vì lý do này (`tokio::fs`);
io_uring là câu trả lời async-file thật sự ([`15-io_uring.md`](15-io_uring.md)).

### Atomic replace: write-temp, fsync, rename
Để cập nhật một file mà reader có thể thấy giữa chừng khi đang ghi (config, certificate, cache
file), đừng bao giờ ghi đè tại chỗ. Pattern: ghi một file tạm **trong cùng directory** (cùng
filesystem), `fsync` nó, `rename(tmp, final)` — reader thấy file cũ hoặc file mới, không bao
giờ là một mớ rách, vì rename là atomic. Config reload
([`09-architecture/03-config.md`](../09-architecture/03-config.md)) và xoay certificate dựa vào điều này. Process sau đó có thể được báo
reload bằng một signal ([`17-signals.md`](17-signals.md)) hoặc tự nhận ra qua **inotify** — một cơ chế của
kernel giao các event "path này đã đổi" dưới dạng fd đọc được (crate `notify`). Hãy watch
*directory*: editor lưu bằng rename, nên inode đang watch biến mất.

### Filesystem đặc biệt: /proc, /sys, /dev
`/proc` và `/sys` không nằm trên đĩa: chúng là state của kernel được trình bày dưới dạng file.
`/proc/<pid>/fd`, `/proc/<pid>/status`, `/proc/meminfo`, `/proc/net/tcp` và `/sys/class/net/`
cho phép bạn soi một hệ thống đang chạy bằng `cat` ([`20-limits-and-proc.md`](20-limits-and-proc.md)). `/dev` chứa các
device node (`/dev/null`, `/dev/urandom`, `/dev/tty`). `tmpfs` (`/tmp` hoặc `/dev/shm` trên nhiều
hệ thống) nằm trong RAM — nhanh, nhưng tính vào bộ nhớ, và biến mất khi reboot.

### Giới hạn, lỗi, và đĩa đầy
Một filesystem có thể "đầy" theo hai cách: hết **block** (`df -h`) hoặc hết **inode** (`df -i`;
hàng triệu cache file nhỏ sẽ làm vậy) — cả hai đều cho `ENOSPC`. Một proxy ghi access log vào
đĩa đầy sẽ gặp lỗi write ngay trên request path; hãy quyết định có chủ đích rằng việc đó làm
fail request hay bỏ log ([`08-observability/01-logging.md`](../08-observability/01-logging.md)). `EMFILE`/`ENFILE` nghĩa là giới hạn
fd theo process/hệ thống ([`20-limits-and-proc.md`](20-limits-and-proc.md)).

### Gotcha: log rotation và file đã bị chuyển
`logrotate` đổi tên `access.log` thành `access.log.1` và tạo file mới — nhưng process của bạn
vẫn giữ fd tới *inode cũ* và tiếp tục ghi vào `access.log.1`. Sửa bằng cách mở lại khi nhận
signal (`SIGHUP`, [`17-signals.md`](17-signals.md)), hoặc `copytruncate` (dễ race), hoặc log ra stdout và để
supervisor lo việc file.

## Practice

1. Chạy `stat file` và `ls -li`, tạo một hard link (`ln`) và một symlink (`ln -s`), và cho thấy
   inode number và link count. Xóa bản gốc và cho thấy cái nào còn sống, cái nào dangle.
2. Tạo một file 100 MB, mở nó bằng `tail -f` (hoặc một chương trình scratch giữ fd), `rm` nó, và
   cho thấy bằng `df`, `lsof +L1` và `ls -l /proc/<pid>/fd` rằng dung lượng vẫn đang bị dùng; rồi
   kết thúc process và xem `df` giảm.
3. Cho thấy một log `O_APPEND` được hai process ghi cùng lúc
   (`for i in $(seq 1000); do echo a >> f; done & ...`) ra nguyên vẹn, và đối chiếu với hai
   process ghi ở các offset tường minh (`pwrite`) ghi đè lên nhau.
4. Ghi 1 GiB có và không có `fsync` trong một chương trình Rust scratch (`File::sync_all`), đo
   thời gian cả hai, và giải thích khoảng cách; kiểm tra `grep -E 'Dirty|Writeback' /proc/meminfo`
   trong lúc ghi không sync.
5. Implement pattern write-temp-fsync-rename cho một config file trong một chương trình scratch,
   chạy một vòng reader (`while true; do cat cfg; done`) song song, và xác nhận nó không bao giờ
   thấy file dở dang; rồi cố tình phá nó (ghi tại chỗ) và bắt một lần đọc bị rách. Đây là cơ chế
   dưới [`labs/13-hot-reload`](../../labs/13-hot-reload); trace nó bằng
   `strace -f -e trace=openat,read,write,fsync,rename` và xác định từng syscall.
6. Trong [`labs/04-static-server`](../../labs/04-static-server), phục vụ một symlink trỏ ra ngoài root được
   phục vụ và xác nhận server của bạn từ chối nó, rồi giải thích race TOCTOU mà việc mở theo path
   rồi mới kiểm tra sẽ để lại.
