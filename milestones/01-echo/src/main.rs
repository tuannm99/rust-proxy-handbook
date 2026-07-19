// Milestone 1: TCP Echo Server. See instruction/10-projects/project-01.md.
//
// TODO:
// - bind a TcpListener (see instruction/01-network/socket.md)
// - accept connections in a loop
// - spawn a task per connection (see instruction/03-rust/async.md, instruction/04-runtime/tokio.md)
// - read bytes from the socket and write them straight back
// - handle EOF and I/O errors without killing the whole server

#[tokio::main]
async fn main() {
    todo!("implement the echo server");
}
