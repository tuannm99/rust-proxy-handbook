# DFA (Deterministic Finite Automata)

[`13-algorithms/regex-engine.md`](regex-engine.md) nói về cách một regex engine dùng DFA bên
dưới. File này nói về bản thân DFA — lý thuyết tổng quát mà bất kỳ state
machine dựa trên bảng nào (một protocol parser, một config lexer) đứng
trên đó.

## What to learn

### State, transition, một lần tra cứu cho mỗi byte
Một DFA là `(states, alphabet, transition_fn, start, accept_states)`,
trong đó `transition_fn(state, byte) -> state` là toàn ánh và tất định:
đúng một next state cho mỗi cặp `(state, byte)`. Việc matching trở thành
một vòng lặp — không backtrack, không nhánh trên nhiều khả năng:

```rust
struct Dfa {
    table: Vec<[usize; 256]>, // table[state][byte] -> next state
    accept: Vec<bool>,
}

fn run(dfa: &Dfa, input: &[u8]) -> bool {
    let mut state = 0;
    for &b in input {
        state = dfa.table[state][b as usize];
    }
    dfa.accept[state]
}
```

Đây là lý do một DFA là cấu trúc matching nhanh nhất hiện có: O(n) với một
hằng số nhỏ, dễ đoán — một lần index mảng cho mỗi byte input, không đệ
quy, không cấp phát.

### DFA đến từ đâu: subset construction
DFA hiếm khi được viết tay; chúng được suy ra từ một NFA (xây bằng
Thompson construction, xem [`13-algorithms/regex-engine.md`](regex-engine.md)) qua **subset
construction**: mỗi state của DFA là *tập hợp* các state NFA có thể tới
được với một tiền tố input nào đó. Số lượng tập con là hàm mũ trong
trường hợp xấu nhất — đây chính xác là hiện tượng bùng nổ bộ nhớ mà phần
lazy-DFA của [`regex-engine.md`](regex-engine.md) mô tả và giải quyết bằng cách xây state
theo yêu cầu; không lặp lại thảo luận đó ở đây, hãy đọc ở file kia.

### Minimization
Nhiều state của DFA giống hệt nhau về hành vi — chúng chấp nhận đúng cùng
một tập input tương lai — và có thể được gộp lại mà không thay đổi những
gì automaton match. Thuật toán Hopcroft tìm và gộp các state này trong
O(n log n), điều này quan trọng cho các DFA parser giao thức được xây tay
(không phải các DFA sinh từ regex) nơi một cách xây ngây thơ tạo ra nhiều
state hơn cần thiết: ít state hơn nghĩa là bảng nhỏ hơn và hành vi cache
tốt hơn trên hot path.

### Đánh đổi bộ nhớ trong một bảng transition đầy đủ
Bảng ở trên có `|states| × 256` entry — nhanh, nhưng lãng phí khi hầu hết
state chỉ quan tâm đến một số ít byte riêng biệt (ví dụ một bộ matcher
HTTP method chỉ rẽ nhánh trên `G`, `P`, `D`, `H`, ...). Một biểu diễn thưa
(một match arm nhỏ hoặc một `HashMap<u8, usize>` cho mỗi state) đánh đổi
một lần gián tiếp lấy ít bộ nhớ hơn nhiều. Chọn bảng dày khi DFA nhỏ và
nóng (một vài state được check trên mỗi byte của mỗi request); chọn thưa
khi nó lớn và hiếm khi được check.

## Practice
1. Tự tay xây DFA (không phải NFA) để match một tập cố định các HTTP
   method (`GET`, `POST`, `PUT`, ...) dưới dạng bảng, và dùng nó làm bước
   phân loại byte đầu tiên trong [`labs/01-http-parser`](../../labs/01-http-parser).
2. Áp dụng subset construction bằng tay lên một NFA nhỏ cho `a(b|c)*d` và
   xác nhận bảng DFA kết quả của bạn cho ra cùng phán quyết
   accept/reject như khi trace tập state NFA (bài tập trong
   [`regex-engine.md`](regex-engine.md)).
3. Tìm hai state tương đương về hành vi trong một DFA bạn đã xây và gộp
   chúng bằng tay; xác nhận automaton đã gộp vẫn chấp nhận cùng một ngôn
   ngữ.
4. So sánh một bảng dày `[usize; 256]` cho mỗi state với một phiên bản
   thưa `HashMap<u8, usize>` trên DFA match method của bạn — đo cả bộ nhớ
   lẫn thời gian tra cứu mỗi byte.
