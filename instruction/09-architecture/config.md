# Config Reload

## What to learn
### Why "just restart the process" isn't good enough
A production L7 proxy is fronting live traffic; a config change (new upstream, updated rate limit) that requires a restart means a connection drop for every in-flight request. Hot reload means: load new config, validate it, atomically swap it in for new requests, while existing requests keep running against whatever config they started with (or the new one, if the field doesn't affect in-flight requests).

### SIGHUP as the reload trigger
The Unix convention (nginx, most daemons) is: `SIGHUP` = "reload config," `SIGTERM` = "shut down gracefully" (see `02-linux/signals.md`, `09-architecture/graceful-shutdown.md`). Listen for it with `tokio::signal::unix::signal(SignalKind::hangup())` rather than blocking signal handling — this keeps the reload async and non-disruptive to in-flight I/O.

```rust
use tokio::signal::unix::{signal, SignalKind};

let mut hangup = signal(SignalKind::hangup())?;
tokio::spawn(async move {
    loop {
        hangup.recv().await;
        match load_and_validate_config().await {
            Ok(new_cfg) => config_store.swap(new_cfg),
            Err(e) => tracing::error!(error = %e, "config reload rejected, keeping old config"),
        }
    }
});
```

### Validate before swap, always
Never apply a config that hasn't been fully parsed and validated (upstream addresses resolve, TLS cert/key pair match, no duplicate routes) — a bad reload should log an error and keep serving traffic on the last-known-good config, not crash or serve broken routes. This is the single most important reliability property of config reload: **reload failure must be a no-op, not an outage.**

### Representing "current config" for concurrent readers
Requests are being handled concurrently while a reload might happen; use `arc_swap::ArcSwap<Config>` (or `tokio::sync::watch`) so readers get a consistent snapshot without locking on every request, and the swap itself is a single atomic pointer update.

```rust
use arc_swap::ArcSwap;
use std::sync::Arc;

static CONFIG: once_cell::sync::Lazy<ArcSwap<Config>> =
    once_cell::sync::Lazy::new(|| ArcSwap::from_pointee(Config::default()));

// reader (hot path): CONFIG.load() -> Arc<Config>, cheap, lock-free
// writer (reload):   CONFIG.store(Arc::new(new_config))
```

### Versioned config for rollback
Keep the last N valid configs (or at least the last one) so an operator can roll back instantly if a syntactically-valid-but-logically-wrong config causes elevated error rates — tie the decision to roll back to the error-rate metrics from `09-architecture/canary-deploy.md`/`08-observability/metrics.md`.

## Practice
1. Define a `Config` struct with `serde` + `toml` for `proxy` (upstreams, rate limits, TLS paths).
2. Load it once at startup behind an `ArcSwap<Config>`; have request handlers read via `.load()`.
3. Add a SIGHUP handler that re-reads the file, validates it (fail closed on parse error or unreachable upstream), and swaps only on success.
4. Write a test that reloads with an intentionally broken config (duplicate route) and asserts the old config is still being served afterward.
5. Log a structured event (per `08-observability/logging.md`) on every reload attempt, success or failure, including a config version/hash.
