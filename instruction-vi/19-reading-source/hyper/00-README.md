# hyper

Thư viện HTTP mà `labs/02-http-server` trở đi được xây trên đó. Chưa được
viết.

Ngoại lệ đọc sớm còn lại: đọc codec `h1` ngay sau khi hoàn thành parser
của riêng bạn (`05-http-stack/01-parser.md`, Practice bước 8) và diff cách
nó xử lý các trường hợp mập mờ về framing so với parser của bạn. Trước khi
tự viết parser của mình, nó chỉ là code; ngay sau đó, nó là một danh sách
các trường hợp bạn đã bỏ sót.

Các file dự kiến (xem `19-reading-source/00-README.md` để biết template):

- `architecture.md`
- `request-flow.md`
- `memory.md`
- `interesting-code.md`
- `what-to-learn.md`
