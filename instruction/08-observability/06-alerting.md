# Alerting

What actually pages a human, vs what's just a line on a dashboard.
The SLIs and error budgets it all rests on are in
`08-observability/05-slo.md`.

## What to learn
### What this is built on
Every alert below fires on an **error budget burn rate**, which only means
something once you have an SLI and an SLO to derive it from — including
the decisions about whether 4xx counts, whether a slow success is a
success, and which requests are even valid. A proxy also needs two
separate SLIs, because "the proxy failed" and "the upstream failed" have
different owners.

See `08-observability/05-slo.md`; this file assumes those exist.

### Alert on symptoms, not causes
Page on what the *user* experiences (elevated error rate, elevated p99 latency, the proxy itself down) — not on every internal condition that *might* cause a symptom (one of three upstreams unhealthy, one retry occurred, GC pause of 50ms). If the load balancer in `06-proxy/02-load-balancer.md` and health checks in `06-proxy/03-healthcheck.md` are doing their job, losing one upstream shouldn't page anyone; losing all of them should. Cause-level signals still matter — keep them as metrics/dashboards (`08-observability/02-metrics.md`) for root-causing an incident after the symptom-level alert already woke someone up.

### The exceptions: things that are invisible until it's too late
"Symptoms only" has a specific and important class of exceptions —
conditions with no symptom *now* and a guaranteed one later. These are
worth a ticket-level alert precisely because waiting for the symptom means
waiting for the outage:
- **Certificate expiry** (`01-network/07-tls.md`,
  `05-http-stack/11-vhost-routing.md`) — alert weeks out, per certificate.
  The symptom is total failure at an exactly predictable moment.
- **Config reload failures** (`09-architecture/03-config.md`) — the proxy
  keeps running on old config and looks perfectly healthy while diverging
  from what operators think is deployed.
- **Disk filling** with logs (`08-observability/01-logging.md`), fd count
  approaching its limit, and connection-pool saturation trending up.
- **Error budget burn rate**, which is the generalization of all of these:
  not broken now, on track to be.

### Multi-window burn-rate alerting
A naive "error rate > 1% for 5 minutes" alert either fires on tiny blips (noisy) or misses a slow leak that never crosses the instant threshold. Burn-rate alerting instead asks "at the current error rate, how fast are we consuming the error budget?" and checks it over two windows: a short window (5m) at a high burn-rate threshold catches fast, severe outages fast; a long window (1h or 6h) at a lower threshold catches a slow, sustained degradation before it eats the whole month's budget. Require *both* windows to agree before paging — this is what keeps a single-window alert from flapping.

```promql
# page if burning error budget >14x the sustainable rate, sustained over
# both a 5m and 1h window (a common two-window multi-burn-rate pattern)
(
  sum(rate(proxy_requests_total{status=~"5.."}[5m]))
  /
  sum(rate(proxy_requests_total[5m]))
) > (14 * 0.001)
and
(
  sum(rate(proxy_requests_total{status=~"5.."}[1h]))
  /
  sum(rate(proxy_requests_total[1h]))
) > (14 * 0.001)
```
Gotcha: the ratio's denominator can be near-zero during low traffic (e.g. 3am), producing wild swings from a handful of requests — guard with a minimum request-count floor (`sum(rate(...[5m])) > N`) before evaluating the ratio at all.

In practice you want a ladder of these rather than one rule, so that
fast-and-severe pages immediately while slow-and-mild opens a ticket. The
SRE-workbook pairs, for a 30-day budget, are roughly: **14.4x** burn over
1h (with a 5m short window) → page, consuming 2% of budget; **6x** over 6h
(30m short window) → page; **1x** over 3d (6h short window) → ticket. The
short window in each pair exists so the alert *resolves* quickly once the
problem stops, rather than staying lit for the length of the long window.

Gotcha: the burn rate is computed against your SLO, so changing the SLO
silently changes every alert threshold derived from it. Keep the SLO
target in one recording rule that the alerts reference, rather than
hard-coding `0.001` in five places (as the example above does, and as you
should not).

### Alert on the absence of data
Every alert above fires on metrics *the proxy exports*. If the proxy is
dead, hung, or unable to reach the metrics backend, those metrics stop
arriving — and an alert that needs data to fire will sit quietly saying
nothing is wrong. This is the single most embarrassing gap in a monitoring
setup and it is easy to close:
- **Alert on missing data** explicitly (`absent()`, or `up == 0` for the
  scrape target). A proxy that stopped reporting is an incident until
  proven otherwise.
- **Probe from outside** with a black-box/synthetic check that actually
  sends a request through the proxy from another network. It catches
  everything internal metrics structurally cannot: the process is up but
  the listener is wedged, DNS for your hostname broke, a firewall rule
  changed, the certificate expired.

Gotcha: check what your alerting path itself depends on. If alert
notifications route through infrastructure that sits behind this proxy, a
proxy outage suppresses the alert about the proxy outage. The synthetic
probe should run somewhere with an independent path out.

### Alert fatigue and routing
Every alert needs an owner, a severity, and a runbook link — an alert nobody can act on at 3am just trains people to ignore pages. Route by severity: page (wakes someone up, symptom-level, budget-burning), ticket (business-hours, cause-level, not yet user-visible), and metric-only (no notification, dashboard/debugging aid only). If a page fires and the response is always "nothing to do, it self-resolved" more than a couple of times, that's a signal to move it down a tier, not a reason to keep ignoring it.

Gotcha: route by *who can fix it*, not by who noticed. A proxy that
correctly reports "every upstream for service X is returning 500" should
page service X's owners — paging the proxy team, who can only forward the
message, adds a hop of human latency to every incident. This is why the
two SLIs above need to be separate metrics: they have different on-call
rotations.

## Practice
Build these in order.

1. Work through `08-observability/05-slo.md` first. **Done when** two SLIs
   and their targets exist as recording rules, and you can state which
   team each one pages.
2. Write the burn-rate ladder (14.4x/1h, 6x/6h, 1x/3d) with matching short
   windows, referencing the SLO target from a single recording rule.
   **Done when** changing the target in one place moves every threshold.
3. Add request-count floors. **Done when** a simulated 3am with 5 requests
   and 1 error does not page.
4. Add missing-data alerting and an external synthetic probe. **Done
   when** killing `proxy` outright pages within your target window —
   test this, because "the alert that fires when everything is dead" is
   the one most likely to be broken.
5. Verify the alerting path's independence. **Done when** you can state
   what your notification delivery depends on, and confirm none of it
   routes through the proxy being monitored.
6. Add ticket-level alerts for certificate expiry, config reload failure,
   and fd/pool saturation. **Done when** a cert 7 days from expiry and a
   deliberately broken config reload each produce a ticket without paging
   anyone.
7. Run a chaos test (`12-testing/03-chaos.md`) under load
   (`12-testing/01-load-testing.md`). **Done when** the burn-rate page fires
   within the expected window, the correct SLI moves (proxy vs end-to-end,
   depending on which fault you injected), and everything clears after the
   fault is removed.
8. Write the runbook for each page: what it means, first three checks,
   and how to mitigate. **Done when** the runbook link is in the alert
   definition itself and someone unfamiliar with the system could follow
   it.
