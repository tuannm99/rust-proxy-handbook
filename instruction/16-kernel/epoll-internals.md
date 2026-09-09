# epoll Internals

`02-linux/epoll.md` covers using epoll from the application side,
including the edge-triggered `EAGAIN` bug. This file covers what's
happening inside the kernel that makes epoll's API shape — and that bug
— make sense.

## What to learn

### Two structures: a tree for registration, a list for readiness
An `epoll` instance (`eventpoll` in kernel source) keeps two separate
data structures: a red-black tree of every watched file descriptor,
keyed by fd, so `epoll_ctl(ADD/MOD/DEL)` is O(log n); and a **ready
list** — a plain linked list of just the fds that currently have an
event pending. `epoll_wait` does nothing but drain the ready list: its
cost is proportional to the number of *ready* fds, not the number of
*watched* fds. This is the entire reason epoll scales where `select`/
`poll` don't — those rescan every watched fd on every call, O(n)
regardless of how many are actually ready.

### The wakeup path
When a NIC receives data for a socket, the driver's interrupt handler
hands off to a softirq (`16-kernel/interrupt.md`) that walks the network
stack up to the socket layer. The socket has a wait queue of callbacks
registered by anything currently waiting on it; epoll registers one such
callback when you add the fd. That callback's job is exactly one thing:
add this fd's entry to the ready list, and if a task is blocked in
`epoll_wait`, wake it. Nothing about matching, filtering, or scanning
happens in this path — it's a direct, O(1) push onto the ready list.

### Level-triggered vs edge-triggered, from the ready-list's point of view
**Level-triggered** (the default): an fd stays on (or is re-added to) the
ready list every time `epoll_wait` is called as long as it's still
actually ready (e.g. the socket's receive buffer still has unread bytes).
**Edge-triggered** (`EPOLLET`): the fd is added to the ready list only on
the *transition* into ready — going from "nothing to read" to "something
to read." If you don't drain the socket down to `EAGAIN` on that one
notification, no new transition happens (the socket was already ready,
it stays ready, but nothing re-triggers), and epoll never tells you again
even though unread data is sitting there. This is the exact mechanism
behind the missed-wakeup bug `02-linux/epoll.md`'s exercise has you
reproduce — now in terms of *why* the kernel behaves that way rather
than just observing the symptom.

### `EPOLLEXCLUSIVE` and thundering herd
Multiple threads or processes can share one listening socket and each
register it with their own epoll instance. Without `EPOLLEXCLUSIVE`,
every one of them wakes on the same incoming connection and all race to
`accept()` — all but one get `EAGAIN`, and every wakeup past the first
was wasted CPU. `EPOLLEXCLUSIVE` (Linux 4.5+) tells the kernel to wake
only one waiter per event, which matters once a proxy is running
multiple accept loops (one per worker thread, or `SO_REUSEPORT` with
multiple listening sockets) on the same address.

## Practice
1. Read `/proc/<pid>/fdinfo/<epfd>` for a running `labs/00-tcp-server`
   to see the registered fds and their event masks — confirm it matches
   what your code actually registered.
2. Reproduce the edge-triggered missed-wakeup bug from `02-linux/epoll.md`
   again, but this time explain the fix in terms of the ready-list
   mechanics above: why draining to `EAGAIN` is what generates the next
   transition rather than just "the documented right thing to do."
3. Run multiple accept-loop threads against one `SO_REUSEPORT` listener
   without `EPOLLEXCLUSIVE`, and count wasted `EAGAIN` wakeups under a
   burst of incoming connections; add `EPOLLEXCLUSIVE` and compare.
4. Trace (with `strace -e epoll_wait,epoll_ctl` or similar) a running
   `tokio` program under `labs/00-tcp-server` and match what you see
   against the tree/ready-list model above.
