# Rolling Restart Without an Orchestrator

Zero-downtime binary upgrades on a single host, with no Kubernetes/systemd-managed rolling deploy to lean on.

## What to learn
### Why this is a different problem than graceful shutdown
`09-architecture/graceful-shutdown.md` covers stopping *one* process cleanly. A rolling restart needs a **new** process (new binary, new config) to take over the listening port before the old one goes away — with zero gap where a client's connection attempt gets refused. On a single host with no orchestrator to spin up a second instance behind a load balancer, the proxy itself has to make this handoff safe.

### SO_REUSEPORT: dual-accept during the overlap window
`SO_REUSEPORT` lets multiple processes bind the *same* address:port simultaneously; the kernel load-balances new connections across all of them. Start the new process with `SO_REUSEPORT` set, let it bind alongside the still-running old process, confirm the new one is healthy, then send the old one `SIGTERM` (triggering its normal drain from `graceful-shutdown.md`). For the brief overlap, both processes accept new connections — no window where the port is unbound.

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

### Socket handoff via `exec` (the nginx pattern)
The alternative nginx uses for `SIGUSR2`-triggered binary upgrades: the old master process keeps its listening file descriptor open, spawns the new binary and passes that fd down to it (inherited across `exec`, or handed over a Unix domain socket via `SCM_RIGHTS`), and the new process starts accepting on the *same* socket — not a second one. There is no overlap window and no reliance on `SO_REUSEPORT` at all; only one process ever owns the fd at a time, but it changes hands without ever closing it.

### systemd socket activation
If systemd is available (even without a full orchestrator/Kubernetes), a `.socket` unit can own the listening socket independently of the `.service` unit. Systemd opens and holds the socket; `systemctl restart` on the service just restarts the process, and the kernel queues incoming connections in the socket backlog for the few hundred milliseconds the process is down — no `SO_REUSEPORT` or fd-passing code needed in the proxy at all, at the cost of depending on systemd being present.

### What must hold true regardless of technique
The new process must pass its own readiness check (config parsed, upstreams reachable — tie to `06-proxy/healthcheck.md`) *before* the old one is signaled to drain, or a bad new binary/config takes the whole proxy down instead of just failing to deploy. Long-lived connections (WebSocket, `05-http-stack/websocket.md`) held by the old process need the same drain deadline as `graceful-shutdown.md` — a rolling restart doesn't make that problem go away, it just adds "and don't refuse new connections while draining."

## Practice
1. In `proxy`, implement the `SO_REUSEPORT` bind helper and manually run two instances bound to the same port; confirm (via a per-instance log line) that the kernel is distributing connections across both.
2. Wire a readiness check the new process must pass (successful upstream health check) before it signals "ready to take traffic" — don't send `SIGTERM` to the old process until then.
3. Write a small restart script: start new process, wait for readiness, `SIGTERM` the old process (reusing the drain logic from `graceful-shutdown.md`), confirm old process exits after draining.
4. Load-test through the restart (`12-testing/load-testing.md`) and confirm zero failed requests across the handover — any failures mean the overlap window or drain deadline is wrong.
5. Optional: write a systemd `.socket` + `.service` unit pair for `proxy` and compare — confirm `systemctl restart` alone achieves the same zero-dropped-connection result with none of the `SO_REUSEPORT` code.
