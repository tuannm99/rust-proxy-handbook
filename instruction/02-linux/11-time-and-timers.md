# Time and Timers: Clocks, Timeouts, and Why Wall-Clock Time Lies

What "now" means to a program, how the kernel delivers "wake me in 30 seconds," and
the clock mistakes that cause proxy bugs: timeouts that never fire, negative durations,
expired-before-issued tokens. Prerequisite for every timeout in [`06-proxy/`](../06-proxy)
and [`07-security/`](../07-security).

## What to learn

### Two clocks, two different jobs
Linux offers many clocks; two matter:

- **`CLOCK_REALTIME`** — wall-clock time (seconds since 1970-01-01 UTC). It answers "what time is it
  for a human," is what timestamps in logs, certificates and JWTs mean, and it **can jump**: NTP
  corrects it (small errors by **slewing** — running slightly faster or slower; large ones by
  **stepping**), an admin can set it, a VM restored from a snapshot wakes up hours behind, and
  **leap seconds** insert a second.
- **`CLOCK_MONOTONIC`** — time since an arbitrary point (boot), which only moves forward at a steady
  rate and never jumps. It answers "how much time *elapsed*." (`CLOCK_BOOTTIME` is the same but keeps
  counting through suspend.)

Rust maps them directly: `std::time::SystemTime` is realtime, `std::time::Instant` is monotonic.
**Measure durations and implement timeouts with `Instant`; use `SystemTime` only for timestamps shown to
humans or compared against other machines' wall-clock.** `SystemTime::duration_since(earlier)` returns an
`Err` if the clock went backwards — real production panics come from `.unwrap()` on it.

```rust
let start = std::time::Instant::now();
do_work().await;
let elapsed = start.elapsed();            // can't be negative; immune to NTP/steps
```

### Where wall-clock time legitimately matters
Some meanings *are* wall-clock: a TLS certificate's `notBefore`/`notAfter`
([`01-network/19-tls.md`](../01-network/19-tls.md)), a JWT's `exp`/`nbf` ([`07-security/02-jwt.md`](../07-security/02-jwt.md)), HTTP `Date`, `Expires`
and `Last-Modified` ([`01-network/15-http.md`](../01-network/15-http.md)). Different machines disagree by milliseconds to seconds,
so verifiers allow **clock skew** (a leeway of seconds–minutes) — a token "issued in the future" by a few seconds is
normal, not an attack. A proxy host with a badly drifting clock rejects valid certificates and tokens and
produces nonsense log ordering; run an NTP client (`chronyd`, `systemd-timesyncd`) and alert on drift.

### How a program waits: from `sleep` to `epoll_wait`
Waiting for time is just blocking with a deadline. `nanosleep` parks a thread until a time. An event
loop instead passes the **next deadline** as the *timeout* argument of `epoll_wait` ([`14-epoll.md`](14-epoll.md)): it wakes
when an fd is ready *or* when the earliest timer is due, whichever first. Alternatively **`timerfd`**
exposes a kernel timer as an fd that becomes readable on expiry, so timers join the same `epoll` set as
sockets ([`10-ipc.md`](10-ipc.md)). The kernel implements these on **high-resolution timers** (`hrtimers`),
sub-microsecond in principle, though real wake-ups have scheduling jitter of tens of microseconds to
milliseconds under load ([`12-cpu-scheduling.md`](12-cpu-scheduling.md)); a timer means "not before," never "exactly at."

