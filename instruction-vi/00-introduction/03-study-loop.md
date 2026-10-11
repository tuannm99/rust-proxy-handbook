# Vòng lặp học

File này nói về cách làm các lab để handbook thực sự biến thành kỹ năng, thực tế sẽ mất bao lâu, và việc
contribute open source nằm ở đâu. Hãy đọc nó một lần trước [`labs/00-tcp-server`](../../labs/00-tcp-server), và đọc
lại mỗi khi bạn nhận ra mình đã đọc cả tuần mà chưa viết dòng code nào.

## What to learn

### Vòng lặp, áp dụng cho mỗi lab
Mọi lab đi theo cùng sáu bước. Bỏ qua bất kỳ bước nào là cách phổ biến nhất khiến việc tự học bị chững lại.

1. **Đọc** các file handbook mà README của lab liệt kê, và chỉ những file đó. Cố gắng đừng đọc trước.
2. **Diễn đạt lại spec** bằng lời của bạn. Đọc lại checklist `## Done when` của lab và viết ra, trước khi code, bạn sẽ
   *chứng minh* từng mục bằng cách nào (một lệnh `curl`, một load test, một test case).
3. **Tự build.** Không copy từ internet, từ pingora, hay từ AI. Tra API thì được. Tra thiết kế thì không.
4. **Chứng minh**: chạy các bước chứng minh bạn đã viết ở bước 2. Một mục chưa xong chỉ vì code trông đúng. Nó chỉ xong
   khi bạn tận mắt thấy nó pass.
5. **Được review.** Đưa code cho Claude trong repo này. [`CLAUDE.md`](../../CLAUDE.md) yêu cầu nó làm reviewer nghiêm
   khắc, không đưa sẵn lời giải. Sửa những gì nó tìm ra, rồi hỏi lại. Đây là thứ thay thế cho người kỹ sư senior mà
   việc tự học thường thiếu.
6. **So với production.** Đọc phần tương ứng trong source thật theo các guide ở
   [`19-reading-source/`](../19-reading-source), và ghi lại một điều họ làm mà bạn không làm, kèm lý do.

Gotcha: bước 5 chỉ có tác dụng nếu bạn xin review, không xin lời giải. Hỏi "vì sao chỗ này sai?" và "mình đang bỏ sót
edge case nào?" thì bạn học được. Hỏi "viết giúp mình cái này" thì bạn có một lab mà tuần sau bạn không tự làm lại được.

### Khi bị kẹt
Bị kẹt là chuyện bình thường, và nó cho bạn thông tin. Hãy làm theo quy trình này:

- **Sau 30 phút:** thu nhỏ vấn đề. Viết chương trình nhỏ nhất vẫn cho thấy hành vi gây bối rối. Một nửa số lần, việc
  dựng bản tái hiện đã trả lời được câu hỏi.
- **Vẫn kẹt:** thu thập bằng chứng bằng các công cụ trong [`12-testing/05-debugging.md`](../12-testing/05-debugging.md)
  (một log `tracing`, một `tcpdump`, một `strace`) thay vì đọc lại code.
- **Vẫn kẹt:** xin một *gợi ý* ở mức khái niệm (ví dụ "phần handbook nào giải thích vì sao future này không `Send`?"),
  đừng xin cách sửa.
- **Kẹt ở cùng một khái niệm qua hai lab:** lùi lại một thư mục. Lỗ hổng nằm ở phần trước lab.

### Vì sao bạn quên, và cách sửa
Đọc có cảm giác như đang học. Nhưng một sự thật chỉ đọc một lần phần lớn sẽ biến mất sau hai ngày. Thứ ở lại là thứ
bạn *tự kéo ra từ đầu mình* sau đó. Vì vậy vòng lặp học cần thêm một lớp giữ kiến thức, đặc biệt cho networking và OS,
nơi các sự thật rất nhiều và đan vào nhau:

- **Retrieval, không phải đọc lại.** Sau mỗi file, gập nó lại và viết câu trả lời cho "cái này giải quyết vấn đề gì,
  và thiếu nó thì cái gì hỏng?" Rồi mới kiểm tra.
  [`01-network/22-recall-and-review.md`](../01-network/22-recall-and-review.md) và
  [`02-linux/22-recall-and-review.md`](../02-linux/22-recall-and-review.md) đưa sẵn câu hỏi cho bạn.
- **Giãn cách.** Ôn lại cùng các câu hỏi sau 1, 3, 7 và 21 ngày, rồi hàng tháng. Ngắn mà lặp lại thắng dài mà chỉ một
  lần. Hãy ghi ngày vào log của bạn.
- **Suy ra trước khi tra.** Khi không nhớ ra một điều gì, hãy suy luận từ mô hình layer, hoặc từ bản đồ tài nguyên của
  OS, trước. Hiểu *vì sao* là thứ làm cho một sự thật có thể tự dựng lại được, thay vì học vẹt.
- **Làm cho nó hữu hình.** Một packet bạn đã bắt, một syscall bạn đã `strace`, hay một limit bạn tự chạm vào sẽ được
  nhớ lâu hơn nhiều so với một đoạn văn. `Done when` của mỗi lab cũng là một công cụ trợ nhớ.
