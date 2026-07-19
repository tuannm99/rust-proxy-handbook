# Alerting & SLOs

What actually pages a human, vs what's just a line on a dashboard.

## What to learn
### SLI, SLO, and error budget
An **SLI** (service level indicator) is a measured ratio, e.g. `successful_requests / total_requests` or the fraction of requests under 300ms. An **SLO** is a target for that SLI over a window, e.g. "99.9% of requests succeed over 30 days." The **error budget** is `1 - SLO`: over 30 days at 99.9%, you're allowed ~43 minutes of full downtime (or an equivalent smear of partial failures) before you've spent the whole budget. The budget reframes reliability work as a resource to spend deliberately, not an abstract goal — burn it responding to a real incident, not by over-alerting on noise.

### Alert on symptoms, not causes
Page on what the *user* experiences (elevated error rate, elevated p99 latency, the proxy itself down) — not on every internal condition that *might* cause a symptom (one of three upstreams unhealthy, one retry occurred, GC pause of 50ms). If the load balancer in `06-proxy/load-balancer.md` and health checks in `06-proxy/healthcheck.md` are doing their job, losing one upstream shouldn't page anyone; losing all of them should. Cause-level signals still matter — keep them as metrics/dashboards (`08-observability/metrics.md`) for root-causing an incident after the symptom-level alert already woke someone up.

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

### Alert fatigue and routing
Every alert needs an owner, a severity, and a runbook link — an alert nobody can act on at 3am just trains people to ignore pages. Route by severity: page (wakes someone up, symptom-level, budget-burning), ticket (business-hours, cause-level, not yet user-visible), and metric-only (no notification, dashboard/debugging aid only). If a page fires and the response is always "nothing to do, it self-resolved" more than a couple of times, that's a signal to move it down a tier, not a reason to keep ignoring it.

## Practice
1. Define one SLO for `proxy` (e.g. 99.9% non-5xx responses over 30 days) built on the request counter from `08-observability/metrics.md`.
2. Write the PromQL recording rule for the error-budget burn rate, and a two-window (5m/1h) alert rule that only fires when both windows exceed the threshold.
3. Add a request-count floor to the alert so low-traffic periods don't produce false positives from small-sample noise.
4. Load-test the proxy (`12-testing/load-testing.md`) while injecting failures via `12-testing/chaos.md`, and confirm the burn-rate alert fires within the expected window — then confirm it clears once the fault is removed.
5. Write a one-paragraph runbook for the alert (what it means, first three things to check) and link it from the alert definition itself.
