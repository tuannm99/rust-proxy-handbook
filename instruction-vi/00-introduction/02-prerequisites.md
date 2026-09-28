# Điều kiện tiên quyết: Phase 0

Những gì bạn cần có *trước* [`labs/00-tcp-server`](../../labs/00-tcp-server), và cách để có nếu chưa
có. Handbook dạy systems, networking, và async Rust từ một điểm xuất phát
thực dụng, nhưng cố ý không dạy syntax Rust — [`03-rust/`](../03-rust) nói rõ điều đó ngay
đoạn đầu. Nếu bạn bắt đầu với những lỗ hổng lớn về Rust, networking, hay
hệ điều hành, đây là nơi bắt đầu, và dành ba bốn tháng ở đây là hoàn toàn
bình thường.

## What to learn

### Tự kiểm tra: bạn đã sẵn sàng cho lab 00 chưa?
Trả lời mà không tra cứu gì. Mỗi câu "chưa" trỏ tới phần bên dưới lấp nó.

**Rust:**
- Bạn có viết được một `struct` và một `enum` có dữ liệu, rồi `match` trên enum đó không?
- Bạn có viết được một hàm trả về `Result<T, E>` và dùng `?` bên trong không?
- Gặp một lỗi borrow checker ("cannot borrow `x` as mutable because it is also borrowed as immutable"), bạn có giải thích được *vì sao* compiler từ chối, chứ không chỉ làm nó biến mất bằng `.clone()` không?
- Bạn có dùng được `Vec`, `HashMap`, `String` vs `&str`, iterator với `.map()`/`.filter()`, và closure không?
- Bạn đã từng viết một trait và implement nó cho hai type chưa?

**Networking:**
- Bạn có mô tả được, từng bước, chuyện gì xảy ra giữa lúc gõ `curl http://example.com` và lúc thấy HTML — tra DNS, bắt tay TCP, HTTP request, response không?
- Bạn có biết port là gì, và vì sao hai chương trình không thể cùng listen trên port 8080 không?

**Hệ điều hành:**
- Bạn có giải thích được sự khác nhau giữa process và thread không?
- Bạn có biết system call và file descriptor là gì, ít nhất ở mức đại khái không?

Nếu mọi câu Rust đều là "có", nhảy sang lab 00 và dùng track người mới của
[`01-network/`](../01-network) và [`02-linux/`](../02-linux) song song. Nếu phần lớn là "chưa", làm phần
còn lại của file này trước.

### Rust từ số 0: lộ trình tối thiểu
Đi qua *The Rust Programming Language* ("the Book", miễn phí tại
doc.rust-lang.org/book) theo thứ tự, tới hết các chương về generics,
trait, lifetime, closure, iterator, smart pointer, và fearless
concurrency. Tự gõ lại mọi ví dụ — đọc Rust mà không compile gần như không
dạy được gì, vì chính các thông báo lỗi của compiler là một nửa giáo trình.

Chạy **Rustlings** (github.com/rust-lang/rustlings) song song: các bài tập
nhỏ không compile cho tới khi bạn sửa đúng. Đây là con đường nhanh nhất
biến "tôi đã đọc về ownership" thành "tôi làm borrow checker hài lòng được".

Sau đó đọc hai chương nối thẳng sang handbook này:
- Chương async của the Book — vốn từ vựng mà [`03-rust/05-async.md`](../03-rust/05-async.md) xây tiếp.
- Dự án cuối của the Book, "Building a Multithreaded Web Server" — một TCP
  server với thread pool tự viết, chính là thiết kế mà [`labs/00-tcp-server`](../../labs/00-tcp-server)
  thay bằng tokio. Làm phiên bản thread pool trước sẽ khiến *vì sao* async
  tồn tại trở nên hiển nhiên.

Cuối cùng là **Tokio tutorial** chính thức (tokio.rs/tokio/tutorial), xây
một mini-Redis từng bước. Nó bao quát `spawn`, shared state, channel, và
framing — đúng bộ công cụ của lab 00-05.

Gotcha: đừng bắt đầu [`03-rust/`](../03-rust) hay [`04-runtime/`](../04-runtime) trước khi xong các chương
trait và lifetime của the Book. Hai thư mục đó giải thích *vì sao* `Pin`
và các bound `Send` tồn tại; thiếu nền tảng, câu nào đọc cũng như nhiễu,
và bạn sẽ kết luận — sai — rằng mình không đủ khả năng.

### Networking và OS: dùng track người mới ở đây
Bạn không cần một khóa học bên ngoài. [`01-network/`](../01-network) và [`02-linux/`](../02-linux) mỗi thư
mục có phần "Cách đọc thư mục này" với một track người mới: đọc mọi file
theo thứ tự, bắt đầu từ `01-fundamentals.md`, và làm Practice trước khi đi
tiếp. Hai nhóm fundamentals đó được viết như primer từ số 0 cho đúng tình
huống này.

Nếu muốn thêm một góc nhìn thứ hai, có hai tài liệu miễn phí đúng độ sâu:
*Beej's Guide to Network Programming* (socket bằng C, tầng mà tokio che
đi) và *Operating Systems: Three Easy Pieces* (phần virtualization và
concurrency). Cả hai có trong [`21-reading-list/`](../21-reading-list).

### Các cổng sẵn sàng
Đừng vượt qua một cổng khi chưa pass nó. Đây là bài kiểm tra, không phải
bài đọc.

| Trước | Bạn làm được |
| --- | --- |
| [`labs/00-tcp-server`](../../labs/00-tcp-server) | Tự làm lại dự án multithreaded web server của the Book không nhìn tài liệu; giải thích được mọi lỗi borrow gặp phải |
| [`labs/01-http-parser`](../../labs/01-http-parser) | Viết lại lab 00 từ đầu không nhìn code cũ; giải thích được mỗi `.await` đang chờ cái gì |
| [`labs/02-http-server`](../../labs/02-http-server) | Giải thích được vì sao một lần đọc TCP có thể trả về nửa HTTP request, và parser của bạn xử lý chuyện đó thế nào |
| [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) | Giải thích được `Arc<Mutex<T>>` so với channel cho shared state, và vì sao một future phải `Send` mới spawn được |

### Ngân sách thời gian
Với khoảng mười giờ mỗi tuần: the Book cộng Rustlings mất hai tới ba
tháng với người bắt đầu từ số 0; track người mới của [`01-network/`](../01-network) và
[`02-linux/`](../02-linux) mất thêm khoảng một tháng và có thể chạy chồng lên nhau.
[`00-introduction/03-study-loop.md`](03-study-loop.md) có phần còn lại của timeline.

## Practice
1. Trả lời phần tự kiểm tra ở trên bằng văn bản, trung thực. Giữ lại câu trả lời — bạn sẽ làm lại sau phase 0 và so sánh.
2. Hoàn thành dự án multithreaded web server của the Book. **Xong khi** nó phục vụ request đồng thời qua thread pool của chính bạn và shutdown sạch khi drop.
3. Làm xong Rustlings. **Xong khi** mọi bài pass và bạn giải thích được cách sửa của bất kỳ mười bài nào cho người khác.
4. Hoàn thành mini-Redis của Tokio tutorial. **Xong khi** hai client set và get key đồng thời được trên server của bạn.
5. Đọc [`01-network/01-fundamentals.md`](../01-network/01-fundamentals.md) và [`02-linux/01-fundamentals.md`](../02-linux/01-fundamentals.md), làm phần Practice của chúng, rồi làm lại phần tự kiểm tra. **Xong khi** mọi câu trả lời đều là "có".