- **Dạy lại.** Giải thích một chủ đề thành tiếng, dù không có ai nghe. Chỗ nào bạn bị kẹt chính là lỗ hổng cần sửa.
- **Giữ một error log.** Ghi lại mỗi câu hỏi bạn đã sai hai lần. Chỉ những câu đó mới đáng làm flashcard.

Gotcha: cảm giác "à đúng rồi, tôi nhớ cái đó" khi bạn đọc một câu trả lời là *recognition* (nhận ra), không phải
*recall* (nhớ lại). Nếu bạn không tự tạo ra câu trả lời trước, hãy tính là bạn đã sai.

### Một timeline thực tế
Với khoảng mười giờ mỗi tuần, bắt đầu từ những lỗ hổng lớn:

| Giai đoạn | Nội dung | Thời gian |
| --- | --- | --- |
| 0 | [`00-introduction/02-prerequisites.md`](02-prerequisites.md): Rust từ số 0, track dành cho người mới của networking/OS | 3-4 tháng |
| 1 | `labs/00`-`05`: TCP, parsing, HTTP server, routing, static file, reverse proxy — vòng lặp cốt lõi của proxy | 4-6 tháng |
| 2 | `labs/06`-`12`: load balancing, TLS, HTTP/2, HTTP/3, cache, rate limiting, WAF | 6-9 tháng |
| 3 | `labs/13`-`17`, ghép lại thành [`proxy/`](../../proxy), chạy load và chaos trong [`12-testing/`](../12-testing), đọc pingora | 4-6 tháng |

Tổng cộng khoảng mười tám tháng đến hai năm. Nếu nền Rust của bạn đã vững, phase 0 biến mất và phase 1 rút còn một nửa.
Ai hứa nhanh hơn nhiều khi bạn xuất phát từ số 0 là đang nói về một thứ khác, không phải việc bạn tự build được cái này.

### "Ngon như nginx" có thể và không thể có nghĩa gì
nginx có hai thập kỷ phát triển và triển khai thực tế đằng sau. pingora là nhiều năm chạy production ở Cloudflare. Không
cá nhân nào làm lúc rảnh mà sánh được độ rộng tính năng hay mức độ được kiểm chứng của chúng, và đó cũng không phải mục
tiêu. Mục tiêu khả thi hẹp hơn, và giá trị hơn với bạn: một proxy với tập tính năng tập trung (HTTP/1.1 và HTTP/2, TLS,
load balancing, health check, rate limiting, hot reload, metrics) mà *chạy đúng*, chống được các cuộc tấn công trong
[`07-security/`](../07-security), benchmark cùng bậc với nginx trên cùng phần cứng, và mọi lựa chọn thiết kế của nó bạn
đều bảo vệ được khi đặt cạnh cách nginx và pingora đưa ra cùng lựa chọn đó.

### Contribute open source nằm ở đâu
"Contribute bất kỳ thứ gì" là nói về độ rộng, không phải một trình độ. Kỹ sư Rust senior cũng chuyên sâu theo hướng
riêng, và làm compiler (`rustc`) là một hướng hoàn toàn khác. Mục tiêu thực tế và có giá trị là hệ sinh thái mà dự án này
đứng trên: tokio, hyper, h2, rustls, quinn, pingora. Các mốc đại khái:

- **Sau phase 1:** bạn đọc được issue của tokio và hyper và theo được các thảo luận. Hãy bắt đầu bằng việc tái hiện bug
  report. Xác nhận hoặc khoanh vùng một bug đã là một đóng góp thật.
- **Sau phase 2:** các bản fix nhỏ và cải thiện tài liệu ở những crate bạn dùng nhiều nhất.
- **Sau phase 3:** đóng góp có trọng lượng. Lúc này bạn hiểu domain ngang với nhiều contributor.

[`19-reading-source/contributing-upstream.md`](../19-reading-source/contributing-upstream.md) nói về phần cơ chế.

## Practice
1. Chọn phase bắt đầu một cách trung thực, dựa vào phần tự kiểm tra trong
   [`00-introduction/02-prerequisites.md`](02-prerequisites.md), và ghi ra ngày dự kiến xong phase đó.
2. Bắt đầu một learning log (một file markdown là đủ). Sau mỗi lab, ghi lại: cái gì đã hỏng, review tìm ra gì, và một
   điều source production làm khác bạn. **Done when** log có một mục cho mỗi lab đã xong.
3. Với [`labs/00-tcp-server`](../../labs/00-tcp-server), chạy đủ vòng lặp, kể cả bước 5 (review). **Done when** review
   không còn phát hiện nào mà bạn không đồng ý, và mọi phát hiện còn lại đã được sửa.
4. Mỗi quý, đọc lại timeline của file này và điều chỉnh theo tốc độ thật của bạn, thay vì bỏ kế hoạch khi bị trễ.
5. Sau khi xong mỗi file trong [`01-network/`](../01-network) hoặc [`02-linux/`](../02-linux), hãy lên lịch ôn các câu hỏi
   trong `22-recall-and-review.md` ở +1, +3, +7 và +21 ngày trong learning log của bạn. **Done when** log có các mục ôn
   tập kèm ngày và một điểm số cho mỗi mục.

---

<!-- nav -->
← Trước: [`00-introduction/02-prerequisites.md`](02-prerequisites.md) · [Mục lục](00-README.md)
