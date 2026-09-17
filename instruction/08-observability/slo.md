# SLIs, SLOs, and Error Budgets

Defining what "working" means, numerically, before deciding what should
wake someone up. `08-observability/alerting.md` builds on this — an alert
without an SLO behind it is a threshold somebody guessed.

## What to learn
### The three terms
An **SLI** (service level indicator) is a measured ratio, e.g.
`successful_requests / total_requests` or the fraction of requests under
300ms. An **SLO** is a target for that SLI over a window, e.g. "99.9% of
requests succeed over 30 days." The **error budget** is `1 - SLO`: over 30
days at 99.9%, you're allowed ~43 minutes of full downtime (or an
equivalent smear of partial failures) before you've spent the whole
budget.

The budget reframes reliability as a resource to spend deliberately rather
than an abstract goal — burn it responding to a real incident, not by
over-alerting on noise. It also makes the tradeoff explicit: a team with
budget remaining can ship faster; one that has spent it should be fixing
reliability instead.

### Write it as good events over valid events
Defining the SLI precisely is most of the work, and the definition has to
survive an argument during an incident. Pin down both halves:

- **Are 4xx errors yours?** Usually not — a client sending malformed
  requests shouldn't burn your budget. But a 429 you emitted because you
  were overloaded (`07-security/load-shedding.md`) *is* your failure
  wearing a client-error status code, and belongs in the numerator.
- **Is a slow success a success?** For a latency SLO, no — define "good"
  as "succeeded *and* under threshold", with the threshold matching a
  histogram bucket edge (`08-observability/metrics.md`) so it's an exact
  count rather than an interpolation.
- **Which requests are valid?** Health checks, synthetic probes, and
  scrapes of `/metrics` should be excluded, or a quiet night of nothing
  but health checks reports 100% availability.

Gotcha: write the definition down as a recording rule, not as prose in a
document. The query *is* the definition; anything else drifts from it.

### A proxy needs at least two SLIs
"The proxy did its job" (it routed, it didn't 5xx on its own account) and
"the end-to-end request succeeded" (which includes the upstream's
failures) are different numbers with different owners. Conflating them
makes the error budget useless: the proxy team burns budget for an
upstream's bad deploy and has nothing to fix.

Measure both. Page the proxy team on the first
(`08-observability/alerting.md` covers routing by who can act). The second
is still worth tracking — it's what users actually experience — but its
owner is the service behind you.

Gotcha: attributing a failure to the right side is not always obvious. A
504 because the upstream exceeded the proxy's timeout could be an upstream
problem or a timeout you set too aggressively (`06-proxy/upstream.md`).
Decide the attribution rule in advance and encode it in the recording
rule, or every incident starts with the same argument.

### Choosing the target
A target is a business decision constrained by physics, and both halves
matter:
- **Higher isn't automatically better.** Each additional nine costs
  roughly an order of magnitude more effort, and past a point the
  dominant failure is something you don't control — the client's network,
  DNS, the internet.
- **You cannot exceed your dependencies.** A proxy fronting a 99.9%
  upstream cannot offer 99.99% end-to-end, unless it can serve without
  that upstream (`05-http-stack/cache.md`'s `stale-if-error` is exactly
  this kind of decoupling).
- **The window matters as much as the number.** 99.9% over 30 days is 43
  minutes; over 7 days it's 10 minutes, and a single bad deploy can spend
  the shorter budget entirely.

Gotcha: set the *internal* SLO tighter than any externally promised SLA,
so you start reacting before you owe anyone anything.

### The budget is the thing that drives action
Two derived signals do the actual work:
- **Budget remaining.** A gauge anyone can look at. When it's healthy,
  ship; when it's exhausted, the team's priority shifts to reliability
  until it recovers. That policy has to be agreed in advance to mean
  anything.
- **Burn rate** — how fast you're consuming budget relative to the
  sustainable rate. This is what alerts fire on, because it catches both
  "an outage right now" and "a slow leak that will spend the month" with
  one mechanism (`08-observability/alerting.md`).

Gotcha: keep the SLO target in *one* recording rule that everything else
references. Hard-coding `0.001` in five alert rules means changing the SLO
silently leaves four of them enforcing the old one.

## Practice
Build these in order.

1. Define two SLIs for `proxy` — proxy-caused failures and end-to-end
   failures — as recording rules over the counters from
   `08-observability/metrics.md`. **Done when** each has explicit handling
   for 4xx, for 429-under-load, and for excluded health-check traffic, and
   you can name the team each one belongs to.
2. Write the attribution rule for 504s. **Done when** a timeout is
   attributed to one side deliberately and the reasoning is in a comment
   next to the rule.
3. Add a latency SLI whose threshold matches a histogram bucket edge.
   **Done when** "fraction under threshold" is an exact bucket ratio
   rather than a `histogram_quantile` interpolation — adjust your buckets
   if they don't line up.
4. Pick targets and windows, and compute the budgets. **Done when** you
   can state, in minutes, how much downtime each SLO allows per window,
   and have sanity-checked it against your upstream's own reliability.
5. Publish budget-remaining as a gauge. **Done when** a dashboard shows
   how much is left in the current window without anyone running a query
   by hand.
6. Put the target in exactly one recording rule. **Done when** changing it
   in that one place moves every derived signal — verify by changing it
   and watching them all move.
7. Validate against reality. **Done when** you replay a past incident (or
   inject one with `12-testing/chaos.md`) and confirm the budget consumed
   matches the incident's actual duration and severity.
