// Lab: hand-write an HTTP/1.1 request parser from raw bytes, no hyper.
// See instruction/05-http-stack/01-parser.md.
//
// TODO:
// - parse the request line (method, target, version)
// - parse headers into a map, handle folding/duplicates
// - determine body framing: Content-Length vs Transfer-Encoding: chunked
// - reject malformed input (this is also where request smuggling bugs hide,
//   see instruction/07-security/05-request-smuggling.md)

fn main() {
    todo!("implement the raw HTTP parser");
}
