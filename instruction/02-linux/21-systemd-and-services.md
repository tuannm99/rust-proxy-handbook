# Running as a Service: systemd, Supervision, and the Process Contract

How a production proxy is actually started, restarted, signalled, limited and observed on a Linux host — and the
contract between the supervisor and your process (signals in, exit code and readiness out). Ties together
[`04-process-lifecycle.md`](04-process-lifecycle.md), [`17-signals.md`](17-signals.md), [`20-limits-and-proc.md`](20-limits-and-proc.md) and [`13-containers.md`](13-containers.md).

## What to learn

### What a supervisor does
A **supervisor** (systemd on most distributions; the container runtime and Kubernetes' kubelet in containers) starts your
process in a controlled environment — user, working directory, environment, limits, cgroup — watches it, restarts it if it dies,
delivers stop signals, and collects its output. Your process's job is to behave predictably within that contract rather than
daemonizing itself: **stay in the foreground, log to stdout/stderr, handle `SIGTERM`, exit with a meaningful code**
([`04-process-lifecycle.md`](04-process-lifecycle.md), [`08-observability/01-logging.md`](../08-observability/01-logging.md)). systemd is PID 1 on the host and so reaps orphans and owns every service's cgroup.

### A unit file
```ini
# /etc/systemd/system/proxy.service
# (systemd does not allow trailing comments, so each comment gets its own line)
[Unit]
Description=L7 reverse proxy
After=network-online.target
Wants=network-online.target

[Service]
# "notify" is better once the proxy reports readiness; see Type=notify below
Type=simple
ExecStart=/usr/local/bin/proxy --config /etc/proxy/proxy.toml
# reload contract: SIGHUP re-reads config
ExecReload=/bin/kill -HUP $MAINPID
User=proxy
Group=proxy
Restart=on-failure
RestartSec=1
# raise the fd ceiling (see 20-limits-and-proc.md)
LimitNOFILE=1048576
# bind :443 without being root
AmbientCapabilities=CAP_NET_BIND_SERVICE
NoNewPrivileges=yes
# how long graceful shutdown may take before SIGKILL
TimeoutStopSec=30

[Install]
WantedBy=multi-user.target
```
`systemctl daemon-reload` (after editing), `systemctl start|stop|restart|reload|status proxy`, `systemctl enable` (start at boot),
`journalctl -u proxy -f` (its logs). `ExecStart` must name the binary directly: if it's wrapped in `sh -c`, signals go to the shell
([`04-process-lifecycle.md`](04-process-lifecycle.md)).

### The stop contract: SIGTERM, timeout, SIGKILL
`systemctl stop` sends **`SIGTERM`** to the service's main process, waits up to `TimeoutStopSec`, then sends **`SIGKILL`** to everything left in
the cgroup. So your proxy should, on `SIGTERM`, stop accepting new connections, let in-flight requests finish (bounded by a deadline),
then exit 0 ([`17-signals.md`](17-signals.md), [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md)). Set `TimeoutStopSec` longer than your drain deadline, or a slow
drain is cut off by `SIGKILL` — and the exit status 137 in the logs is the clue ([`04-process-lifecycle.md`](04-process-lifecycle.md)). Kubernetes
has the same shape: `SIGTERM`, then `terminationGracePeriodSeconds`, then `SIGKILL`; plus a `preStop` hook and the fact that removal from load-balancer
endpoints is *concurrent* with the signal, so a proxy often needs to keep serving briefly after `SIGTERM` ([`09-architecture/06-canary-deploy.md`](../09-architecture/06-canary-deploy.md)).
`ExecReload=` is the reload contract: `SIGHUP` -> re-read config without dropping connections ([`09-architecture/03-config.md`](../09-architecture/03-config.md)).

### Restart policy and crash loops
`Restart=on-failure` restarts after a non-zero exit or a fatal signal; `Restart=always` also after a clean exit. `RestartSec` delays it. By default systemd
gives up after several restarts in a short window (`StartLimitBurst`/`StartLimitIntervalSec`). A proxy that **crashes on bad config** will be restarted into the same
crash — *fail fast on invalid config at startup with a clear message and a distinct exit code*, and validate a new config **before** swapping it in a reload so a typo
doesn't take the service down ([`09-architecture/03-config.md`](../09-architecture/03-config.md)). Alert on restart count, not just on "process is up" ([`08-observability/06-alerting.md`](../08-observability/06-alerting.md)).

### Type=notify: readiness, not just "started"
With `Type=simple`, systemd considers the service started the instant the process is forked — before it has bound its port or loaded its config. Dependents
and health gates then race ahead. `Type=notify` fixes this: the process sends `READY=1` over the socket named in `$NOTIFY_SOCKET` (via `sd_notify`, or the
`sd-notify` crate) *after* it is genuinely ready, and can send `RELOADING=1`/`STOPPING=1`. The same distinction exists in Kubernetes as **readiness probes** vs the
process merely running ([`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md)). Avoid `Type=forking` (the old daemonizing style) for new software.

### Socket activation: systemd owns the listening socket
A `.socket` unit lets **systemd bind the port** and hand your process the already-listening fd (passed as fd 3, with `$LISTEN_FDS`/`$LISTEN_PID` set; the
`listenfd` or `sd-listen-fds` crates read it). Benefits: the port stays open and **connections queue in the kernel backlog while the service restarts** — a
simple zero-downtime restart ([`01-network/11-socket.md`](../01-network/11-socket.md)); the service can bind port 443 and run unprivileged because it never calls `bind` itself
([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)); and startup order stops mattering. It uses the same inheritance mechanism as a hot restart
([`10-ipc.md`](10-ipc.md), [`09-architecture/05-rolling-restart.md`](../09-architecture/05-rolling-restart.md)): a listening fd outliving any one process.

### cgroups per service: limits and accounting for free
systemd places each service in its own cgroup ([`13-containers.md`](13-containers.md)), so `systemctl status proxy` lists every process of the service (including children), and unit
settings map to cgroup limits: `MemoryMax=`, `CPUQuota=200%`, `TasksMax=`, `IOWeight=`. `systemd-cgls` shows the tree; `systemd-cgtop` shows live usage. Be careful with `CPUQuota`:
it is a throttling cap with the latency effect described in [`12-cpu-scheduling.md`](12-cpu-scheduling.md). `MemoryMax` triggers the cgroup OOM killer ([`16-memory.md`](16-memory.md)).

### Hardening as configuration
A dozen unit lines give sandboxing without code: `NoNewPrivileges=yes`, `ProtectSystem=strict` (read-only OS tree), `ProtectHome=yes`, `PrivateTmp=yes`,
`ReadWritePaths=/var/log/proxy`, `CapabilityBoundingSet=CAP_NET_BIND_SERVICE`, `SystemCallFilter=@system-service` (a seccomp allow-list,
[`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)), `RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX`. `systemd-analyze security proxy` scores a unit and
lists what is still exposed. Hardening that is too tight shows up as `EACCES`/`EPERM` in `strace`.

### Logs: journald and stdout
Anything on stdout/stderr lands in the journal, tagged with the unit and timestamped; `journalctl -u proxy --since '10 min ago' -o json` is structured retrieval.
Logging to files, then, requires rotation handled outside the process ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)); logging to stdout is the contract containers use as well.

