// REVIEW(medium): src/main.rs đã bị xoá (git báo D). README "Run" vẫn ghi
// `cargo run -p http-parser`, và không còn entry point nào để chạy. Bước 5
// (TcpListener) cũng sẽ cần main.rs. Cân nhắc khôi phục stub main() để giữ
// `cargo check --workspace` và lệnh Run hoạt động — xem CLAUDE.md về scaffolding.

// === Bước 1: định nghĩa kiểu dữ liệu (chưa cần I/O) ===
// - enum ParseError { ... }        // mỗi lỗi trong README "Done when" là 1 variant riêng
// - struct Request<'a> { ... }     // method/target/version/headers borrow từ buffer input
// - enum ParseStatus<'a> { Complete { request: Request<'a>, consumed: usize }, Incomplete }

// === Bước 2: hàm parse thuần, không network ===
// fn parse_request(buf: &[u8]) -> Result<ParseStatus, ParseError>
// - tìm request-line trước (method SP target SP version CRLF)
// - rồi header section, dừng khi gặp CRLF CRLF
// - nếu chưa tìm thấy CRLF CRLF -> Incomplete, không phải lỗi

// === Bước 3: test trước khi viết thân hàm (TDD) ===
// #[cfg(test)] mod tests { ... }
// - test 1 request hợp lệ tối thiểu -> Complete đúng field
// - test buffer bị cắt giữa chừng -> Incomplete
// - test tại MỌI split offset của cùng 1 request hợp lệ -> ra kết quả giống nhau (split-point test)

// === Bước 4: body framing (sau khi bước 2-3 pass) ===
// - Content-Length: đủ byte body trong buffer chưa?
// - Transfer-Encoding: chunked: vòng lặp chunk-size;ext CRLF data CRLF, tới size 0 + trailer + CRLF
// - cả hai cùng lúc / Content-Length không hợp lệ -> ParseError, không phải panic

// === Bước 5 (sau cùng, không phải lúc bắt đầu): nối vào TcpListener ===
// - đọc vào buffer tích lũy, gọi lại parse_request mỗi lần có thêm byte
// - đủ rồi thì xử lý request, drain `consumed` byte khỏi buffer (pipelining)

const CRLF: &[u8; 2] = b"\r\n";
const SP: u8 = 0x20; // ' ' (space)
const HTAB: u8 = 0x09; // \t

#[derive(Debug, PartialEq, Default)]
enum Method {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Options,
}

impl Method {
    fn parse(s: &[u8]) -> Result<Self, ParseError> {
        match s {
            b"GET" => Ok(Self::Get),
            b"POST" => Ok(Self::Post),
            b"PUT" => Ok(Self::Put),
            b"PATCH" => Ok(Self::Patch),
            b"DELETE" => Ok(Self::Delete),
            b"OPTIONS" => Ok(Self::Options),
            _ => Err(ParseError::BadRequest),
        }
    }
}

#[derive(Debug, PartialEq, Default)]
enum Version {
    #[default]
    Http10,
    Http11,
}

impl Version {
    fn parse(s: &[u8]) -> Result<Self, ParseError> {
        match s {
            b"HTTP/1.0" => Ok(Self::Http10),
            b"HTTP/1.1" => Ok(Self::Http11),
            _ => Err(ParseError::HttpVersionNotSupported),
        }
    }
}

#[derive(Debug)]
enum ParseError {
    // 4xx
    BadRequest,                  // 400
    UriTooLong,                  // 414
    RequestHeaderFieldsTooLarge, // 431
    ContentTooLarge,             // 413

    // 5xx
    InternalServerError,     // 500
    NotImplemented,          // 501
    HttpVersionNotSupported, // 505
}

impl ParseError {
    fn to_code(&self) -> u16 {
        match self {
            ParseError::BadRequest => 400,
            ParseError::UriTooLong => 414,
            ParseError::RequestHeaderFieldsTooLarge => 431,
            ParseError::ContentTooLarge => 413,

            ParseError::InternalServerError => 500,
            ParseError::NotImplemented => 501,
            ParseError::HttpVersionNotSupported => 505,
        }
    }
}

type RequestHeader<'a> = Vec<(&'a [u8], &'a [u8])>;

#[derive(Debug, PartialEq, Default)]
struct Request<'a> {
    method: Method, // &'a [u8],
    target: &'a [u8],
    // REVIEW(medium): đã chuyển sang Vec nên duplicate không còn bị đè, nhưng
    // lookup theo tên (ví dụ tìm mọi Content-Length để so sánh) phải so khớp
    // KHÔNG phân biệt hoa/thường (RFC 9110 §5.1). Hiện chưa có hàm lookup nào
    // nên chưa lộ bug, nhưng đây là chỗ "content-length" vs "Content-Length"
    // sẽ bị bỏ sót khi bắt đầu xử lý framing — see instruction/07-security/05-request-smuggling.md.
    headers: RequestHeader<'a>,
    version: Version,
}

