# Rolling Restart Without an Orchestrator

Zero-downtime binary upgrades on a single host, with no Kubernetes/systemd-managed rolling deploy to lean on.

## What to learn
### Why this is a different problem than graceful shutdown
`09-architecture/04-graceful-shutdown.md` covers stopping *one* process cleanly. A rolling restart needs a **new** process (new binary, new config) to take over the listening port before the old one goes away — with zero gap where a client's connection attempt gets refused. On a single host with no orchestrator to spin up a second instance behind a load balancer, the proxy itself has to make this handoff safe.

### SO_REUSEPORT: dual-accept during the overlap window
`SO_REUSEPORT` lets multiple processes bind the *same* address:port simultaneously; the kernel load-balances new connections across all of them. Start the new process with `SO_REUSEPORT` set, let it bind alongside the still-running old process, confirm the new one is healthy, then send the old one `SIGTERM` (triggering its normal drain from `04-graceful-shutdown.md`). For the brief overlap, both processes accept new connections — no window where the port is unbound.

```rust
use socket2::{Domain, Socket, Type};
use std::net::SocketAddr;

fn bind_reuseport(addr: SocketAddr) -> std::io::Result<std::net::TcpListener> {
    let socket = Socket::new(Domain::for_address(addr), Type::STREAM, None)?;
    socket.set_reuse_address(true)?;
    socket.set_reuse_port(true)?; // the key difference from a normal bind
    socket.bind(&addr.into())?;
    socket.listen(1024)?;
    Ok(socket.into())
}
```
Gotcha: `SO_REUSEPORT` distributes *new* connections across all bound processes essentially at random — during the overlap window the old (draining) process can still receive brand-new connections unless it's also refusing them at the application layer, which partially defeats the purpose. Keep the overlap window short.

Gotcha, and this is the one that silently costs you connections: the
kernel assigns an incoming connection to a listener **at SYN time**, by
hashing the 4-tuple, and it lands in *that* listener's accept queue. When
the old process closes its listening socket, connections sitting in its
accept queue — already completed handshakes the client believes are
established — are reset, not redistributed. So the draining process must
keep calling `accept()` and serving what's already queued for a moment
after it stops being the preferred target, rather than closing the
listener the instant it decides to drain. This is the same
"stop-accepting is not free" lesson as the pre-stop delay in
`09-architecture/04-graceful-shutdown.md`, one layer down.

### Socket handoff via `exec` (the nginx pattern)
The alternative nginx uses for `SIGUSR2`-triggered binary upgrades: the old master process keeps its listening file descriptor open, spawns the new binary and passes that fd down to it (inherited across `exec`, or handed over a Unix domain socket via `SCM_RIGHTS`), and the new process starts accepting on the *same* socket — not a second one. There is no overlap window and no reliance on `SO_REUSEPORT` at all; only one process ever owns the fd at a time, but it changes hands without ever closing it.

Gotcha: "inherited across `exec`" requires clearing `FD_CLOEXEC` on that
descriptor — Rust sets close-on-exec on sockets it creates by default, and
forgetting to clear it produces a new process that starts, finds no
inherited listener, and either exits or binds a fresh socket with a gap.
Pass the fd number to the child explicitly (an environment variable is the
usual channel) rather than assuming a convention.

Gotcha: nginx's full version of this keeps the old master alive
indefinitely so a failed upgrade can be rolled back by signaling the new
one to exit and the old one to resume accepting. That rollback path is the
actual reason the pattern is more complex than `SO_REUSEPORT` — if you
don't implement rollback, you've paid the complexity without the benefit.

### systemd socket activation
If systemd is available (even without a full orchestrator/Kubernetes), a `.socket` unit can own the listening socket independently of the `.service` unit. Systemd opens and holds the socket; `systemctl restart` on the service just restarts the process, and the kernel queues incoming connections in the socket backlog for the few hundred milliseconds the process is down — no `SO_REUSEPORT` or fd-passing code needed in the proxy at all, at the cost of depending on systemd being present.

Gotcha: this works because the kernel's accept queue absorbs the gap — so
it works only if the gap is shorter than the queue takes to fill. At high
connection rates a slow-starting process (TLS certs to load, config to
validate, caches to build) overflows the backlog and connections are
refused anyway. Measure your startup-to-accepting time and compare it
against your connection rate times your backlog depth
(`16-kernel/03-tcp-stack.md`) before trusting it.

