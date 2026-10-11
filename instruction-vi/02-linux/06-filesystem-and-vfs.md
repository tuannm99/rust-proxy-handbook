# File và Filesystem: Inode, VFS, Page Cache và Durability

File này giải thích: một "file" thực sự là gì trên Linux, một path được resolve ra cái gì, và
`write()` hứa gì và không hứa gì. Bạn cần nó cho log file, static serving, config reload. Nó
cũng giúp hiểu vì sao "mọi thứ là file" cho phép một `epoll` theo dõi socket và pipe theo cùng
một cách ([`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md)).

## What to learn

### Inode: file không phải là cái tên của nó
Filesystem lưu mỗi file dưới dạng một **inode**. Đó là một record chứa: loại file, kích thước,
chủ sở hữu, permission ([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)),
timestamp, và vị trí các data block. Inode **không chứa tên**.

Tên nằm trong **directory**. Directory chỉ là một file ánh xạ `tên -> inode number`. Một tên là
một **hard link** tới một inode. Một inode có thể có nhiều tên. Inode (cùng dữ liệu của nó) chỉ
được giải phóng khi **link count** về 0 *và* không còn process nào đang mở nó.

Những hệ quả bạn sẽ gặp:

- `rm` một file mà một process vẫn đang mở chỉ gỡ cái tên. Dữ liệu vẫn sống và vẫn chiếm đĩa
  cho đến khi fd cuối cùng được đóng. Khi đó `df` báo đĩa đầy còn `du` báo không. `lsof +L1`
  tìm ra những file như vậy. Đây là tình huống kinh điển sau khi rotate log mà không báo cho
  process.
- **Symlink** là thứ khác hẳn: một file nhỏ có nội dung là một path, và được resolve lúc dùng.
  Nó có thể trỏ vào hư không (dangle), và có thể trỏ tới bất cứ đâu.
- Rename trong cùng một filesystem chỉ là sửa directory. Nó atomic, và không có dữ liệu nào bị
  di chuyển.

```text
$ ls -li file hardlink symlink      # -i in inode number
1048590 -rw-r--r-- 2 u u  5 file
1048590 -rw-r--r-- 2 u u  5 hardlink   <- cùng inode, link count 2
1048600 lrwxrwxrwx 1 u u  4 symlink -> file
```

### Path resolution và VFS
Khi bạn mở `/var/log/proxy/access.log`, kernel đi dọc theo path: bắt đầu từ root directory, tìm
`var` trong đó, rồi `log`, `proxy`, `access.log`. Ở mỗi directory trên đường đi, kernel kiểm tra
permission. Bạn cần quyền **execute/search** trên directory, không chỉ quyền read. Kernel cache
kết quả các lần tra này trong **dentry cache**.

**VFS** (virtual filesystem) là layer làm cho mọi thứ đồng nhất. Lời gọi `open`/`read`/`write`
của bạn đi vào VFS, và VFS chuyển cho filesystem cụ thể:
- ext4, xfs: filesystem trên đĩa.
- tmpfs: nằm trong RAM.
- overlayfs: dùng cho container ([`13-containers.md`](13-containers.md)).
- NFS: qua mạng.
- `/proc`: trạng thái của kernel được trình bày dưới dạng file.

**Mount** gắn một filesystem vào một directory của filesystem khác. Một path băng qua mount
point mà bạn không hề thấy.

Điều này cũng có nghĩa là path chỉ là một *yêu cầu*. Giữa lúc bạn `stat` và lúc bạn `open`, ai
đó có thể tráo một symlink. Đó là race **TOCTOU** (time-of-check to time-of-use), nguyên nhân
của nhiều lỗ hổng file-serving ([`05-http-stack/06-static.md`](../05-http-stack/06-static.md),
[`07-security/04-normalization.md`](../07-security/04-normalization.md)). Cách phòng thủ: mở file
trước, rồi kiểm tra *fd đã mở* (`fstat`). Không bao giờ kiểm tra bằng path.

### File descriptor và open file description
`open()` trả về một fd, tức một chỉ số trong bảng fd của process. fd này trỏ tới một **open file
description** trong kernel. Description giữ **offset hiện tại** và các flag, và nó trỏ tới inode.

`dup()` và `fork()` copy *fd*, không copy description. Vì vậy các bản sao **dùng chung một
offset**:
- Parent và child cùng write vào một fd thừa kế sẽ ghi nối tiếp nhau đúng thứ tự.
- Hai lần `read` trên các fd đã dup sẽ lấy dữ liệu nối tiếp nhau.

Nếu bạn mở cùng một path hai lần, bạn có hai description độc lập, mỗi cái có offset riêng.

`O_APPEND` làm mỗi lần `write` tự seek về cuối file một cách atomic. Nhờ vậy nhiều process có
thể append nguyên vẹn từng dòng vào một file log. `pread`/`pwrite` nhận offset tường minh và
không đụng đến offset dùng chung. Thread pool và các helper file của tokio dùng chúng.

### Buffered, cached và durable là ba thứ khác nhau
`write()` vào một file thường trả về ngay khi dữ liệu đã được copy vào **page cache** của kernel
([`16-kernel/08-page-cache.md`](../16-kernel/08-page-cache.md)). Kernel ghi nó xuống đĩa sau đó
(**write-back**, trong vòng vài giây).

Vì vậy một `write` thành công nghĩa là "process khác đọc được nó". Nó **không** nghĩa là "nó đã
nằm trên đĩa". Mất điện hoặc kernel crash có thể làm mất dữ liệu.

`fsync(fd)` block cho đến khi dữ liệu (và metadata) của file đó thực sự đến được bộ lưu trữ
ổn định. Nó chậm (mili giây trên SSD), và đó là cái giá của durability.

Còn có một lớp buffer nữa ở trên, trong user space (`BufWriter` của Rust, `stdio` của libc).
Dữ liệu ở đó thậm chí chưa vào kernel. Nếu crash hoặc gọi `process::exit` mà chưa flush, dữ liệu
bị mất.

Read thì chạm page cache trước. Vì vậy lần đọc thứ hai của một file nóng nhanh như đọc bộ nhớ.
Đó cũng là lý do `sendfile` có thể stream static asset mà không đi qua userspace
([`18-zerocopy.md`](18-zerocopy.md), [`16-memory.md`](16-memory.md)).

Đọc file nguội thì bị block ở đĩa. Khác với socket, file thường **luôn "ready"** với `epoll`,
nên bạn không thể làm disk I/O non-blocking bằng `epoll`. Vì vậy tokio chạy các thao tác file
trên một blocking thread pool (`tokio::fs`). io_uring mới là câu trả lời async-file thực sự
([`15-io_uring.md`](15-io_uring.md)).

### Atomic replace: ghi file tạm, fsync, rename
Giả sử có một file mà reader có thể đọc giữa chừng khi bạn đang cập nhật (config, certificate,
cache file). Đừng bao giờ ghi đè tại chỗ. Hãy làm theo pattern sau:

1. Ghi một file tạm **trong cùng directory** (cùng filesystem).
2. `fsync` nó.
3. `rename(tmp, final)`.

Reader thấy file cũ hoặc file mới, không bao giờ thấy một file dở dang, vì rename là atomic.
Config reload ([`09-architecture/03-config.md`](../09-architecture/03-config.md)) và xoay
certificate dựa vào điều này.

Sau đó process có thể được báo reload bằng một signal ([`17-signals.md`](17-signals.md)), hoặc
tự nhận ra qua **inotify**. Đây là cơ chế kernel báo "path này đã đổi" dưới dạng một fd đọc được
(crate `notify`). Hãy watch *directory*, vì editor lưu file bằng rename nên inode bạn đang watch
sẽ biến mất.

### Các filesystem đặc biệt: /proc, /sys, /dev
`/proc` và `/sys` không nằm trên đĩa. Chúng là trạng thái của kernel được trình bày dưới dạng
file. `/proc/<pid>/fd`, `/proc/<pid>/status`, `/proc/meminfo`, `/proc/net/tcp` và
`/sys/class/net/` cho phép bạn soi một hệ thống đang chạy bằng `cat`
([`20-limits-and-proc.md`](20-limits-and-proc.md)).

`/dev` chứa các device node (`/dev/null`, `/dev/urandom`, `/dev/tty`). `tmpfs` (`/tmp`, hoặc
`/dev/shm` trên nhiều hệ thống) nằm trong RAM. Nó nhanh, nhưng tính vào bộ nhớ, và mất khi
reboot.

### Giới hạn, lỗi và đĩa đầy
Một filesystem có thể "đầy" theo hai cách: hết **block** (`df -h`), hoặc hết **inode**
(`df -i`; hàng triệu cache file nhỏ sẽ gây ra chuyện này). Cả hai đều trả `ENOSPC`.

Một proxy ghi access log vào đĩa đầy sẽ gặp lỗi write ngay trên đường xử lý request. Hãy quyết
định có chủ đích: lỗi đó làm request fail, hay chỉ bỏ log
([`08-observability/01-logging.md`](../08-observability/01-logging.md)).

`EMFILE`/`ENFILE` nghĩa là chạm giới hạn fd của process hoặc của cả hệ thống
([`20-limits-and-proc.md`](20-limits-and-proc.md)).

### Gotcha: log rotation và file đã bị đổi tên
`logrotate` đổi tên `access.log` thành `access.log.1` rồi tạo file mới. Nhưng process của bạn vẫn
giữ fd tới *inode cũ*, nên nó tiếp tục ghi vào `access.log.1`. Có ba cách xử lý:
- Mở lại file khi nhận signal (`SIGHUP`, [`17-signals.md`](17-signals.md)).
- Dùng `copytruncate` (dễ gây race).
- Log ra stdout và để supervisor lo việc ghi file.

## Practice

1. Chạy `stat file` và `ls -li`. Tạo một hard link (`ln`) và một symlink (`ln -s`), và cho thấy
   inode number cùng link count. Xóa bản gốc và cho thấy cái nào còn sống, cái nào bị dangle.
2. Tạo một file 100 MB, mở nó bằng `tail -f` (hoặc một chương trình scratch giữ fd), rồi `rm`
   nó. Dùng `df`, `lsof +L1` và `ls -l /proc/<pid>/fd` để cho thấy dung lượng vẫn đang bị dùng.
   Sau đó kết thúc process và xem `df` giảm xuống.
3. Cho hai process cùng ghi vào một log mở bằng `O_APPEND`
   (`for i in $(seq 1000); do echo a >> f; done & ...`) và cho thấy các dòng ra nguyên vẹn. Đối
   chiếu với trường hợp hai process ghi ở các offset tường minh (`pwrite`) và ghi đè lên nhau.
4. Trong một chương trình Rust scratch, ghi 1 GiB có và không có `fsync` (`File::sync_all`). Đo
   thời gian của cả hai và giải thích khoảng cách. Trong lúc ghi không sync, kiểm tra
   `grep -E 'Dirty|Writeback' /proc/meminfo`.
5. Implement pattern ghi-file-tạm-fsync-rename cho một config file trong một chương trình
   scratch. Chạy song song một vòng lặp reader (`while true; do cat cfg; done`) và xác nhận nó
   không bao giờ thấy file dở dang. Sau đó cố tình phá nó (ghi tại chỗ) và bắt một lần đọc bị
   rách. Đây là cơ chế bên dưới [`labs/13-hot-reload`](../../labs/13-hot-reload). Trace nó bằng
   `strace -f -e trace=openat,read,write,fsync,rename` và chỉ ra từng syscall.
6. Trong [`labs/04-static-server`](../../labs/04-static-server), thử phục vụ một symlink trỏ ra
   ngoài thư mục root được phục vụ, và xác nhận server của bạn từ chối nó. Sau đó giải thích race
   TOCTOU còn lại nếu bạn mở theo path rồi mới kiểm tra.