#[derive(Debug, PartialEq)]
enum ParseStatus<'a> {
    Complete {
        // SHOULD READ - RFC 9112 message syntax
        request: Request<'a>,
        // Số byte đầu tiên của buf tạo thành 1 request hoàn chỉnh. tính thừ
        //  buf = "GET / HTTP/1.1\r\nHost: a\r\n\r\nGET /b HTTP/1.1\r\nHost: a\r\n\r\n"
        //                                         ^ consumed here
        consumed: usize,
    },
    Partial,
}

// REVIEW(low): `parse_request` là `pub` nhưng `ParseStatus` và `ParseError` là
// private, nên compiler cảnh báo "more private". Chọn một: hoặc đưa các kiểu đó
// lên `pub`, hoặc bỏ `pub` ở hàm (lab này chưa cần export).
pub fn parse_request(buf: &[u8]) -> Result<ParseStatus<'_>, ParseError> {
    let mut request = Request::default();

    match parse_request_line(buf)? {
        RequestLineStatus::Partial => Ok(ParseStatus::Partial),
        RequestLineStatus::Complete {
            method,
            target,
            version,
            consumed: request_line_consumed,
        } => {
            let start_from_header = &buf[request_line_consumed..];

            match parse_headers(start_from_header)? {
                HeadersStatus::Partial => Ok(ParseStatus::Partial),
                HeadersStatus::Complete {
                    headers,
                    consumed: header_consumed,
                } => {
                    request.method = method;
                    request.version = version;
                    request.target = target;
                    request.headers = headers;

                    // REVIEW(medium): trả Complete ngay khi xong header, bất kể có
                    // Content-Length hay Transfer-Encoding. Với request có body, bạn
                    // đang báo "xong" trước khi body tới, và `consumed` không tính
                    // body. Bước 4 (body framing) phải nằm TRƯỚC điểm trả Complete
                    // này. Xem lại instruction/01-network/16-http1-wire-format.md.
                    Ok(ParseStatus::Complete {
                        request,
                        consumed: header_consumed + request_line_consumed,
                    })
                }
            }
        }
    }
}

fn is_tchar(b: u8) -> bool {
    // RFC 9110 §5.6.2
    // tchar = DIGIT / ALPHA / "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'!' | b'#'..=b'\'' | b'*'..=b'+'
                | b'-'..=b'.' | b'^'..=b'`'
                | b'|' | b'~'
        )
}

enum RequestLineStatus<'a> {
    Partial,
    Complete {
        method: Method,
        target: &'a [u8],
        version: Version,
        consumed: usize,
    },
}

// REVIEW(medium): chưa quyết định xử lý bare LF (dòng kết thúc bằng LF không có
// CR). Tìm CRLF đầu tiên trong cả buf nên với input `GET / HTTP/1.1\nX\r\n` thì
// request-line bị lấy quá dài và lỗi chỉ tình cờ được bắt ở bước khác. README
// yêu cầu bạn ghi lại quyết định (accept hay reject) cùng lý do — see
// instruction/07-security/05-request-smuggling.md.
fn parse_request_line(buf: &[u8]) -> Result<RequestLineStatus, ParseError> {
    let len_crlf = CRLF.len();

    match buf.windows(len_crlf).position(|window| window == CRLF) {
        None => Ok(RequestLineStatus::Partial),
        Some(idx) => {
            let mut parts = buf[0..idx].split(|&b| b == SP);

            let method = parts.next().ok_or(ParseError::BadRequest)?;
            // REVIEW(medium): target không được kiểm tra rỗng. Input `GET  HTTP/1.1`
            // (hai dấu SP liên tiếp) cho target là chuỗi rỗng và vẫn ra Complete.
            // Request-target phải có ít nhất một byte — xem RFC 9112 §3.2.
            let target = parts.next().ok_or(ParseError::BadRequest)?;
            // REVIEW(low): Version::parse trả 505 cho mọi chuỗi không khớp, kể cả
            // chuỗi sai cú pháp như `HTTP/x.y`. 505 chỉ dành cho version đúng cú
            // pháp nhưng không hỗ trợ (ví dụ HTTP/2.0). Cú pháp sai thuộc 400 — xem
            // RFC 9112 §2.3 về HTTP-version.
            let version = parts.next().ok_or(ParseError::BadRequest)?;

            if parts.next().is_some() {
                return Err(ParseError::BadRequest);
            }

            Ok(RequestLineStatus::Complete {
                method: Method::parse(method)?,
                target,
                version: Version::parse(version)?,
                consumed: idx + CRLF.len(), // start header
            })
        }
    }
}

enum HeadersStatus<'a> {
    Partial,
    Complete {
        headers: RequestHeader<'a>,
        consumed: usize,
    },
}