### What must hold true regardless of technique
The new process must pass its own readiness check (config parsed, upstreams reachable — tie to `06-proxy/03-healthcheck.md`) *before* the old one is signaled to drain, or a bad new binary/config takes the whole proxy down instead of just failing to deploy. Long-lived connections (WebSocket, `05-http-stack/09-websocket.md`) held by the old process need the same drain deadline as `04-graceful-shutdown.md` — a rolling restart doesn't make that problem go away, it just adds "and don't refuse new connections while draining."

### The restart loses state, and the state mattered
Zero *dropped connections* is not the same as zero impact, because
everything the old process accumulated in memory is gone. Each of these is
covered elsewhere; together they are why a "successful" zero-downtime
restart can still show up as a spike on every dashboard:
- **The response cache is empty** (`05-http-stack/07-cache.md`). Every entry
  is a miss, all at once — a self-inflicted cache stampede against the
  origin at exactly the moment you'd like things to be calm. Request
  coalescing is what keeps this survivable.
- **Rate limiter buckets reset** (`07-security/07-ratelimit.md`). Every
  client silently receives a fresh budget; a client you were actively
  throttling is unthrottled. An attacker who can trigger restarts gets a
  limit reset on demand.
- **Circuit breakers reset** (`06-proxy/05-retry.md`). The new process
  doesn't know an upstream is broken and will send traffic into it to
  find out, re-learning at the cost of real requests.
- **Connection pools are cold** (`06-proxy/01-upstream.md`). The first
  requests pay handshake latency, including TLS, so p99 spikes for tens of
  seconds after the handoff.
- **Health state is unknown.** Until the first probe cycle completes, the
  new process either treats every upstream as healthy (and sends traffic
  to dead ones) or as unhealthy (and serves nothing) — decide which, and
  prefer making readiness wait for one full probe cycle.

Gotcha: the fix for most of these is to make readiness mean *warm*, not
merely *started* — complete one health-check cycle and pre-open a few
pooled connections before declaring ready. The cache is the exception;
warming it generally isn't worth it, but knowing the miss spike is coming
means not mistaking it for a regression.

### Verifying it honestly
A load test that reports only HTTP status codes will happily declare
success while connections are being reset, because a reset connection
often produces no status code at all — the request simply vanishes from
the result set, or is counted in a category nobody reads. Measure at the
connection level: count connection errors, resets, and refusals
separately from non-2xx responses, and assert all three are zero across
the handover.

## Practice
Build these in order.

1. Implement the `SO_REUSEPORT` bind helper in `proxy` and run two
   instances on one port. **Done when** per-instance logging shows the
   kernel distributing new connections across both.
2. Add a readiness check the new process must pass — config parsed, certs
   loaded, one full health-check cycle completed, a few pooled upstream
   connections opened. **Done when** a process with a broken config or
   unreachable upstreams never reports ready.
3. Write the restart script: start new, wait for readiness, `SIGTERM` the
   old (reusing the drain from `09-architecture/04-graceful-shutdown.md`),
   confirm it exits after draining. **Done when** a bad new binary leaves
   the old one serving, untouched.
4. Make the draining process keep accepting its already-queued connections
   before closing the listener. **Done when** a load test at high
   connection rate through the handover shows zero resets — run it without
   this step first and count them, because they're invisible unless you
   look for them.
5. Load-test through a restart measuring connection-level errors
   separately (`12-testing/01-load-testing.md`). **Done when** connection
   refusals, resets, and non-2xx are all zero across the handover.
6. Measure the state-loss cost. **Done when** you have a chart of origin
   request rate (cache misses), p99 latency (cold pools), and upstream
   error rate (reset circuit breakers) across a restart — and have decided
   which of them you'll mitigate.
7. (Stretch) Implement fd handoff via `SCM_RIGHTS` or `exec` inheritance,
   including clearing `FD_CLOEXEC`. **Done when** the new process accepts
   on the identical socket with no overlap window — verify with
   `ss -tlnp` that only one process owns the listener at any moment.
8. (Stretch) Add the rollback path: if the new process fails readiness
   after taking over, signal it to exit and have the old one resume.
   **Done when** a deliberately broken upgrade rolls back automatically
   with zero dropped connections.
9. Optional: write a systemd `.socket` + `.service` pair and compare.
   **Done when** `systemctl restart` achieves the same zero-dropped result
   with none of the above code — and you've measured your startup time
   against your backlog depth to know the limits of that approach.
