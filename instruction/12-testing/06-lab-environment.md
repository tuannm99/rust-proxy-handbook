# Lab Environment

The external tools the labs' `Done when` checks use, and how to install
and drive each one on Ubuntu / WSL2. The other files in this directory
explain *what* to measure. This one is the setup, so a lab never stalls on
"how do I even run that". Where a tool's setup belongs to a specific
topic, this file points there instead of repeating it.

## What to learn

### Install once
```sh
sudo apt install -y curl netcat-openbsd iproute2 tcpdump strace \
  wrk nghttp2-client jq python3
cargo install oha tokio-console cargo-fuzz
```

That covers `curl`, `nc`, `ss`, `tcpdump`, `strace` ([`12-testing/05-debugging.md`](05-debugging.md)),
`wrk`, `h2load`/`nghttp` (from `nghttp2-client`), and `oha`. `vegeta` isn't
in apt: download a release binary from its GitHub page (`tsenart/vegeta`),
or `go install github.com/tsenart/vegeta/v12@latest` if you have Go. Docker
runs Prometheus and Jaeger below. With Docker Engine installed inside WSL
itself, `--network host` works as on Linux. With Docker Desktop on Windows,
use `host.docker.internal` wherever a container must reach a process in
WSL.

### Load generators: which one when
- **`oha`** is the everyday choice. It prints a latency histogram and
  percentiles, speaks HTTP/1.1 and HTTP/2, and can hold a fixed rate:
  `oha -z 30s -c 50 http://127.0.0.1:8080/` (30 s, 50 connections, as fast
  as possible) or `oha -z 30s -q 500 http://...` (500 requests per second,
  close to open-loop, see [`12-testing/01-load-testing.md`](01-load-testing.md)). `--http2` switches
  protocol and `--insecure` accepts your self-signed certificate.
- **`wrk`** gives the highest raw throughput for HTTP/1.1, and its Lua
  hooks can build custom requests: `wrk -t4 -c100 -d30s http://...`.
- **`vegeta`** is open-loop by design and reads a file of targets, which is
  how you replay a specific request mix:
  `vegeta attack -targets=targets.txt -rate=500 -duration=60s | vegeta report`.
- **`h2load`** is the HTTP/2 generator, and the one that shows
  multiplexing: `h2load -n 10000 -c 1 -m 100 http://127.0.0.1:8080/` sends
  10,000 requests over **one** connection with up to 100 concurrent
  streams. For `http://` URLs it speaks cleartext HTTP/2 directly (prior
  knowledge), which is what hyper's auto builder expects
  ([`05-http-stack/02-hyper.md`](../05-http-stack/02-hyper.md)). `nghttp -nv http://...` prints every frame
  of one exchange (`SETTINGS`, `HEADERS`, `DATA`, `WINDOW_UPDATE`,
  `GOAWAY`), which is the quickest way to *see* what your server sent.

Gotcha: none of these can send an HTTP/2 Rapid Reset
([`01-network/12-http2.md`](../01-network/12-http2.md)). For [`labs/08-http2`](../../labs/08-http2) you write that client
yourself, and hyper makes it short. With hyper's HTTP/2 client, dropping a
response future before the response arrives makes the underlying `h2`
crate send `RST_STREAM(CANCEL)` for that stream. So "open a stream, drop
it, repeat" in a loop *is* the attack.

### Skewed load: a Zipf key distribution
Cache hit ratios ([`labs/10-cache`](../../labs/10-cache)) and rate-limit key churn
([`labs/11-rate-limit`](../../labs/11-rate-limit)) are meaningless under uniform random keys. Real
traffic is skewed: a few keys are very hot, most are cold. The standard
model is Zipf, where the k-th most popular key gets traffic proportional
to `1/k^s` (`s` ≈ 1). Generate a vegeta targets file with that
distribution, then replay it:

```python
# zipf_targets.py: write 200k requests over 10k keys, Zipf s=1.1
import random
keys, n, s = 10_000, 200_000, 1.1
weights = [1 / (k ** s) for k in range(1, keys + 1)]
with open("targets.txt", "w") as f:
    for k in random.choices(range(keys), weights=weights, k=n):
        f.write(f"GET http://127.0.0.1:8080/item/{k}\n\n")
```

