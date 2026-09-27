# PROXY Protocol

Cách một L7 proxy biết được địa chỉ client thật khi nó không phải hop đầu
tiên.

## What to learn

### Vấn đề nó giải quyết
Khi proxy của bạn đứng sau một load balancer hay proxy khác (một cloud
LB, một edge CDN, một reverse proxy khác), kết nối TCP mà proxy của bạn
thấy đến *từ* bên trung gian đó, không phải từ client gốc — `peer_addr()`
trên socket đã accept cho bạn IP của LB, không phải IP của client. PROXY
protocol giải quyết việc này ở tầng TCP, trước khi bất kỳ HTTP parsing
nào diễn ra, bằng cách để hop upstream thêm vào phía trước một header nhỏ
mang địa chỉ của client gốc.

### PROXY protocol v1 (văn bản)
Một dòng đọc được, con người đọc hiểu, được gửi như những byte đầu tiên
của kết nối:

```
PROXY TCP4 192.0.2.1 198.51.100.1 56324 443\r\n
```

`PROXY <family> <src-ip> <dst-ip> <src-port> <dst-port>\r\n`. Đơn giản để
parse và debug, nhưng dài dòng và giới hạn ở TCP4/TCP6/UNKNOWN.

### PROXY protocol v2 (nhị phân)
Một signature 12 byte cố định theo sau bởi một header nhị phân
(version/command, address family/protocol, độ dài, rồi tới khối địa chỉ,
tùy chọn theo sau bởi các TLV cho metadata bổ sung như SNI/ALPN TLS gốc).
Được dùng trong production vì nó rẻ hơn để parse và không mập mờ — không
cần quét delimiter, chỉ offset cố định và một body có tiền tố độ dài.

### Phát hiện và parse nó trước tầng HTTP
Proxy phải peek các byte đầu tiên của một kết nối vừa được accept *trước*
khi đưa nó cho HTTP parser: kiểm tra signature nhị phân v2 trước (các byte
cố định không mập mờ), rồi fallback về kiểm tra tiền tố `PROXY ` theo
nghĩa đen cho v1, và nếu không thì giả định không có header PROXY protocol
nào hiện diện. Làm sai chỗ này — ví dụ đối xử với header như một phần của
HTTP request body — làm hỏng mọi request phía sau nó.

```rust
const V2_SIG: [u8; 12] = [
    0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A,
];

async fn peek_is_proxy_v2(stream: &tokio::net::TcpStream) -> std::io::Result<bool> {
    let mut buf = [0u8; 12];
    let n = stream.peek(&mut buf).await?;
    Ok(n == 12 && buf == V2_SIG)
}
```

### Ranh giới tin cậy
Chỉ chấp nhận một header PROXY protocol từ các kết nối bạn thực sự tin
tưởng (tức là dải IP của LB upstream đã biết của bạn) — nếu không, bất kỳ
ai tiếp cận trực tiếp được listener của bạn đều có thể *giả mạo* địa chỉ
client theo cùng cách một `X-Forwarded-For` không được validate có thể bị
giả mạo ở tầng HTTP (xem `07-security/08-ip-filtering.md`). Quyết định
theo từng listener xem PROXY protocol có được kỳ vọng hay không; đừng
chấp nhận nó vô điều kiện trên một listener công khai.

## Practice

1. Gửi thủ công một dòng PROXY v1 thô bằng `nc` phía trước một test
   server và xác nhận server có thể parse ra địa chỉ client gốc từ đó.
2. Implement phát hiện/parse v1 trong đường accept-kết nối của `proxy`,
   expose IP client thật cho phần còn lại của request pipeline (logging,
   rate limiting, WAF).
3. Thêm hỗ trợ v2 (nhị phân) và test cả hai format trên cùng một
   listener.
4. Thêm một kiểm tra trusted-source: chỉ tôn trọng một header PROXY nếu
   IP của peer đang kết nối nằm trong một danh sách cho phép (gắn với
   `07-security/08-ip-filtering.md`).
5. Giải thích vì sao một client không bao giờ nên có khả năng gửi trực
   tiếp một header PROXY protocol và được tin tưởng.
