# Memory Layout

Thứ tự field của struct, padding, và `#[repr]` — cùng những field đó có
thể chiếm lượng bộ nhớ rất khác nhau, và vì sao điều này vươn tới cả
`13-algorithms/slab.md` và `14-memory/06-fragmentation.md`.

## What to learn

### Alignment buộc phải có padding
Mọi type đều có một alignment; một field phải nằm ở một offset là bội số
của nó. Compiler chèn padding để thỏa mãn điều này, và *thứ tự* field
quyết định bao nhiêu. Một `u8` theo sau bởi một `u64` lãng phí 7 byte
padding để align `u64`; cùng những field đó sắp xếp lại lớn-tới-nhỏ không
lãng phí gì.

```rust
struct Bad  { a: u8, b: u64, c: u8, d: u64 }  // 32 byte (padding)
struct Good { b: u64, d: u64, a: u8, c: u8 }  // 24 byte (đóng gói chặt hơn)
```

### Rust sắp xếp lại theo mặc định — và đó là điều tốt
Khác với C, `repr(Rust)` mặc định của Rust được tự do sắp xếp lại field,
và nó thực sự làm vậy, để tự động giảm thiểu padding. Nên thường bạn
*không* cần tự tay sắp xếp field để tối ưu kích thước — compiler đã làm
rồi. Lý do cần quan tâm là các ngoại lệ: `#[repr(C)]` (cho FFI hoặc một
wire format) đóng băng thứ tự khai báo, và khi đó sự khác biệt
`Bad`/`Good` ở trên quay lại nằm trong tay bạn. Các struct map eBPF trong
`16-kernel/09-ebpf.md` và bất kỳ struct nào bạn memcpy lên đường truyền
đều là `repr(C)` và phải được sắp xếp một cách chủ động.

### Vì sao kích thước vươn tới cả fragmentation và slab
Đây là phần thưởng, không phải một micro-optimization. Allocator làm tròn
lên các size class (`14-memory/06-fragmentation.md`): một struct 129 byte
chiếm một slot 160 byte. Thu nhỏ struct đó xuống dưới 128 — bằng cách bỏ
padding, hoặc bằng hot/cold split bên dưới — chuyển nó sang class 128 và
tiết kiệm 32 byte *mỗi instance*. Ở quy mô 100 nghìn connection đó là một
bước nhảy 3 MB, và nó cũng có nghĩa là nhiều object hơn trên mỗi slab page
(`13-algorithms/slab.md`), tức là mật độ cache tốt hơn trên hot path
(`17-performance/01-cpu-cache.md`). Kích thước là đòn bẩy trên ba hệ thống
con cùng lúc.

### Hot/cold splitting
Một struct theo từng connection thường có một vài field được chạm trên
mỗi request (state, con trỏ buffer) và nhiều field hiếm khi được chạm
(chứng chỉ client gốc, timestamp tạo, các bộ đếm debug). Đóng gói chúng
cùng nhau kéo các field lạnh qua cache trên mỗi lần truy cập. Tách chúng
ra: giữ các field nóng trong một struct nhỏ và box các field lạnh phía
sau một con trỏ.

```rust
struct Conn {
    state: State,          // nóng: chạm mỗi request
    buf: BytesMut,         // nóng
    cold: Box<ConnCold>,   // hiếm khi chạm → một lần gián tiếp, ngoài line nóng
}
```

Đánh đổi là một lần gián tiếp con trỏ để tới dữ liệu lạnh; chỉ đáng giá
khi struct nóng khi đó vừa được nhiều hơn trên mỗi cache line và dữ liệu
lạnh thực sự lạnh.

### Enum và niche
Rust khai thác "niche" — các bit pattern không hợp lệ — để lưu discriminant
của enum miễn phí. `Option<&T>` cùng kích thước với `&T` vì null chính là
niche của `None`; `Option<NonZeroU32>` là 4 byte, không phải 8. Điều này
nghĩa là dùng `NonZero*` và reference thay vì giá trị sentinel (`u32::MAX`
nghĩa là "không có") có thể thu nhỏ một struct mà không cần đổi code chút
nào. Dùng nó trong các cấu trúc arena/index (`13-algorithms/lru.md`,
`15-parser/03-ast.md`) nơi một liên kết "none" là phổ biến.

Gotcha: `#[repr(packed)]` (loại bỏ *toàn bộ* padding) gần như không bao
giờ là câu trả lời — nó tạo ra các field không align, và lấy một reference
tới một field như vậy là undefined behavior, nên nó biến một chiến thắng
về kích thước thành một rủi ro về tính đúng đắn (`03-rust/03-unsafe.md`).
Dùng sắp xếp field và niche `NonZero`, không phải `packed`.

## Practice
1. Dùng `std::mem::size_of` và `#[repr(C)]` để tái tạo khác biệt
   `Bad`/`Good`, rồi bỏ `repr(C)` và xác nhận Rust đã tự đóng gói nó —
   chứng minh bạn hiếm khi cần tự sắp xếp field.
2. In `size_of` cho một struct theo từng connection thật của `proxy`;
   kiểm tra xem nó có nằm ngay sau một ranh giới size-class
   (`14-memory/06-fragmentation.md`) không và liệu thu nhỏ nó có đưa nó
   quay lại dưới ranh giới đó không.
3. Làm một hot/cold split trên struct đó, đo benchmark hot-path
   (`17-performance/01-cpu-cache.md`), và chỉ giữ thay đổi nếu con số đó
   dịch chuyển.
4. Thay một liên kết kiểu "u32::MAX nghĩa là none" trong một cấu trúc
   arena bằng `Option<NonZeroU32>` và xác nhận struct nhỏ đi mà không có
   thay đổi runtime nào.
5. Xác minh mối liên kết với fragmentation: cấp phát 100 nghìn struct đó
   trước và sau khi thu nhỏ nó qua một ranh giới size-class và so sánh
   RSS, nối điều này lại với số object-trên-mỗi-page của
   `13-algorithms/slab.md`.
