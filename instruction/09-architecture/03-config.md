# Config Reload

## What to learn
### Why "just restart the process" isn't good enough
A production L7 proxy is fronting live traffic; a config change (new upstream, updated rate limit) that requires a restart means a connection drop for every in-flight request. Hot reload means: load new config, validate it, atomically swap it in for new requests, while existing requests keep running against whatever config they started with (or the new one, if the field doesn't affect in-flight requests).

### SIGHUP as the reload trigger
The Unix convention (nginx, most daemons) is: `SIGHUP` = "reload config," `SIGTERM` = "shut down gracefully" (see `02-linux/04-signals.md`, `09-architecture/04-graceful-shutdown.md`). Listen for it with `tokio::signal::unix::signal(SignalKind::hangup())` rather than blocking signal handling — this keeps the reload async and non-disruptive to in-flight I/O.

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

Gotcha: the alternative trigger — watching the file with `inotify`/`notify`
— has a race that `SIGHUP` doesn't. An editor or deploy tool writing the
file in place fires an event when the write *starts*, so you read a
truncated, half-written config and reject a change that was actually fine.
Requiring writers to do an atomic rename (write to a temp file, then
`rename()` over the target) fixes it, because rename is atomic and the
watcher sees only the complete file — but you are now depending on every
tool that touches the file to behave. `SIGHUP` puts the "I am done
writing" decision where it belongs: with the writer.

Gotcha: in a container, there may be nobody to send `SIGHUP`. Kubernetes
ConfigMap updates appear as a symlink swap in the mounted volume, so
file-watching (with the rename caveat handled — the swap *is* atomic
there) is often the only trigger available. Support both.

### Validate before swap, always
Never apply a config that hasn't been fully parsed and validated (upstream addresses resolve, TLS cert/key pair match, no duplicate routes) — a bad reload should log an error and keep serving traffic on the last-known-good config, not crash or serve broken routes. This is the single most important reliability property of config reload: **reload failure must be a no-op, not an outage.**

"Validated" has to mean more than "parsed." The checks that actually catch
real breakage are the ones that try the side effects:
- **Certificate and key actually load and match** (`01-network/07-tls.md`) —
  a path typo or a mismatched pair is a total outage for that vhost.
- **Routes don't conflict** (`05-http-stack/03-router.md`) — two rules that
  can never be distinguished mean one endpoint silently disappears.
- **Referenced upstream pools exist** — a route pointing at a pool name
  that isn't defined should fail validation, not 502 at request time.
- **New listeners can actually bind** — if the config changes a port,
  discovering it's already in use *after* you've closed the old listener
  is an outage you caused.

That last one is why a real implementation is two-phase: *prepare*
everything that can fail (parse, load certs, bind new sockets) into a
staging object, and only then *commit* by swapping pointers. Anything that
fails during prepare leaves the running config completely untouched.

Gotcha: validation that hits the network (resolving upstream DNS,
connecting to check liveness) makes reload fail when a *dependency* is
down, which is the fail-static problem from
`06-proxy/07-service-discovery.md` in a new costume. Validate syntax and
internal consistency strictly; treat an unresolvable upstream as a health
check's problem, not a reason to reject an otherwise valid config.

### Not everything can be hot-reloaded
Be explicit, in the config's own documentation, about which fields take
effect on reload and which need a restart. Typical split:
- **Reloadable**: upstream lists, routes, rate limits, WAF rules, log
  level, timeouts for new requests.
- **Needs a new listener (or restart)**: bind address/port, TLS protocol
  versions and cipher configuration, worker thread count.

Gotcha: silently ignoring a changed non-reloadable field is worse than
rejecting it. An operator who edits the listen port, sends `SIGHUP`, sees
"reload successful," and finds the old port still serving has been
actively misled. Either apply it (rebind, which `09-architecture/05-rolling-restart.md`
covers properly) or fail the reload with a message naming the field.

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

Gotcha: `ArcSwap` guarantees each `load()` returns a consistent snapshot —
it does **not** guarantee two `load()` calls return the *same* snapshot. A
request that loads config to pick an upstream pool and loads it again to
read that pool's timeout can straddle a reload and combine fields from two
different configs. Load **once** at the start of a request, hold the `Arc`
for its duration, and pass it down the pipeline
(`09-architecture/01-components.md`'s extensions). This bug is rare, entirely
non-deterministic, and essentially undebuggable after the fact.

Gotcha: holding that `Arc` for the request's lifetime is also what keeps
the old config alive while in-flight requests use it — the memory is
released when the last request holding it finishes, which is exactly the
refcount-driven drain from `06-proxy/07-service-discovery.md`. A long-lived
streaming request pins its config version; that's correct, and worth
knowing when you wonder why an old config hasn't been dropped.

### Secrets don't belong in the config file
Config is read from disk, logged on reload, dumped in debug output, and
frequently committed to a repository by accident. Keep credentials
(upstream auth, JWT signing keys) in environment variables or a secrets
store referenced *by name* from the config, and wrap them in a redacting
newtype (`08-observability/01-logging.md`) so `{:?}` on the config can't leak
them.

Gotcha: logging the config on reload is genuinely useful for auditing what
changed. Log a **hash or version**, plus a structured diff of non-secret
fields — never the whole struct.

### Versioned config for rollback
Keep the last N valid configs (or at least the last one) so an operator can roll back instantly if a syntactically-valid-but-logically-wrong config causes elevated error rates — tie the decision to roll back to the error-rate metrics from `09-architecture/06-canary-deploy.md`/`08-observability/02-metrics.md`.

Gotcha: a reload that *succeeds* and then degrades traffic is the
dangerous case, because nothing alerted — validation passed. Emit a
config version/hash as a metric label or a gauge so dashboards can
correlate "error rate rose" with "config changed at this moment", and
alert on reload *failures* as a ticket (`08-observability/06-alerting.md`):
a proxy running happily on stale config while every reload attempt fails
is a silent, compounding divergence from what operators believe is
deployed.

## Practice
Build these in order.

1. Do steps 1-5 in `labs/13-hot-reload`, then repeat in `proxy`. Define a
   `Config` struct with `serde` + `toml` (upstreams, routes, rate limits,
   TLS paths). **Done when** it loads at startup behind an `ArcSwap` and
   handlers read via `.load()`.
2. Load config exactly once per request and pass the `Arc` down the
   pipeline. **Done when** a test that reloads continuously under
   concurrent load cannot produce a request that saw two different config
   versions — instrument the version into the request span to prove it.
3. Add a `SIGHUP` handler with two-phase prepare/commit: parse, validate
   route conflicts, load and match cert/key, bind any new listener — all
   before swapping. **Done when** each failure mode leaves the old config
   serving.
4. Write the negative tests. **Done when** a duplicate route, a
   mismatched cert/key pair, a route referencing an undefined upstream
   pool, and a port already in use each produce a logged rejection and
   zero change in behavior.
5. Add file-watching as a second trigger, handling the partial-write race.
   **Done when** a `cp` that writes in place does not trigger a spurious
   rejection, and an atomic rename does trigger a reload.
6. Classify every field as reloadable or not, and reject changes to
   non-reloadable ones with a message naming the field. **Done when**
   editing the listen port produces a clear failure rather than a
   misleading success.
7. Move secrets out of the file and wrap them in a redacting newtype.
   **Done when** `{:?}` on the loaded config prints no secret material and
   the reload log line contains a config hash plus a non-secret diff.
8. Emit config version as a metric and alert on reload failure. **Done
   when** a dashboard can correlate an error-rate change with the exact
   reload that caused it, and repeated silent reload failures produce a
   ticket.
9. Keep the last N valid configs with a rollback path. **Done when** an
   operator command reverts to the previous version without a restart and
   without dropping in-flight requests.
