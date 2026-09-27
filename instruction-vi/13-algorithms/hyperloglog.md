# HyperLogLog

`13-algorithms/count-min-sketch.md` trả lời "key này đã xuất hiện bao
nhiêu lần." HyperLogLog trả lời một câu hỏi khác trong bộ nhớ cố định
cũng nhỏ tương tự: "có bao nhiêu key *riêng biệt* đã xuất hiện" — ước
lượng cardinality, hữu ích cho câu hỏi "có bao nhiêu IP tấn công riêng
biệt đã đánh vào chúng ta trong phút vừa rồi" (`07-security/09-ddos.md`)
mà không bao giờ phải lưu bản thân tập IP.

## What to learn

### Mẹo cốt lõi: số bit-0 dẫn đầu như một tín hiệu về quy mô
Hash mỗi key đến thành một chuỗi bit ngẫu nhiên đồng đều. Vị trí của bit 1
đầu tiên tính từ trái (tương đương, số bit 0 dẫn đầu) trong hash đó, tự
bản thân nó, là một tín hiệu yếu về cardinality: thấy một hash có 10 bit 0
dẫn đầu là một sự kiện `1-trên-1024`, nên việc quan sát được dù chỉ một
lần gợi ý rằng khoảng một nghìn hash riêng biệt đã được thử. Một quan sát
đơn lẻ như vậy quá nhiễu để tin một mình — đóng góp của HyperLogLog là
lấy trung bình tín hiệu này qua nhiều bucket độc lập để kiểm soát phương
sai.

### Cấu trúc
Chia hash thành hai phần: vài bit chọn một trong `m` register (bucket),
phần còn lại được dùng để tính số bit 0 dẫn đầu, và mỗi register lưu giá
trị *lớn nhất* của số bit 0 dẫn đầu từng thấy cho bất kỳ key nào hash vào
nó:

```rust
struct HyperLogLog {
    registers: Vec<u8>, // m register, mỗi cái giữ số bit-0-dẫn-đầu lớn nhất
}
// add(key): (bucket, rest) = split(hash(key));
//           registers[bucket] = registers[bucket].max(leading_zeros(rest))
// estimate(): trung bình điều hòa của 2^registers[i] trên mọi bucket,
//             nhân với một hằng số hiệu chỉnh sai lệch phụ thuộc vào m
```

Với `m = 16384` register (2 KB với mỗi register một byte), sai số chuẩn
khoảng `1.04 / sqrt(m) ≈ 0.8%` — và quan trọng là, bound sai số đó đúng
dù cardinality thật là một nghìn hay một tỷ; bộ nhớ cố định bất kể có bao
nhiêu key riêng biệt thực sự xuất hiện, đây chính xác là tính chất quan
trọng khi đối mặt với một attacker kiểm soát cardinality của key.

### Có thể merge giữa các shard
Hai cấu trúc HyperLogLog trên hai luồng dữ liệu rời nhau có thể được kết
hợp thành ước lượng của hợp của chúng bằng cách lấy **giá trị lớn nhất**
theo từng phần tử giữa các register của chúng — không cần quét lại bất kỳ
tập dữ liệu nào. Đây là lý do các hệ thống phân tán (`PFCOUNT`/`PFMERGE`
của Redis, `APPROX_COUNT_DISTINCT` của BigQuery/Presto) dùng nó: mỗi
shard/worker giữ cấu trúc nhỏ của riêng nó, và một số đếm riêng biệt toàn
cục chỉ là một lần merge theo từng register rẻ tiền, không phải một bài
toán điều phối.

### Gotcha: nó đếm "đã từng thấy," không phải "thấy trong N phút vừa qua"
Một register HyperLogLog chỉ có thể tăng (`max`, không bao giờ giảm), nên
nó trả lời "số phần tử riêng biệt kể từ khi tôi bắt đầu đếm," không phải
một sliding window. Với một tín hiệu DDoS kiểu "số IP tấn công riêng biệt
trong phút vừa qua," kết hợp nó với cùng mẹo decay/rotation mà
`13-algorithms/count-min-sketch.md` dùng: giữ một HLL cho mỗi khoảng thời
gian, merge N bucket gần nhất để có ước lượng theo cửa sổ, và bỏ bucket cũ
nhất khi thời gian trôi qua.

## Practice
1. Implement HyperLogLog cơ bản và kiểm chứng nó so với một `HashSet` làm
   ground truth trên một luồng tổng hợp với số lượng key riêng biệt đã
   biết; xác nhận error nằm trong bound lý thuyết cho `m` bạn đã chọn.
2. Cho cùng một key vào hai lần và xác nhận ước lượng không đổi — chỉ có
   tính riêng biệt là quan trọng, không phải tần suất.
3. Xây hai cấu trúc HyperLogLog trên hai luồng key rời nhau, merge chúng
   bằng max theo từng phần tử, và xác nhận ước lượng đã merge gần với
   cardinality hợp thật.
4. Thêm rotation theo khoảng thời gian (một HLL mỗi phút, merge 5 cái gần
   nhất để có số đếm theo cửa sổ) và dùng nó làm tín hiệu tốc-độ-IP-riêng-biệt
   cạnh việc phát hiện theo khối lượng của `07-security/09-ddos.md`; test
   nó với một cuộc tấn công phân tán giả lập từ nhiều IP nguồn tổng hợp.
