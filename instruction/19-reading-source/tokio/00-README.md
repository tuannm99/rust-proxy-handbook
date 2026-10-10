# tokio

The async runtime underneath everything in this workspace. Follow
[`reading-guide.md`](reading-guide.md) for a route through the source, keyed to which lab each
stop should come after; the notes files listed below are yours to write
while following it.

One of the two early-reading exceptions to this folder's "read after
[`proxy/`](../../../proxy)" default: read `tokio::runtime::io`'s reactor right after the
raw-epoll exercise ([`02-linux/14-epoll.md`](../../02-linux/14-epoll.md), Practice step 6), while your own
`epoll_create1`/`ctl`/`wait` loop is still fresh — that is the comparison
that makes it legible. The rest (scheduler, task system, waker plumbing)
pairs with [`04-runtime/01-tokio.md`](../../04-runtime/01-tokio.md) and [`04-runtime/02-waker.md`](../../04-runtime/02-waker.md).

Planned files (see [`19-reading-source/00-README.md`](../00-README.md) for the template):

- `architecture.md`
- `request-flow.md`
- `memory.md`
- `interesting-code.md`
- `what-to-learn.md`
