# Reference

Tài liệu phụ lục — để tra cứu nhanh, không phải một lộ trình học. Được
tham chiếu chéo từ các file topic khác thay vì đọc từ đầu tới cuối.

## Trạng thái: chỉ có index — chưa được viết, và nên được chính bạn viết

Chưa có gì ở đây được viết và chưa có gì liên kết tới nó. Đó ít là một lỗ
hổng hơn vẻ ngoài của nó: một glossary và cheatsheet là công cụ tra cứu,
và phiên bản hữu ích là phiên bản được viết bằng ngôn từ của chính bạn khi
bạn va vào từng thuật ngữ.

Khi nào quay lại: liên tục, như một sản phẩm phụ. Mỗi lần bạn phải tra một
`errno`, một status code, hay ý nghĩa của "head-of-line blocking" theo
nghĩa HTTP/2 khác với nghĩa TCP thế nào, hãy thêm một dòng vào đây. Viết
thư mục này từ trước, khi các thuật ngữ chưa khiến bạn tốn công gì, sẽ tạo
ra một glossary bạn sẽ không bao giờ mở lại.

## Chủ đề dự kiến

- `glossary.md` — các thuật ngữ dùng xuyên suốt handbook (upstream,
  backpressure, head-of-line blocking, v.v.) kèm định nghĩa một dòng và
  một pointer tới file topic bàn sâu về nó
- `cheatsheet.md` — các bảng tra cứu nhanh (status code HTTP đáng xử lý
  riêng, các `errno` phổ biến một proxy sẽ gặp, các bước handshake TLS) để
  tra khi implement
