# Graceful Shutdown

## What to learn
### SIGTERM vs SIGKILL, and why the proxy must handle SIGTERM itself
Orchestrators (systemd, Kubernetes) send `SIGTERM` first and give the process a grace period before `SIGKILL` (which cannot be caught — it's an instant hard stop). If the proxy doesn't catch `SIGTERM` and act on it, it either dies immediately mid-request (dropped connections) or gets hard-killed after the grace period expires, which is the same outcome. See `02-linux/signals.md`.

### The drain sequence
On `SIGTERM`: (1) stop accepting *new* connections/requests immediately — deregister from any load balancer/service discovery first if possible, so upstream traffic stops arriving before you even close the listener; (2) let in-flight requests finish normally; (3) after a bounded deadline, forcibly cancel anything still running and exit. Getting step 1 and 3 wrong is the most common cause of a "graceful" shutdown that still causes client-visible errors.

```rust
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;
use std::time::Duration;

let (shutdown_tx, _) = broadcast::channel::<()>(1);
let mut sigterm = signal(SignalKind::terminate())?;

tokio::select! {
    _ = sigterm.recv() => {
        tracing::info!("SIGTERM received, draining");
        let _ = shutdown_tx.send(()); // tell all connection tasks to stop accepting new work
        tokio::time::timeout(Duration::from_secs(30), wait_for_all_connections_to_finish()).await.ok();
    }
}
```

### Listener shutdown vs connection shutdown
Closing the listening socket stops *new* TCP connections but does nothing for already-open keep-alive connections that might still send more requests. Each connection handler needs its own signal (e.g. a `broadcast::Receiver` cloned per task) to know "finish the current request, then respond `Connection: close` and stop reading further requests on this socket instead of waiting for the next one."

### Deadlines and forced cancellation
Always bound the drain with a timeout (`tokio::time::timeout`). A single stuck upstream connection (hung TCP, slowloris-style client) must not block shutdown forever — after the deadline, cancel remaining tasks and exit anyway, logging which requests were force-cancelled so it's visible in `08-observability/logging.md`, not silent.

## Practice
1. In `proxy`, register a `SIGTERM` handler using `tokio::signal::unix` and confirm (via a log line) it fires under `docker stop` / `kill -TERM`.
2. Implement a shutdown broadcast channel; on signal, stop the accept loop first, then notify per-connection tasks to stop keep-alive reuse.
3. Wrap the drain wait in a `tokio::time::timeout` (e.g. 30s); force-cancel and log any tasks still running after the deadline.
4. Load-test the proxy (`12-testing/load-testing.md`) while sending `SIGTERM` mid-test; confirm the client sees zero connection-reset errors for requests that were in flight, and clean rejects for anything sent after drain started.
5. If you built `06-proxy/service-discovery.md`, deregister from discovery as the very first drain step and verify (via metrics) that new traffic actually stops before existing connections finish.
