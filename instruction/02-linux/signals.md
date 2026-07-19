# Signals

HUP, TERM, QUIT.

## What to learn

### The three signals a proxy actually cares about
- `SIGHUP` — conventionally means "reload configuration without restarting."
  No default handler forces this meaning; it's a convention nginx and most
  daemons follow. Ties directly to `09-architecture/config.md`.
- `SIGTERM` — "shut down gracefully": stop accepting new connections, finish
  in-flight requests, then exit. This is what orchestrators (systemd,
  Kubernetes) send before escalating to `SIGKILL`. Ties directly to
  `09-architecture/graceful-shutdown.md`.
- `SIGINT`/`SIGQUIT` — `SIGINT` is Ctrl-C, typically treated the same as
  `SIGTERM` in dev; `SIGQUIT` traditionally triggers a core dump and is
  rarely handled specially in a proxy.

### Signal-safety constraints
A traditional Unix signal handler runs asynchronously, possibly *inside*
another syscall, on whatever thread the kernel picked — allocating memory,
locking a mutex, or calling most libc functions from inside a raw handler is
undefined-behavior-adjacent (not "async-signal-safe"). The idiomatic fix,
and what every serious async runtime does, is: the raw handler does nothing
but write a byte to a pipe/eventfd (or increment an atomic), and your actual
reload/shutdown logic runs later on a normal thread that's woken by that
byte.

### tokio::signal
Tokio implements exactly that pattern for you:
```rust
use tokio::signal::unix::{signal, SignalKind};

let mut sighup = signal(SignalKind::hangup())?;
let mut sigterm = signal(SignalKind::terminate())?;

tokio::select! {
    _ = sighup.recv() => { /* reload config */ }
    _ = sigterm.recv() => { /* begin graceful shutdown */ }
}
```
Gotcha: registering a `tokio::signal` handler for a given `SignalKind`
replaces the default disposition once, globally, per process — you can't
have two independent listeners racing to be "the" SIGTERM handler; fan the
single received signal out to whatever subsystems need to react (drain
connections, flush logs, stop the listener) from one place.

### Ordering under real deployment
Kubernetes sends `SIGTERM`, waits `terminationGracePeriodSeconds` (default
30s), then sends `SIGKILL`. If your graceful shutdown (draining in-flight
requests, closing upstream connections cleanly) can take longer than that
window under peak load, requests get hard-killed anyway — the grace period
must be tuned against your actual p99 request duration, not left at the
default.

## Practice
1. Write a tiny binary that registers `tokio::signal` handlers for SIGHUP and SIGTERM and just prints which one fired.
2. Send `kill -HUP <pid>` and `kill -TERM <pid>` manually and confirm both are caught without killing the process.
3. Implement SIGHUP-triggered config reload in `proxy` per `09-architecture/config.md` — verify existing connections are unaffected by a reload.
4. Implement SIGTERM-triggered graceful shutdown per `09-architecture/graceful-shutdown.md`: stop the listener, let in-flight requests finish, then exit.
5. Simulate the Kubernetes grace-period scenario: hold a slow request open, send SIGTERM, and confirm your shutdown either finishes in time or is cleanly killed rather than corrupting state.
