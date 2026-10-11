# PROXY Protocol

File này nói về cách một L7 proxy biết địa chỉ thật của client, khi nó không phải hop đầu tiên.

## What to learn

### Vấn đề nó giải quyết
Giả sử proxy của bạn đứng sau một load balancer hay proxy khác (cloud LB, edge CDN, một reverse proxy khác). TCP
connection mà proxy của bạn thấy là đến *từ* thiết bị trung gian đó, không phải từ client gốc. `peer_addr()` trên socket
đã accept cho bạn IP của LB, không phải IP của client.

PROXY protocol giải quyết việc này ở TCP layer, trước khi có bất kỳ HTTP parsing nào. Hop phía trước chèn một header nhỏ
vào đầu connection, mang địa chỉ của client gốc.

### PROXY protocol v1 (dạng văn bản)
Một dòng người đọc được, gửi làm những byte đầu tiên của connection:

```
PROXY TCP4 192.0.2.1 198.51.100.1 56324 443\r\n
```

`PROXY <family> <src-ip> <dst-ip> <src-port> <dst-port>\r\n`. Nó dễ parse và dễ debug, nhưng dài dòng và chỉ hỗ trợ
TCP4/TCP6/UNKNOWN.

### PROXY protocol v2 (nhị phân)
v2 gồm:
- một signature cố định 12 byte,
- một header nhị phân (version/command, address family/protocol, độ dài),
- khối địa chỉ,
- tùy chọn: các TLV mang metadata bổ sung, như SNI/ALPN TLS gốc.

v2 được dùng trong production vì rẻ hơn để parse và không mơ hồ: không cần quét delimiter, chỉ có offset cố định và một
body có tiền tố độ dài.

### Phát hiện và parse nó trước HTTP layer
Proxy phải peek những byte đầu tiên của connection vừa accept *trước khi* đưa nó cho HTTP parser:
1. Kiểm tra signature nhị phân của v2 trước (các byte cố định, không mơ hồ).
2. Rồi mới kiểm tra tiền tố `PROXY ` theo nghĩa đen cho v1.
3. Nếu không khớp, giả định không có PROXY protocol header.

Làm sai chỗ này (ví dụ coi header là một phần của HTTP request body) sẽ làm hỏng mọi request phía sau.

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
Chỉ chấp nhận PROXY protocol header từ các connection mà bạn thực sự tin, tức là dải IP của LB upstream đã biết. Nếu
không, bất kỳ ai tới được listener của bạn đều có thể *giả mạo* địa chỉ client. Cách giả mạo giống như `X-Forwarded-For`
chưa được validate ở HTTP layer (xem [`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md)).

Hãy quyết định theo từng listener: listener này có kỳ vọng PROXY protocol hay không. Đừng chấp nhận nó vô điều kiện trên
một listener công khai.

## Practice

1. Gửi thủ công một dòng PROXY v1 thô bằng `nc` vào một test server, và xác nhận server parse ra được địa chỉ client gốc
   từ đó.
2. Implement phát hiện và parse v1 trong đường accept-connection của [`proxy`](../../proxy). Đưa IP thật của client cho
   phần còn lại của request pipeline (logging, rate limiting, WAF).
3. Thêm hỗ trợ v2 (nhị phân) và test cả hai format trên cùng một listener.
4. Thêm kiểm tra trusted-source: chỉ chấp nhận PROXY header nếu IP của peer đang connect nằm trong một allow list (liên
   quan tới [`07-security/08-ip-filtering.md`](../07-security/08-ip-filtering.md)).
5. Giải thích vì sao không bao giờ nên để một client gửi trực tiếp một PROXY protocol header mà vẫn được tin.
