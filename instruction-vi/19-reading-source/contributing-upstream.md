# Contribute ngược lên upstream

Cách đi từ đọc tokio, hyper, rustls, quinn, hay pingora tới contribute cho
chúng. Đây là bước cuối của vòng lặp trong
[`00-introduction/03-study-loop.md`](../00-introduction/03-study-loop.md): tự build proxy là thứ cho bạn đủ context để
có ích cho các project này, còn contribute là thứ biến kiến thức đó thành
bền vững.

## What to learn

### Chọn hệ sinh thái bạn đã biết
Contribute ở nơi bạn có context. Sau handbook này, đó là các crate mà proxy
của bạn dựng trên — tokio, hyper, h2, rustls, quinn, pingora, và những
crate nhỏ lân cận (`httparse`, `bytes`, `tokio-rustls`, `hyper-util`). Bạn
đã chạm vào API, thông báo lỗi, và có lẽ cả những góc gồ ghề của chúng.
Contribute cho một crate không liên quan mà bạn chưa từng dùng nghĩa là
vừa học domain của nó từ đầu vừa làm.

### Những đóng góp không bắt đầu bằng code
Cách nhanh nhất để trở nên hữu ích thường không phải là một pull request:
- **Tái hiện bug report.** Nhiều issue nằm đó chưa được xác nhận. Một bản tái hiện tối thiểu — một chương trình nhỏ kích hoạt bug ổn định, ghi rõ version — là một trong những đóng góp maintainer quý nhất.
- **Khoanh vùng một bug.** "Chỉ xảy ra với HTTP/2 chứ không phải HTTP/1.1, và chỉ khi body vượt initial window" biến một báo cáo mơ hồ thành thứ sửa được.
- **Sửa documentation từng làm bạn bối rối.** Bạn là tác giả lý tưởng: bạn vừa trải qua sự bối rối đó.
- **Trả lời câu hỏi** trong phần discussion của project từ trải nghiệm gần đây của chính bạn.

### Tìm thay đổi code đầu tiên
Tìm các issue gắn nhãn `good first issue`, `E-easy`, `help wanted`, hay
`A-docs` trong issue tracker của project. Trước khi viết code: comment
rằng bạn muốn nhận nó và phác thảo hướng làm. Maintainer thường biết một
ràng buộc mà bạn không biết, và một câu trả lời hai dòng giúp bạn khỏi mất
một tuần làm sai hướng là chuyện thường gặp.

Gotcha: một PR đầu tiên mà tiện tay "dọn dẹp" cả code không liên quan
trong cùng file sẽ khó review hơn nhiều và dễ bị treo hơn nhiều. Giữ thay
đổi đúng phạm vi issue; đề xuất các dọn dẹp khác riêng.

### Một PR merge được trông thế nào ở các project này
- **Một test fail trước thay đổi và pass sau thay đổi.** Với một bug fix, đây là điều không thể thương lượng ở các project cỡ tokio/hyper.
- **CI pass ở local trước:** `cargo fmt`, `cargo clippy --all-targets`, toàn bộ test suite, và — ở crate có `unsafe` — Miri. tokio còn test concurrency bằng `loom` (một model checker duyệt các cách đan xen thread); nếu bạn đụng vào code synchronization, hãy chuẩn bị cần một loom test.
- **Mô tả giải thích vì sao**, link tới issue, và nêu bạn đã test những gì.
- **Kiên nhẫn với review.** Maintainer là tình nguyện viên hoặc có ưu tiên khác; một tuần im lặng không phải là từ chối. Phản hồi review bằng cách sửa code hoặc giải thích lập luận, không phải bằng cách bảo vệ nó.

### Đọc một codebase lớn nhanh
Các reading guide trong thư mục này ([`tokio/reading-guide.md`](tokio/reading-guide.md),
[`hyper/reading-guide.md`](hyper/reading-guide.md), [`pingora/reading-guide.md`](pingora/reading-guide.md)) áp dụng một phương pháp
dùng lại được ở bất cứ đâu: bắt đầu từ một API public bạn đã gọi, lần vào
trong từng lớp, và giữ một danh sách câu hỏi chạy dọc. Hai công cụ giúp
nhanh hơn nhiều: một editor có "go to definition" và "find references" của
rust-analyzer, và chính các test của repository — một test là một ví dụ
chạy được về đúng cách tác giả kỳ vọng một thành phần được dùng.

### Thảo luận thiết kế và RFC
Những thay đổi lớn ở các project này bắt đầu bằng một issue hoặc tài liệu
thiết kế, không phải một PR. Viết tốt một tài liệu như vậy tự nó là một
kỹ năng: nêu vấn đề bằng một use case cụ thể, liệt kê các phương án đã cân
nhắc và vì sao loại bỏ chúng, và chỉ ra cái gì sẽ gãy. Đây cũng là kiểu lập
luận mà mọi phần "vì sao thiết kế này" trong handbook đòi hỏi ở bạn — thói
quen đó chuyển thẳng sang được.

## Practice
1. Chọn một crate mà proxy của bạn phụ thuộc và đọc `CONTRIBUTING.md` của nó. **Xong khi** bạn chạy được toàn bộ test suite và clippy của nó ở local mà không có lỗi.
2. Tìm một bug report đang mở, chưa được xác nhận trong crate đó và thử tái hiện nó. **Xong khi** bạn đã post một comment kèm bản tái hiện tối thiểu, hoặc ghi chú rằng nó không tái hiện trên version hiện tại và bạn đã thử những gì.
3. Tìm một phần documentation trong crate đó từng làm bạn bối rối khi làm lab, và mở một PR cải thiện nó.
4. Nhận một `good first issue`, comment hướng làm trước khi code, và đưa nó qua review. **Xong khi** nó được merge — hoặc bị đóng kèm một giải thích mà bạn hiểu.
5. Sau khi xong [`proxy/`](../../proxy), viết một ghi chú thiết kế ngắn về một điều bạn muốn thay đổi ở một API của pingora hoặc hyper, theo cấu trúc vấn đề/phương án/đánh đổi ở trên. Bạn không cần gửi nó; viết nó ra chính là bài tập.
