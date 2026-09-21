# Graceful Shutdown

## What to learn
### SIGTERM vs SIGKILL, and why the proxy must handle SIGTERM itself
Orchestrators (systemd, Kubernetes) send `SIGTERM` first and give the process a grace period before `SIGKILL` (which cannot be caught — it's an instant hard stop). If the proxy doesn't catch `SIGTERM` and act on it, it either dies immediately mid-request (dropped connections) or gets hard-killed after the grace period expires, which is the same outcome. See `02-linux/04-signals.md`.

Gotcha: as PID 1 in a container, the default signal dispositions don't
apply — the kernel does not kill PID 1 for signals it hasn't explicitly
handled. A proxy that *doesn't* install a `SIGTERM` handler and runs as
PID 1 therefore ignores `docker stop` entirely and waits out the full
grace period before being killed. Every shutdown then takes the maximum
time, which people usually diagnose as "shutdown is slow" rather than
"shutdown never started."

Gotcha: handle `SIGINT` too (Ctrl-C in local development), and make a
*second* signal force an immediate exit. An operator who sends `SIGTERM`
and sees nothing happen for 30 seconds will escalate; giving them a
documented "press it again to stop waiting" is better than having them
reach for `SIGKILL` and learn that it works.

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

### Keep accepting for a moment after SIGTERM — the counterintuitive part
The sequence above still drops traffic in Kubernetes, and the reason is
worth internalizing because it defeats every otherwise-correct
implementation.

Pod termination and endpoint removal happen **concurrently, not in
order**. The kubelet sends `SIGTERM` at the same time the control plane
starts propagating your removal to every node's kube-proxy/ipvs rules and
to every ingress controller's endpoint list. That propagation takes
hundreds of milliseconds to several seconds. So for a window *after* you
received `SIGTERM`, load balancers are still sending you new connections —
and if you closed your listener the instant the signal arrived, every one
of those is a connection refused.

The fix is a deliberate **pre-stop delay**: on `SIGTERM`, keep accepting
and serving normally for a few seconds (5-15 is typical), *then* begin the
drain. Kubernetes also offers a `preStop` hook (`sleep 10`) that runs
before `SIGTERM` is sent, which achieves the same thing outside your code.
Either way it is an explicit, configured wait — not something that happens
by default.

```
SIGTERM ──► [ keep serving, ~5-15s ]  ──► stop accepting ──► drain in-flight ──► exit
            (LBs learn you're gone)       (close listener)    (bounded deadline)
```

Gotcha: this interacts with your own service discovery
(`06-proxy/07-service-discovery.md`). If the proxy registers itself, active
deregistration as the first drain step shortens the window a lot — but it
never eliminates it, because other components still cache the old
membership. Keep the delay even when you deregister actively.

### Listener shutdown vs connection shutdown
Closing the listening socket stops *new* TCP connections but does nothing for already-open keep-alive connections that might still send more requests. Each connection handler needs its own signal (e.g. a `broadcast::Receiver` cloned per task) to know "finish the current request, then respond `Connection: close` and stop reading further requests on this socket instead of waiting for the next one."

For HTTP/2 the equivalent is `GOAWAY`, and it's better than
`Connection: close` in a specific way: it names the highest stream ID the
server will process, so a client with requests in flight knows exactly
which ones were accepted and which it must retry elsewhere. The graceful
form is two `GOAWAY` frames — one with the maximum stream ID to announce
intent (letting in-flight streams finish while the client stops opening
new ones), then a final one with the real last-processed ID
(`01-network/05-http2.md`).

Gotcha: an idle keep-alive connection is the same race as
`05-http-stack/04-keepalive.md`'s close/request collision, now happening
across your whole connection table at once. Announcing (`Connection:
close` / `GOAWAY`) before closing is what turns "client sees a reset" into
"client opens a new connection elsewhere."

### Long-lived connections need a different policy
"Let in-flight requests finish" assumes requests finish. A WebSocket
(`05-http-stack/09-websocket.md`), a server-streaming gRPC call
(`05-http-stack/10-grpc.md`), or an SSE stream may be minutes or hours from
completing, and waiting for them means never shutting down.

They need an explicit policy, decided per connection type: send a
WebSocket close frame with a "going away" code (1001) so the client
reconnects cleanly, or end a stream with a retryable gRPC status
(`UNAVAILABLE`), rather than letting the deadline force a bare TCP reset.
A clean protocol-level close lets clients reconnect to another instance
immediately; a reset makes them retry blindly and often more slowly.

### Deadlines and forced cancellation
Always bound the drain with a timeout (`tokio::time::timeout`). A single stuck upstream connection (hung TCP, slowloris-style client) must not block shutdown forever — after the deadline, cancel remaining tasks and exit anyway, logging which requests were force-cancelled so it's visible in `08-observability/01-logging.md`, not silent.

Gotcha: the total of your pre-stop delay plus your drain deadline must be
**less** than the orchestrator's grace period
(`terminationGracePeriodSeconds`, default 30s in Kubernetes), or `SIGKILL`
arrives mid-drain and you get the ungraceful shutdown you wrote all this
code to avoid. Write the arithmetic down next to both settings; they are
two numbers in two different repositories and they *will* drift.

### What else has to flush before exit
Shutdown isn't just connections. Anything buffered in the name of
performance is unflushed data at exit:
- **Log and trace buffers** — `tracing_appender`'s non-blocking writer and
  the OTLP batch exporter (`08-observability/03-tracing.md`) both hold
  records in memory. Losing exactly the records from the shutdown window
  is losing the evidence for whatever caused the shutdown.
- **Metrics** — a final scrape won't happen, so any counter movement since
  the last scrape is gone. This is a known, accepted gap with pull-based
  metrics; know that it exists before concluding a deploy caused a drop to
  zero.
- **Upstream connections** — close pooled idle connections explicitly
  (`06-proxy/01-upstream.md`) rather than letting the process exit drop them,
  so upstreams see clean closes instead of resets.

Gotcha: flush ordering matters — flush telemetry *last*, after the drain
completes, so the shutdown's own events are included.

## Practice
Build these in order.

1. Register `SIGTERM` and `SIGINT` handlers in `proxy`, with a second
   signal forcing immediate exit. **Done when** `docker stop` triggers the
   drain log line — verify while running as PID 1, since that's where the
   default-disposition trap lives.
2. Implement the shutdown broadcast: stop the accept loop, then notify
   per-connection tasks to stop keep-alive reuse and send
   `Connection: close` / `GOAWAY`. **Done when** an idle keep-alive client
   is told to stop rather than discovering it by reset.
3. Add the bounded drain deadline with logging of force-cancelled
   requests. **Done when** a deliberately hung upstream doesn't prevent
   exit, and the cancelled requests are named in the logs.
4. Load-test while sending `SIGTERM` mid-test
   (`12-testing/01-load-testing.md`). **Done when** in-flight requests see
   zero connection resets. Expect to still see errors from *newly
   arriving* requests at this stage — that's step 5.
5. Add the pre-stop delay and re-run step 4 behind a load balancer (or a
   second proxy standing in for one). **Done when** the residual
   connection-refused errors from step 4 disappear — measure both runs, as
   this is the whole point of the delay.
6. Verify the arithmetic. **Done when** pre-stop delay + drain deadline is
   demonstrably under the orchestrator's grace period, and you've tested
   what happens when it *isn't* (set the grace period low and watch
   `SIGKILL` land mid-drain) so you recognize the failure.
7. Add explicit close policies for long-lived connections. **Done when** a
   WebSocket gets a 1001 close frame and a streaming gRPC call gets
   `UNAVAILABLE`, and both clients reconnect to another instance without
   an error surfacing to the user.
8. Flush telemetry after the drain. **Done when** log lines and spans from
   requests completed during shutdown still reach their backends, and
   pooled upstream connections are closed cleanly rather than reset.
9. If you built service discovery (`06-proxy/07-service-discovery.md`),
   deregister as the very first drain step. **Done when** metrics show
   inbound request rate falling to zero *before* the listener closes.
