# Arena Allocation

`14-memory/01-allocator.md` nói về cấp phát tổng quát. Arena là chiến lược
ngược lại cho một dạng workload cụ thể — thứ mà một proxy có liên tục:
nhiều allocation nhỏ đều "chết" cùng nhau vào cuối một request.

## What to learn

### Bump allocation: một con trỏ, không free list
Một arena cấp phát bộ nhớ bằng cách đẩy một con trỏ duy nhất tiến lên
trong một khối đã cấp phát sẵn; không có free theo từng object — toàn bộ
khối được giải phóng (hoặc reset và tái sử dụng) như một thao tác duy nhất
khi phạm vi của arena kết thúc:

```rust
// ý tưởng của bumpalo
struct Arena {
    chunk: Vec<u8>,
    offset: usize,
}
impl Arena {
    fn alloc<T>(&mut self, value: T) -> &mut T {
        // đẩy self.offset lên size_of::<T>() (đã align), ghi value vào đó
        // chuyển sang chunk mới nếu chunk hiện tại đầy
        unimplemented!()
    }
}
```
Trong thực tế bạn nên dùng crate `bumpalo` hoặc `typed-arena` thay vì tự
viết — điểm quan trọng là *hình dạng* của cách làm: allocation chỉ là một
phép bump con trỏ (rẻ nhất có thể), và N object mà đáng lẽ là N lần
alloc-và-free riêng của `Box`/`Vec` trở thành một allocation và một lần
giải phóng hàng loạt.

### Phù hợp với: dữ liệu theo phạm vi request
Một HTTP request thường cấp phát nhiều mảnh dữ liệu sống ngắn — header đã
parse, giá trị trung gian của quyết định routing, buffer cho việc biến
đổi — tất cả chỉ cần tồn tại đến khi response được gửi đi. Cấp phát mỗi
mảnh đó vào một arena theo request và drop toàn bộ arena khi request hoàn
tất biến hàng chục lần free riêng lẻ thành một lần. Đây cũng chính xác là
cách sửa mà `14-memory/06-fragmentation.md` khuyến nghị cho nguyên nhân
fragmentation "vòng đời lẫn lộn": dữ liệu theo phạm vi request không bao
giờ có cơ hội đan xen với connection state sống lâu hơn nếu nó nằm trong
arena riêng của nó.

### Gotcha: vòng đời của dữ liệu cấp phát từ arena chính là vòng đời của arena
Giá trị cấp phát từ một arena là reference mượn từ nó — chúng không thể
sống lâu hơn arena mà không được copy ra trước. Trong một async handler,
điều này có nghĩa là arena (hay một reference vào nó) phải sống ít nhất
bằng mọi điểm `.await` chạm vào dữ liệu mượn từ nó — chính xác là kiểu
tình huống self-referential-qua-await-point mà `03-rust/06-pin.md` mô tả.
Cụ thể: đừng cấp phát vào một arena là biến local rồi cố giữ một reference
vào nó qua một future bị suspend sống lâu hơn hàm — hoặc để future sở hữu
arena trong state của nó, hoặc copy dữ liệu ra trước điểm suspend.

### Gotcha: arena không giới hạn theo request là một quả bom bộ nhớ
Một arena tăng trưởng không giới hạn (ví dụ xử lý một request body không
giới hạn do kẻ tấn công kiểm soát thành các mảnh cấp phát trong arena) loại
bỏ backpressure tự nhiên mà một giới hạn theo từng allocation lẽ ra đã
cung cấp. Giới hạn tổng kích thước arena theo mỗi request và từ chối/báo
lỗi khi vượt quá, giống cách bạn giới hạn bất kỳ tài nguyên nào khác theo
request (`07-security/09-ddos.md`).

## Practice
1. Dùng `bumpalo` để arena-allocate header đã parse cho một request trong
   `labs/01-http-parser`, thay thế các allocation `String`/`Vec` theo
   từng header.
2. Đo số lượng allocation và tổng thời gian parse một request có 20
   header, có và không có arena, ở mức đồng thời cao.
3. Cố tình thử trả về một reference mượn từ một arena local trong hàm, xem
   lỗi compile, rồi viết lại để hàm gọi sở hữu arena hoặc copy dữ liệu cần
   thiết ra ngoài.
4. Thêm kích thước arena tối đa cho mỗi request và xác nhận một request có
   header vượt quá giới hạn bị từ chối gọn gàng thay vì làm arena tăng
   trưởng vô hạn.
