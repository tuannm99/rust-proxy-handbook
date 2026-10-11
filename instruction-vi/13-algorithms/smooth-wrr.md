# Smooth Weighted Round Robin

Thuật toán đứng sau directive `weight=` của nginx. [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)
giới thiệu WRR và nói rằng các lượt chọn nên "đan xen thay vì dồn cục" —
file này nói về cách sự đan xen đó thực sự được tạo ra.

## What to learn

### Vì sao WRR ngây thơ chưa đủ tốt
Weighted round robin hiển nhiên nhất mở rộng các weight thành một danh
sách (`[a,a,a,b]` cho weight 3 và 1) rồi lặp vòng qua nó. Cách này thỏa mãn
tỷ lệ *trung bình* nhưng tạo ra một chuỗi dồn cục: `a,a,a,b,a,a,a,b`. Ba
request liên tiếp trúng `a` trước khi `b` được một lượt nào, nên số connection của `a` tăng vọt rồi hồi phục theo chu kỳ 4 request. Với weight lớn
hơn (`weight=100` so với `weight=1`), cú dồn cục dài tới 100 request — đủ
dài để ảnh hưởng tail latency và các metric kiểu least-connection được
quan sát bởi bất cứ thứ gì ở downstream.

Smooth WRR tạo ra `a,a,b,a` với cùng weight đó: cùng tỷ lệ 3:1, nhưng `b`
xuất hiện sớm nhất có thể theo weight của nó cho phép.

### Thuật toán current-weight
Mỗi upstream mang hai con số: một `weight` tĩnh (từ config) và một
`current_weight` có thể thay đổi. Ở mỗi lượt chọn:

1. Cộng `weight` của từng upstream vào `current_weight` của nó.
2. Chọn upstream có `current_weight` lớn nhất.
3. Trừ `total_weight` (tổng tất cả weight tĩnh) khỏi `current_weight` của
   upstream vừa được chọn.

```rust
struct WeightedUpstream {
    weight: i64,          // tĩnh, từ config
    current_weight: i64,  // trạng thái chọn có thể thay đổi
}
```

Bước 3 chính là toàn bộ mẹo: người thắng bị đẩy xuống rất âm và phải leo
lại qua nhiều vòng, trong lúc đó các peer weight thấp hơn được đến lượt.
Chuỗi này có thể chứng minh là tuần hoàn với chu kỳ `total_weight`, và
trong một chu kỳ mỗi upstream được chọn đúng `weight` lần — bạn có được cả
tỷ lệ chính xác *lẫn* khoảng cách đều, mà không cần lưu một danh sách mở
rộng nào.

Gotcha: `current_weight` phải có dấu. Chọn một upstream weight-1 từ một
pool có `total_weight = 100` đẩy giá trị của nó xuống -99; clamp về 0 (hay
dùng kiểu unsigned) sẽ phá vỡ tỷ lệ, vì upstream đó không còn phải "trả
nợ" lượt của mình trước khi được chọn lại.

### Effective weight và phản ứng passive với lỗi
nginx mang theo một con số thứ ba, `effective_weight`, chính là cái thực
sự được cộng ở bước 1. Khi một request tới một upstream thất bại, nó giảm
`effective_weight` của upstream đó; khi thành công, nó tăng trở lại, giới
hạn ở `weight` đã cấu hình. Kết quả là một load balancer dần dần giảm
traffic khỏi một upstream đang xuống cấp và dần dần khôi phục lại — mà
không cần bất kỳ active health check nào ([`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md))
kích hoạt.

Đây là health checking *passive* được biểu diễn thuần túy bằng số học
weight, và nó kết hợp được: vòng lặp chọn của smooth-WRR không đổi, nó chỉ
đọc một con số trôi theo tỷ lệ thành công quan sát được.

### Chi phí và concurrency
Selection tốn O(N) trên pool cho mỗi request, với N = số lượng upstream —
ổn với trường hợp vài chục upstream mà một proxy đơn thường có, và trên
thực tế rẻ hơn vẻ ngoài của nó vì state là một mảng liền nhỏ. Đây không
phải lock-free: toàn bộ thao tác pick-and-update phải atomic đối với các
bộ chọn khác, nếu không hai request đồng thời đọc cùng một
`current_weight` và cả hai chọn cùng một upstream.

Gotcha: một `Mutex` bọc quanh pool ở mỗi lượt chọn trở thành điểm tranh
chấp trung tâm của proxy ở tốc độ request cao — mọi request serialize trên
nó. Các cách sửa thường gặp là shard load balancer theo từng worker thread
(mỗi thread giữ mảng `current_weight` riêng, chấp nhận độ chính xác tỷ lệ
theo từng thread thay vì toàn cục) hoặc chuyển sang một thuật toán không
có shared mutable state nào cả, như rendezvous hashing
([`13-algorithms/rendezvous-hash.md`](rendezvous-hash.md)).

## Practice
1. Implement smooth WRR trong [`labs/06-load-balancer`](../../labs/06-load-balancer) cho weight `[5,1,1]`
   và in ra 21 lượt chọn đầu tiên (ba chu kỳ đầy đủ). Xác nhận mỗi chu kỳ
   chứa đúng 5/1/1 lượt chọn và các upstream weight-1 được rải ra thay vì
   nằm cạnh nhau.
2. So sánh nó với WRR danh sách mở rộng ngây thơ trên cùng weight: in cả
   hai chuỗi song song và xác định chuỗi lặp liên tiếp dài nhất ở mỗi bên.
3. Đổi `current_weight` sang kiểu unsigned (hoặc clamp về 0) và chạy lại —
   quan sát tỷ lệ bị phá vỡ, và viết ra vì sao trong một câu.
4. Thêm `effective_weight`: đánh dấu một upstream fail 50% request, giảm/
   tăng khi fail/thành công, và vẽ đồ thị tỷ lệ traffic nó nhận qua 1000
   request khi nó xuống cấp rồi hồi phục.
5. Benchmark selection dưới 8 task đồng thời với một `Mutex` duy nhất bọc
   quanh pool, rồi với state được shard theo từng task; so sánh throughput
   và độ chính xác tỷ lệ toàn cục kết quả.
