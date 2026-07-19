// Lab: a TCP echo server on raw epoll via libc, no tokio.
// See instruction/02-linux/epoll.md.
//
// TODO:
// - create a non-blocking listening socket by hand (see instruction/01-network/socket.md)
// - create an epoll instance, register the listener with epoll_ctl
// - run the event loop with epoll_wait, dispatch readable/writable events
// - accept new connections and register them; handle EAGAIN correctly
// - compare edge-triggered vs level-triggered behavior

fn main() {
    todo!("implement the raw epoll echo server");
}
