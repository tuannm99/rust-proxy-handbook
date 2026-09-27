# Linux/OS Fundamentals

## Status: ngoại lệ duy nhất của "không phải một tutorial"

Cùng một ngoại lệ như `01-network/01-fundamentals.md`, vì cùng một lý do:
`07-epoll.md`, `09-memory.md`, `10-signals.md`, và `11-zerocopy.md` đều mở
đầu bằng cách giả định bạn đã biết syscall, file descriptor, và address
space của một process là gì. File này cùng năm file anh em của nó tồn tại
để đảm bảo điều đó đúng trước khi bạn chạm vào chúng. Bỏ qua cả nhóm nếu
bạn đã biết rồi.

## What to learn

### Kernel tồn tại để làm gì
**Kernel** là chương trình duy nhất trên máy được phép nói chuyện trực tiếp
với hardware, quản lý memory cho mọi process, và enforce cách ly giữa
chúng. Mọi thứ khác — proxy của bạn, shell của bạn, mọi process khác —
chạy trong **user space**, không có quyền truy cập hardware trực tiếp và
không thể đụng vào memory của process khác. Gần như mọi chủ đề trong
`02-linux/` là hệ quả của đúng một ranh giới đó: cái giá phải trả khi băng
qua nó (`03-kernel-and-syscalls.md`), nó cách ly cái gì
(`02-processes-and-threads.md`, `06-containers.md`), và chuyện gì xảy ra
khi kernel cần ngắt bạn thay vì chờ được hỏi
(`05-blocking-io-and-signals.md`).

### Năm mảnh, và mỗi mảnh nằm ở đâu
- **`02-processes-and-threads.md`** — process và thread thật ra là gì, vì
  sao thread rẻ hơn, và vì sao chia sẻ memory giữa chúng là lý do
  `03-rust/04-sync.md` tồn tại.
- **`03-kernel-and-syscalls.md`** — ranh giới user space/kernel space,
  syscall tốn bao nhiêu, và file descriptor — cái handle dạng số nguyên mà
  mọi thứ trong `02-linux/07-epoll.md` được xây quanh nó.
- **`04-memory-basics.md`** — mức tối thiểu cần để đoạn mở đầu của
  `02-linux/09-memory.md` cảm giác quen thuộc thay vì thông tin mới, cộng
  thêm RAM nằm ở đâu so với cache và disk.
- **`05-blocking-io-and-signals.md`** — vì sao một syscall có thể block
  một thread, vì sao event loop tồn tại như một giải pháp thay thế, và
  signal là gì (một sự gián đoạn từ bên ngoài luồng điều khiển bình
  thường của bạn, không phải một giá trị trả về).
- **`06-containers.md`** — container thật ra là gì (các process bị cách ly
  chạy trên cùng một kernel dùng chung, không phải một VM tí hon), vì
  Kubernetes/pod/cgroup được nhắc tới liên tục từ `09-architecture/` trở
  đi mà chưa có chỗ nào định nghĩa chúng.

Đọc theo thứ tự đó một lần; sau đó, coi mỗi file như một chỗ tra cứu độc
lập.

## Practice
1. Chạy `uname -a` và đọc phiên bản kernel của bạn; chạy `ps aux` và chọn
   ba process — với mỗi process, đoán (rồi kiểm chứng bằng `man`/docs) nó
   làm gì.
2. Đọc năm file anh em theo thứ tự, rồi quay lại đây và giải thích, mỗi ý
   một câu: vì sao thread rẻ hơn process, syscall thật ra băng qua cái gì,
   vì sao `read()` có thể block, và container cách ly cái gì mà một
   process bình thường không có.
3. Chạy `cat /proc/version` và `cat /proc/cpuinfo | grep -c processor` —
   xác nhận bạn tìm được phiên bản kernel và số core mà không cần công cụ
   GUI.