// REVIEW(medium): chưa có giới hạn kích thước header section hay số lượng
// header. Vòng lặp chạy tới khi gặp dòng trống, còn Vec headers thì lớn vô hạn,
// nên một client gửi header không bao giờ kết thúc sẽ làm buffer và Vec phình
// mãi. README "Done when" yêu cầu từ chối (431) khi vượt giới hạn.
// REVIEW(medium): chưa validate từng dòng header. Tên phải không rỗng và chỉ
// gồm tchar (hàm is_tchar đã viết nhưng chưa được dùng). Không được có
// khoảng trắng giữa tên và dấu `:` (RFC 9112 §5.1, lỗi này là đường vào của
// request smuggling). Value phải bỏ OWS (SP/HTAB) ở hai đầu. Bạn cần quyết
// định xử lý obs-fold (dòng bắt đầu bằng SP/HTAB) — see instruction/07-security/05-request-smuggling.md.
fn parse_headers(start_header: &[u8]) -> Result<HeadersStatus<'_>, ParseError> {
    let mut headers = Vec::new();
    let mut offset = 0;

    loop {
        let remaining = &start_header[offset..];

        let idx = match remaining
            .windows(CRLF.len())
            .position(|window| window == CRLF)
        {
            Some(idx) => idx,
            None => return Ok(HeadersStatus::Partial),
        };

        // Dòng trống đánh dấu kết thúc headers.
        if idx == 0 {
            return Ok(HeadersStatus::Complete {
                headers,
                consumed: offset + CRLF.len(),
            });
        }

        let line = &remaining[..idx];

        let colon_idx = line
            .iter()
            .position(|&b| b == b':')
            .ok_or(ParseError::BadRequest)?;

        let name = &line[..colon_idx];
        let value = &line[colon_idx + 1..];

        if name.is_empty() {
            return Err(ParseError::BadRequest);
        }

        let value = trim_ows(value);

        headers.push((name, value));

        offset += idx + CRLF.len();
    }
}

// REVIEW(nit): `trim_ows` viết literal `b'\t'` trong khi đã có const HTAB. Chọn
// một cách để cùng một ký tự không có hai nguồn định nghĩa.
fn trim_ows(mut value: &[u8]) -> &[u8] {
    while value.first().is_some_and(|&b| b == b' ' || b == b'\t') {
        value = &value[1..];
    }

    while value.last().is_some_and(|&b| b == b' ' || b == b'\t') {
        value = &value[..value.len() - 1];
    }

    value
}

#[cfg(test)]
mod tests {
    use crate::{is_tchar, parse_request, ParseStatus};
    use proptest::prelude::*;

    // REVIEW(low): assert `is_ok()` / `is_err()` không kiểm tra field nào
    // (method/target/version/headers) — một parse sai nhưng không lỗi vẫn pass.
    // Cần assert cụ thể kết quả Complete, và thêm case Partial (README Done when).
    #[test]
    fn test_parse_request() {
        let _req: &str = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert!(parse_request(_req.as_bytes()).is_ok());

        let _req: &str = " / HTTP/1.1\r\nHost: localhost\r\n";
        assert!(parse_request(_req.as_bytes()).is_err());
    }

    // REVIEW(medium): test này pass nhưng che lỗi body. Các offset cố định 5 và 20
    // không cắt đúng ngay sau header (offset 54 trong request này). Tại đó parser
    // hiện trả Complete trong khi body "mydata" còn thiếu, và test sẽ fail nếu
    // thêm offset đó. Expected cũng đang khoá consumed = 54 (không tính body), nên
    // khi sửa body bạn phải cập nhật expected. Thử mọi offset theo README.
    #[test]
    fn test_parse_request_rt() {
        let _full_req: &str =
            "GET / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 6\r\n\r\nmydata";

        let expected = parse_request(_full_req.as_bytes()).unwrap();

        let pieces = [
            &_full_req.as_bytes()[..5],
            &_full_req.as_bytes()[5..20],
            &_full_req.as_bytes()[20..],
        ];

        let mut acc: Vec<u8> = Vec::new();
        for (i, piece) in pieces.iter().enumerate() {
            acc.extend_from_slice(piece);

            let status = parse_request(&acc).unwrap();

            if i < pieces.len() - 1 {
                assert!(matches!(status, ParseStatus::Partial));
            } else {
                assert_eq!(status, expected);
            }
        }
    }

    fn expected_tchar(b: u8) -> bool {
        matches!(
            b,
            b'0'..=b'9'
                | b'A'..=b'Z'
                | b'a'..=b'z'
                | b'!'
                | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
    }

    proptest! {
        #[test]
        fn test_is_tchar_never_panics(b in any::<u8>()) {
            assert_eq!(is_tchar(b), expected_tchar(b));
        }
    }
}
