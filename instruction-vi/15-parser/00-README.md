# Parser

Lý thuyết parsing tổng quát. [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) là ứng dụng
riêng cho HTTP của những ý tưởng này (và [`labs/01-http-parser`](../../labs/01-http-parser) là nơi bạn
implement nó) — thư mục này là nền tảng tái sử dụng bên dưới, cũng áp dụng
được cho việc parse config file trong [`09-architecture/03-config.md`](../09-architecture/03-config.md).

## Trạng thái: đã viết, nhưng không bắt buộc cho các lab

Lý thuyết bên dưới đã được viết, nhưng không có crate [`labs/`](../../labs) nào bắt buộc
phải dùng nó: [`05-http-stack/01-parser.md`](../05-http-stack/01-parser.md) được thiết kế tự chứa hoàn toàn
cho [`labs/01-http-parser`](../../labs/01-http-parser). Đọc thư mục này khi bạn tới [`labs/13-hot-reload`](../../labs/13-hot-reload)
và quyết định tự viết một config format của riêng mình thay vì dựa vào
`serde` + `toml` — đó là lúc lexer/AST/visitor không còn là lý thuyết nữa
([`05-config-parser.md`](05-config-parser.md) đưa ra rõ ràng câu hỏi "bạn có thực sự cần một
parser không?"). Ngoài ra, coi đây là kiến thức nền giúp đào sâu thêm HTTP
parser.

## Files

- [`01-lexer.md`](01-lexer.md) — tokenize byte/text thô thành một stream token
- [`02-parser.md`](02-parser.md) — biến một stream token thành dữ liệu có cấu trúc (recursive descent vs parser combinator)
- [`03-ast.md`](03-ast.md) — biểu diễn cấu trúc đã parse dưới dạng cây, và vì sao bạn muốn có một cây thay vì parse thẳng vào target type
- [`04-visitor.md`](04-visitor.md) — visitor pattern để duyệt/biến đổi một AST
- [`05-config-parser.md`](05-config-parser.md) — áp dụng tất cả những điều trên vào config format của chính proxy, nuôi [`09-architecture/03-config.md`](../09-architecture/03-config.md)