`vegeta attack` cycles through the file in order, so the file itself
carries the distribution. Change `s` to see how skew moves the hit ratio.
It is exactly what separates LRU from TinyLFU ([`13-algorithms/tinylfu.md`](../13-algorithms/tinylfu.md)).

### Benchmarks with criterion
[`03-rust/16-testing-idioms.md`](../03-rust/16-testing-idioms.md) explains why to use `criterion`. The setup:
`criterion` goes under `[dev-dependencies]` (labs 03, 06, 12 and 14 already
have it). Add a `[[bench]]` table with
`name = "lookup"` and `harness = false` to the crate's `Cargo.toml`, and
write `benches/lookup.rs` using `criterion_group!`/`criterion_main!`. Run
`cargo bench -p router`. A bench, like a fuzz target, is outside your
crate and can only call its **library** API, so code that lives in
`src/main.rs` has to move to `src/lib.rs` first. To show "lookup cost
doesn't grow with route count" ([`labs/03-router`](../../labs/03-router)), benchmark the same lookup
against tables of 10 and 1000 routes in one group and compare.

### Measuring a process: memory and threads
Several labs check memory staying flat or a thread count. Both are in
`/proc`:

```sh
pid=$(pgrep -n static-server)     # the binary is named after the package
grep -E 'VmRSS|Threads' /proc/$pid/status
watch -n1 "grep -E 'VmRSS|Threads' /proc/$pid/status"   # live, during a load test
```

`VmRSS` is resident memory. Record it idle, then during the test. "Flat"
means it stays within a few tens of MB of idle while the transfer is far
larger than that.

### Prometheus and promtool
For [`labs/15-prometheus`](../../labs/15-prometheus), with your `/metrics` on port 9000:

```yaml
# prometheus.yml
global: { scrape_interval: 5s }
scrape_configs:
  - job_name: lab
    static_configs:
      - targets: ["localhost:9000"]   # host.docker.internal:9000 on Docker Desktop
```

```sh
docker run --rm -d --name prom --network host \
  -v "$PWD/prometheus.yml:/etc/prometheus/prometheus.yml" prom/prometheus
# lint your exposition format with the promtool inside the same image:
curl -s localhost:9000/metrics | docker run --rm -i --entrypoint promtool prom/prometheus check metrics
```

The UI is at `http://localhost:9090`. **Status → Targets** must show your
endpoint as `UP` before any query returns data. A p99 from a histogram is
`histogram_quantile(0.99, sum by (le) (rate(<name>_bucket[1m])))`, where
`<name>` is your histogram's name ([`08-observability/02-metrics.md`](../08-observability/02-metrics.md) explains
why the `sum by (le)` is required).

### Setups documented elsewhere
- Fuzzing toolchain and commands: [`12-testing/02-fuzzing.md`](02-fuzzing.md).
- Local CA and certificates, `openssl s_client`: [`01-network/14-tls.md`](../01-network/14-tls.md).
- Jaeger for traces: [`08-observability/03-tracing.md`](../08-observability/03-tracing.md).
- `tokio-console` (needs the `console-subscriber` crate and
  `RUSTFLAGS="--cfg tokio_unstable"`): [`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md).
- Aya toolchain, network namespaces and `veth`: [`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md).

## Practice
1. Install everything in "Install once" and run each tool's `--version` (or `-h`). **Done when** `oha`, `wrk`, `h2load`, `nghttp`, `vegeta` and `tokio-console` all respond.
2. Run `oha -z 10s -c 20` against [`labs/02-http-server`](../../labs/02-http-server), then `h2load -n 2000 -c 1 -m 50` against the same server, and use `ss -tn` during each to count connections. **Done when** you can say how many TCP connections each test used and why.
3. Generate `targets.txt` with the Zipf script, and count how often the top 10 keys appear (`sort | uniq -c | sort -rn | head`). **Done when** you can state what share of requests hit the top 1% of keys.
4. Start Prometheus with the config above against any `/metrics` endpoint and get the target to `UP`. **Done when** a query for one of your metrics returns data.
