# DNS

Recursive vs authoritative, TTL, caching.

## What to learn

### Recursive vs authoritative resolution
Một recursive resolver (ví dụ stub resolver của OS bạn, hay một resolver
công cộng như 1.1.1.1) đi qua chuỗi root -> TLD -> authoritative
nameserver thay mặt bạn và cache kết quả. Một authoritative server là
nguồn sự thật cho một zone và chỉ trả lời cho các record nó sở hữu. Một
proxy thường nói chuyện với một recursive resolver, không phải trực tiếp
với authoritative server — nhưng biết chuỗi này quan trọng khi một thay
đổi DNS "không propagate" và bạn cần biết tầng nào đang stale.

### Các loại record quan trọng với một proxy
`A`/`AAAA` (địa chỉ upstream IPv4/IPv6), `CNAME` (aliasing, không thể tồn
tại cùng record khác ở cùng một tên), `SRV` (host+port+priority+weight —
đây là nền tảng cho rất nhiều cơ chế service discovery), `TXT` (dùng cho
ACME DNS-01 challenge khi tự động hóa việc phát hành certificate). Một
proxy resolve hostname upstream thường chỉ quan tâm tới `A`/`AAAA` và đôi
khi `SRV`.

### TTL và caching
TTL báo cho mọi cache phía dưới (OS, resolver, chính proxy của bạn) biết
một record có thể được tái sử dụng bao lâu. Một proxy tự resolve hostname
upstream cần cache riêng của nó, tôn trọng thời hạn TTL — tái sử dụng một
IP đã cũ sau khi địa chỉ của một backend thay đổi âm thầm sẽ gửi traffic
vào hư không. Gotcha trong production: một số recursive resolver hoặc
client library kẹp hoặc bỏ qua TTL rất thấp (dưới 5s), điều này phá vỡ các
deployment dựa trên DNS failover nhanh — đừng giả định một TTL 1 giây thực
sự cho bạn failover 1 giây trong thực tế.

### Resolution trong Rust: blocking vs async
`getaddrinfo` của libc, được `std::net::ToSocketAddrs` dùng, là *blocking*
và tự làm caching/config parsing ở tầng OS (`/etc/resolv.conf`,
`/etc/hosts`) — gọi nó trực tiếp bên trong một async task sẽ làm khựng
executor thread. Các lựa chọn: chạy nó qua `tokio::task::spawn_blocking`,
hoặc dùng một crate resolver thuần async như `hickory-resolver` nói chuyện
DNS trực tiếp qua UDP/TCP và cho bạn caching tôn trọng TTL cùng quyền
kiểm soát nameserver nào bạn query.

```rust
// resolution blocking được chạy ngoài async executor thread
let addrs = tokio::task::spawn_blocking(|| {
    "backend.internal:8080".to_socket_addrs()
})
.await??;
```

### DNS như một cơ chế service-discovery
Nhiều hệ thống (Kubernetes headless Service, giao diện DNS của Consul)
expose service registry của họ *như là* DNS — một `A` record trả về nhiều
IP, hoặc thay đổi theo thời gian khi pod/instance đến và đi. Một proxy
re-resolve định kỳ và hoán đổi tập upstream của nó một cách atomic có được
service discovery động cơ bản gần như miễn phí; xem
`06-proxy/07-service-discovery.md` để biết việc hoán đổi đó cần diễn ra
thế nào mà không làm rớt request đang xử lý dở.

## Practice

1. Dùng `dig +trace example.com` để quan sát thủ công chuỗi root -> TLD ->
   authoritative, rồi so sánh với một lần `dig example.com` (dùng cache
   của một recursive resolver).
2. Viết một chương trình Rust độc lập nhỏ dùng `hickory-resolver` (async)
   resolve một hostname thành nhiều record `A` và in ra TTL của chúng.
3. Trong `labs/05-reverse-proxy`, resolve hostname upstream thay vì
   hardcode IP, và re-resolve theo một timer tôn trọng TTL của record.
4. Mô phỏng một thay đổi IP backend: trỏ một hostname tới IP A, khởi động
   proxy của bạn, rồi đổi DNS sang IP B. Đo xem proxy của bạn mất bao lâu
   để nhận ra, và có request nào thất bại trong lúc chuyển đổi không.
5. Đọc `06-proxy/07-service-discovery.md` và ghi chú phần nào trong đó
   cách tiếp cận dựa trên DNS ở bước 3 của bạn đã thỏa mãn, phần nào vẫn
   còn cần.