### How tokio does timers
A runtime may have **tens of thousands of pending timeouts** (one per connection, per request). Asking the
kernel for each would be wasteful, so tokio keeps its own **hierarchical timer wheel** — an array of
buckets indexed by expiry time giving O(1) insert and cancel — and arms a **single** kernel wait for the
earliest deadline ([`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md); the wheel as a data structure is in
[`13-algorithms/priority-queue.md`](../13-algorithms/priority-queue.md)). Resolution is ~1 ms; `tokio::time::sleep(Duration::from_micros(10))` does
not give 10 µs. `tokio::time::timeout(d, fut)` races a future against a timer; when the timer wins the
future is **dropped** (cancelled) — so whatever it was doing stops at its next `.await`
([`04-runtime/04-structured-concurrency.md`](../04-runtime/04-structured-concurrency.md)). In tests, `tokio::time::pause()` replaces
the clock with a virtual one that auto-advances, making 30-second-timeout tests instant and deterministic.

### Timeouts are the proxy's central safety mechanism
Every wait a proxy performs needs a timeout, because a peer that never answers otherwise holds a
connection, a task and memory forever ([`07-security/10-slowloris.md`](../07-security/10-slowloris.md)):

- **connect** (the TCP handshake to an upstream),
- **TLS handshake**,
- **read headers / read body** (per-read idle, *and* a total cap),
- **upstream response** (time to first byte, then idle between chunks),
- **idle keep-alive** ([`05-http-stack/05-keepalive.md`](../05-http-stack/05-keepalive.md)),
- **total request deadline**.

Two designs matter. Distinguish an **idle timeout** (reset on each byte) from a **total deadline** (a hard
cap): idle alone lets a slow-drip client hold a connection indefinitely. And **propagate a deadline**:
if the client gave up after 5 s, an upstream call that can finish in 4 s *after* 3 s have passed is wasted;
convert the remaining budget into each hop's timeout, and don't retry past the deadline
([`06-proxy/05-retry.md`](../06-proxy/05-retry.md)). Timeouts also nest awkwardly: if the proxy's upstream timeout exceeds the
client's, the proxy works for a client that left ([`01-network/21-life-of-a-request.md`](../01-network/21-life-of-a-request.md)).

### Reading the time is nearly free: vDSO
`clock_gettime` is called constantly (every log line, every `Instant::now()`). Linux maps a small kernel-provided
page, the **vDSO**, into each process so the call reads the clock **without a syscall** — tens of nanoseconds.
(Some virtualized clock sources, e.g. certain Xen setups, disable this and make `Instant::now()` a real syscall;
`strace` showing endless `clock_gettime` is the tell.) Still, reading the time per byte or per tiny event is
waste: read it once per batch.

### Expiring things: TTLs and caches
Cache entries, DNS answers, rate-limit windows, circuit-breaker cooldowns, health-check intervals and pool idle
reaping all expire. Store an **expiry `Instant`** (monotonic), not a `SystemTime`, so a clock step can't make an
entry live forever or die instantly, and prefer *lazy expiry on access plus a periodic sweep* over one timer per
entry ([`05-http-stack/08-cache.md`](../05-http-stack/08-cache.md), [`06-proxy/06-circuit-breaker.md`](../06-proxy/06-circuit-breaker.md)). **Jitter** TTLs
(randomize ±10%) so entries created together don't all expire together and stampede the upstream
([`05-http-stack/09-cache-stampede.md`](../05-http-stack/09-cache-stampede.md)).

### Gotcha: sleeping a thread inside async code
`std::thread::sleep` in an async task blocks the *whole worker thread* (no `.await` yield), starving every other task
on it — and the symptom is timeouts firing late everywhere. Use `tokio::time::sleep`. The same applies to any
blocking call; see [`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md).

## Practice

1. Print `date +%s`, `cat /proc/uptime`, and `chronyc tracking` (or `timedatectl timesync-status`); explain which of these
   is realtime and which monotonic. Write a scratch Rust program that prints `SystemTime::now()` and `Instant::now()`
   deltas every second, then `sudo date -s '-1 hour'` while it runs and show which clock jumped and what
   `duration_since` returns.
2. `strace -f -c` a small loop calling `Instant::now()` 10 million times and confirm `clock_gettime` doesn't appear
   (vDSO); then time the loop to get ns per call.
3. Write a scratch tokio program with 100,000 `sleep(10s)` tasks and measure its CPU and RSS; then start `strace -f -c`
   and confirm the syscall count is tiny, i.e. the timer wheel multiplexes them.
4. Reproduce the blocking-sleep bug: spawn many tasks that `std::thread::sleep(50ms)` vs `tokio::time::sleep(50ms)` on a
   single-worker runtime (`flavor = "current_thread"`) and measure how late other timers fire.
5. In [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), add connect, upstream-response and total-request timeouts, then verify each
   with a deliberately slow upstream (`nc -l` that never replies, or a handler that `sleep`s) that the right status
   (`504`, [`01-network/15-http.md`](../01-network/15-http.md)) is returned and the connection is closed, and with a slow-drip client
   that only a total deadline stops it.
6. Use `tokio::time::pause()`/`advance` to unit-test a TTL cache's expiry and a retry backoff without real sleeping.
