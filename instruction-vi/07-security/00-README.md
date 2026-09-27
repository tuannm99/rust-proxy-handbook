# Security

Phase 7. Một proxy là thứ đứng giữa internet và mọi thứ phía sau nó, nên
phần lớn nội dung ở đây xoay quanh khoảng cách giữa *bạn nghĩ* một request
nói gì và upstream sẽ *hiểu* nó nói gì.

## Files

**Identity** (đọc `01-network/06-crypto-basics.md` trước nếu "signature,"
"public key," hay "certificate chain" chưa phải thuật ngữ chính xác với
bạn — `02-jwt.md` và `03-mtls.md` đều giả định điều đó)
- `01-auth.md` — auth nằm ở đâu trong pipeline, truyền identity lên upstream, và loại bỏ identity header giả mạo
- `02-jwt.md` — xác thực signature và claim, algorithm confusion, xoay vòng JWKS, revocation
- `03-mtls.md` — client certificate, giới hạn phạm vi CA, hết hạn như một outage đã lên lịch sẵn

**Input handling**
- `04-normalization.md` — parser differential: decode depth, Unicode, thứ tự ưu tiên tham số
- `05-request-smuggling.md` — CL.TE/TE.CL/TE.TE, downgrade smuggling, kẻ tấn công thực sự được gì
- `06-waf.md` — rule engine, signature, anomaly scoring, triển khai ở chế độ chỉ phát hiện

**Abuse and overload**
- `07-ratelimit.md` — key trên cái gì, tính phí theo cost, giới hạn phân tán và các kiểu thất bại của nó
- `08-ip-filtering.md` — matching CIDR, IPv4-mapped IPv6, tin `X-Forwarded-For` đúng cách
- `09-ddos.md` — cost asymmetry, resource ceiling, giới hạn tốc độ accept, decompression bomb
- `10-slowloris.md` — ba biến thể slow-client và rate floor
- `11-load-shedding.md` — shed vs queue, giới hạn theo thời gian, adaptive concurrency

## Reading order

`04-normalization.md` đọc sớm — `router.md`, `06-waf.md`, và
`05-request-smuggling.md` đều là ứng dụng của nó, và không cái nào hoạt
động đúng nếu nó sai. `01-auth.md` trước `02-jwt.md`/`03-mtls.md`, vì nó
đóng khung mục đích của các cơ chế này. `09-ddos.md` trước `10-slowloris.md`
và `11-load-shedding.md`, hai sub-topic lớn nhất của nó.

Các file này backing cho `labs/11-rate-limit`, `labs/12-waf`, và
`labs/17-ebpf`, và `proxy/00-README.md` liệt kê phần lớn thư mục này là
tài liệu bắt buộc đọc cho bản build cuối cùng.
