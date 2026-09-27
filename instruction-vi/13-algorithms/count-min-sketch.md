# Count-Min Sketch

Đếm tần suất xấp xỉ trong bộ nhớ cố định. Cách bạn trả lời "IP này đã gửi
bao nhiêu request" cho hàng triệu IP mà không cần một map tăng theo ngân
sách của kẻ tấn công ([`07-security/09-ddos.md`](../07-security/09-ddos.md)).

## What to learn

### Bài toán: đếm chính xác là một bề mặt tấn công
Một `HashMap<IpAddr, u64>` đếm request bị kiểm soát bởi bất kỳ ai gửi
request. Một kẻ tấn công phân tán với một triệu địa chỉ nguồn tạo ra một
triệu entry — bộ đếm bạn thêm vào để *phát hiện* cuộc tấn công trở thành
chính vector làm cạn bộ nhớ hoàn tất nó. Đếm chính xác trên một không gian
key do đối thủ chọn không phải một thiết kế khả thi.

Một count-min sketch cho ra số đếm xấp xỉ trong bộ nhớ bạn chọn trước,
không phụ thuộc vào có bao nhiêu key riêng biệt đến.

### Cấu trúc
Một mảng 2 chiều các bộ đếm, `d` hàng × `w` cột, cộng với `d` hàm hash độc
lập — một cho mỗi hàng.

- **Increment**: với mỗi hàng `i`, tăng `table[i][h_i(key) % w]`.
- **Query**: lấy giá trị **nhỏ nhất** trong `d` hàng.

```rust
struct CountMinSketch {
    table: Vec<u32>, // d * w, được làm phẳng
    width: usize,    // w
    depth: usize,    // d
}
```

Va chạm (collision) chỉ có thể làm phồng một bộ đếm, không bao giờ làm xẹp
nó — nên mỗi hàng cho ra một overestimate, và giá trị nhỏ nhất là bound
chặt nhất có sẵn. Số đếm không bao giờ *thấp* hơn thực tế, đây chính là
tính chất quan trọng cho việc phát hiện: một heavy hitter không bao giờ có
thể trốn được.

### Error bound
Với `w = ceil(e/ε)` và `d = ceil(ln(1/δ))`, ước lượng vượt quá số đếm thật
hơn `ε × N` (N = tổng số lần increment) với xác suất tối đa `δ`. Với
ε=0.001 và δ=0.01, đó là khoảng 2718 × 5 bộ đếm — khoảng 54 KB với 4 byte
mỗi bộ đếm, cho một error bound 0.1% ở bất kỳ cardinality key nào.

Hãy đọc sự đánh đổi này một cách trung thực: bộ nhớ cố định và đã biết,
error scale theo *tổng khối lượng traffic* N, không theo số lượng key.
Dưới một cuộc tấn công theo khối lượng, N cực lớn, nên error tuyệt đối
tăng lên — điều này ổn cho câu hỏi "IP này có phải heavy hitter không"
nhưng vô dụng cho câu hỏi "chính xác IP này đã gửi bao nhiêu request."

Gotcha: `d` hàm hash phải độc lập. Tái sử dụng một hash với các seed khác
nhau chỉ chấp nhận được nếu hash đó thực sự trộn seed vào (SipHash với các
key riêng biệt, xxHash với các seed riêng biệt); `hash(key) + i` không độc
lập và làm sụp đổ đảm bảo về error, bởi vì hai key va chạm ở một hàng thì
sẽ va chạm ở mọi hàng và giá trị nhỏ nhất không còn giúp ích gì nữa.

### Pattern heavy-hitter
Bản thân sketch chỉ cho bạn biết số đếm nếu bạn đã có key. Kết hợp nó với
một cấu trúc top-K nhỏ: ở mỗi request, tăng sketch, query ước lượng, và
nếu nó vượt ngưỡng, insert key vào một min-heap có giới hạn chứa những kẻ
vi phạm tệ nhất. Sketch xử lý cardinality vô hạn trong bộ nhớ cố định;
heap chỉ giữ những key thực sự quan trọng.

Đây cũng là cách TinyLFU ([`13-algorithms/lru.md`](lru.md)) ước lượng tần suất truy
cập cho việc admission vào cache chỉ với vài bit mỗi key.

### Decay: số đếm phải biết quên
Một sketch chỉ tăng dần là đơn điệu — một IP là heavy hitter cách đây một
giờ vẫn bị đánh dấu mãi mãi, và mọi bộ đếm cuối cùng sẽ bão hòa. Hai cách
sửa tiêu chuẩn:
- **Giảm một nửa mỗi bộ đếm** theo chu kỳ (một lượt "conservative aging").
  Rẻ, giữ được thứ tự tương đối, và đây là điều TinyLFU làm.
- **Sliding window**: giữ nhiều sketch, mỗi cái một khoảng thời gian, xoay
  vòng và zero cái cũ nhất. Tốn `d × w × số bucket` bộ nhớ nhưng cho ra số
  đếm theo cửa sổ thật, đây là thứ mà một rate limiter cần.

Gotcha: bộ đếm `u32` bão hòa dưới một cuộc tấn công kéo dài. Dùng phép
toán saturating, và size khoảng thời gian decay sao cho bộ đếm không thể
chạm trần giữa các lượt decay — một bộ đếm bị wrap đọc ra như traffic
*thấp*, đảo ngược đúng lúc bạn đang bị tấn công.

### Conservative update
Một cải tiến đáng biết: khi increment, chỉ cập nhật các hàng mà bộ đếm
bằng đúng giá trị nhỏ nhất hiện tại, để yên các hàng đã bị phồng. Điều này
giảm overestimation một cách đo lường được mà không tốn thêm bộ nhớ, và
chỉ ảnh hưởng đường ghi. Nó đánh đổi lấy việc mất khả năng *giảm* giá trị,
nên không tương thích với các scheme sliding-window có phép trừ.

## Practice
1. Implement một count-min sketch và kiểm chứng error bound bằng thực
   nghiệm: cho một luồng key phân phối Zipf chạy qua, so sánh ước lượng
   với số đếm chính xác từ một `HashMap`, và xác nhận overestimate nằm
   trong `ε × N` với ε bạn đã size.
2. Cho thấy failure mode: thay `d` hash độc lập bằng `hash(key) + i` và
   chạy lại bước 1; đo xem error suy giảm bao nhiêu.
3. Trong [`labs/11-rate-limit`](../../labs/11-rate-limit), thêm một bộ phát hiện heavy-hitter toàn cục
   dựa trên sketch cạnh các bucket chính xác theo từng IP. So sánh bộ nhớ
   ở 1 triệu IP nguồn riêng biệt.
4. Thêm việc giảm một nửa theo chu kỳ; xác nhận một key burst-rồi-idle rơi
   xuống dưới ngưỡng trong đúng số lượt decay mong đợi.
5. Cố tình làm bão hòa một bộ đếm `u32` và quan sát bộ phát hiện của bạn
   báo cáo gì; sau đó sửa nó bằng phép toán saturating và một khoảng decay
   giữ bộ đếm trong phạm vi.
6. Implement conservative update và đo mức giảm overestimation trung bình
   so với baseline ở bước 1.
