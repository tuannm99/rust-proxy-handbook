// Lab: a minimal single-threaded async executor + waker, no tokio.
// See instruction/03-rust/async.md, instruction/03-rust/pin.md, instruction/04-runtime/waker.md.
//
// TODO:
// - implement a Task type that holds a Pin<Box<dyn Future<Output = ()>>>
// - implement a Waker (via RawWaker/RawWakerVTable or std::task::Wake)
// - implement a run-queue + executor that polls tasks when woken
// - drive a hand-written Future (e.g. a timer or a channel) to completion

fn main() {
    todo!("implement the mini async runtime");
}