### Gotcha: the environment you tested in is not the one it runs in
Under a supervisor the proxy runs with a minimal `PATH`, no TTY, a different working directory, no shell startup files, different limits, a different user, and
possibly a read-only filesystem. "Works when I run it" and "fails under systemd" is nearly always one of these. `systemd-run --pty --uid=proxy -p LimitNOFILE=1024 ...`
or `systemd-run --scope` reproduces the environment interactively; `systemctl show proxy` prints every effective setting.

## Practice

Build these in order.

1. Write the unit above for your proxy build ([`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), or `proxy/` once built) and install it under `/etc/systemd/system/` (or as a `--user`
   unit in `~/.config/systemd/user/`). **Done when** `systemctl status` shows `active (running)`, `journalctl -u` shows its logs, and `systemctl show -p MainPID` equals the
   PID of your binary (no shell in between).
2. Verify the stop contract: add a `SIGTERM` handler that drains in-flight requests, hold a slow request open with `curl`, and `systemctl stop`. **Done when** the
   slow request finishes, the process exits 0 and `journalctl` shows no `SIGKILL`; then lower `TimeoutStopSec` below the drain time and show the 137/`killed` result.
3. Make it crash on purpose (`kill -SEGV $MAINPID`, then a bad config) and watch `Restart=` behavior and `StartLimitBurst`. **Done when** you can explain each line of
   `systemctl status` / `journalctl` for both cases and the proxy refuses a bad config at startup with a clear message.
4. Add `LimitNOFILE=` and `AmbientCapabilities=CAP_NET_BIND_SERVICE`, run as `User=proxy`, and bind `:80`. **Done when** `/proc/<pid>/limits` and `/proc/<pid>/status`
   (`CapEff`) show both effects.
5. Convert to socket activation: a `proxy.socket` with `ListenStream=8080` and read the fd via `$LISTEN_FDS`. **Done when** `systemctl restart proxy` during a `curl` loop shows no refused
   connections (requests wait briefly instead).
6. Run `systemd-analyze security proxy.service`, apply at least five hardening options, and re-test. **Done when** the score improves and the proxy still serves,
   with any `EACCES` you caused explained via `strace`.
7. Add `Type=notify` (the `sd-notify` crate) and `READY=1` after the listener is bound. **Done when** `systemctl start` blocks until the port is truly listening, and a dependent unit with `After=proxy.service`
   can rely on it.
