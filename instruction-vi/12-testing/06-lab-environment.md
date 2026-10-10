# Lab Environment

Các công cụ bên ngoài mà phần kiểm tra `Done when` của các lab dùng tới, và
cách cài đặt, sử dụng từng cái trên Ubuntu / WSL2. Các file khác trong thư
mục này giải thích *đo cái gì*. File này là phần setup, để một lab không bao
giờ bị kẹt ở câu "chạy cái đó kiểu gì". Chỗ nào setup của một công cụ thuộc
về một chủ đề cụ thể, file này trỏ tới đó thay vì lặp lại.

## What to learn

### Cài một lần
```sh
sudo apt install -y curl netcat-openbsd iproute2 tcpdump strace \
  wrk nghttp2-client jq python3
cargo install oha tokio-console cargo-fuzz
```

Như vậy là có `curl`, `nc`, `ss`, `tcpdump`, `strace` ([`12-testing/05-debugging.md`](05-debugging.md)),
`wrk`, `h2load`/`nghttp` (từ `nghttp2-client`), và `oha`. `vegeta` không có
trong apt: tải binary release từ trang GitHub của nó (`tsenart/vegeta`),
hoặc `go install github.com/tsenart/vegeta/v12@latest` nếu bạn có Go. Docker
chạy Prometheus và Jaeger ở phần dưới. Với Docker Engine cài ngay bên trong
WSL, `--network host` hoạt động như trên Linux. Với Docker Desktop trên
Windows, dùng `host.docker.internal` ở bất cứ chỗ nào một container cần
connect tới một process trong WSL.

### Công cụ tạo tải: dùng cái nào khi nào
- **`oha`** là lựa chọn hằng ngày. Nó in histogram latency và các
  percentile, nói được HTTP/1.1 và HTTP/2, và giữ được một tốc độ cố định:
  `oha -z 30s -c 50 http://127.0.0.1:8080/` (30 giây, 50 connection, nhanh
  hết mức) hoặc `oha -z 30s -q 500 http://...` (500 request mỗi giây, gần với
  open-loop, xem [`12-testing/01-load-testing.md`](01-load-testing.md)). `--http2` đổi protocol và
  `--insecure` chấp nhận certificate tự ký của bạn.
- **`wrk`** cho throughput thô cao nhất với HTTP/1.1, và các hook Lua của
  nó có thể dựng request tùy biến: `wrk -t4 -c100 -d30s http://...`.
- **`vegeta`** là open-loop theo thiết kế và đọc một file danh sách target,
  đó là cách bạn phát lại một tổ hợp request cụ thể:
  `vegeta attack -targets=targets.txt -rate=500 -duration=60s | vegeta report`.
- **`h2load`** là công cụ tạo tải HTTP/2, và là cái cho thấy multiplexing:
  `h2load -n 10000 -c 1 -m 100 http://127.0.0.1:8080/` gửi 10.000 request
  qua **một** connection với tối đa 100 stream đồng thời. Với URL `http://`
  nó nói HTTP/2 không mã hóa trực tiếp (prior knowledge), đúng thứ mà auto
  builder của hyper mong đợi ([`05-http-stack/02-hyper.md`](../05-http-stack/02-hyper.md)). `nghttp -nv http://...`
  in ra từng frame của một lượt trao đổi (`SETTINGS`, `HEADERS`, `DATA`,
  `WINDOW_UPDATE`, `GOAWAY`), đó là cách nhanh nhất để *thấy* server của bạn
  đã gửi gì.

Gotcha: không công cụ nào ở trên gửi được HTTP/2 Rapid Reset
([`01-network/17-http2.md`](../01-network/17-http2.md)). Với [`labs/08-http2`](../../labs/08-http2) bạn tự viết client đó, và
hyper làm nó ngắn gọn. Với HTTP/2 client của hyper, drop một response future
trước khi response tới khiến crate `h2` bên dưới gửi `RST_STREAM(CANCEL)`
cho stream đó. Nên "mở một stream, drop nó, lặp lại" trong một vòng lặp
*chính là* cuộc tấn công.

### Tải lệch: phân phối key Zipf
Tỉ lệ hit của cache ([`labs/10-cache`](../../labs/10-cache)) và độ xáo trộn key của rate limit
([`labs/11-rate-limit`](../../labs/11-rate-limit)) là vô nghĩa với key ngẫu nhiên đều. Traffic thật bị
lệch: vài key rất nóng, phần lớn là nguội. Mô hình chuẩn là Zipf, trong đó
key phổ biến thứ k nhận traffic tỉ lệ với `1/k^s` (`s` ≈ 1). Tạo một file
target của vegeta với phân phối đó, rồi phát lại nó:

```python
# zipf_targets.py: ghi 200k request trên 10k key, Zipf s=1.1
import random
keys, n, s = 10_000, 200_000, 1.1
weights = [1 / (k ** s) for k in range(1, keys + 1)]
with open("targets.txt", "w") as f:
    for k in random.choices(range(keys), weights=weights, k=n):
        f.write(f"GET http://127.0.0.1:8080/item/{k}\n\n")
```

`vegeta attack` đi vòng qua file theo thứ tự, nên chính file mang phân
phối. Đổi `s` để thấy độ lệch thay đổi tỉ lệ hit ra sao. Đó chính là thứ
phân biệt LRU với TinyLFU ([`13-algorithms/tinylfu.md`](../13-algorithms/tinylfu.md)).

### Benchmark với criterion
[`03-rust/16-testing-idioms.md`](../03-rust/16-testing-idioms.md) giải thích vì sao dùng `criterion`. Phần setup:
`criterion` nằm trong `[dev-dependencies]` (lab 03, 06, 12 và 14 đã có
sẵn). Thêm một bảng `[[bench]]` với
`name = "lookup"` và `harness = false` vào `Cargo.toml` của crate, và viết
`benches/lookup.rs` dùng `criterion_group!`/`criterion_main!`. Chạy
`cargo bench -p router`. Một bench, giống một fuzz target, nằm ngoài crate
của bạn và chỉ gọi được API **library** của nó, nên code nằm trong
`src/main.rs` phải chuyển sang `src/lib.rs` trước. Để chứng minh "chi phí
lookup không tăng theo số route" ([`labs/03-router`](../../labs/03-router)), benchmark cùng một phép
lookup trên bảng 10 route và 1000 route trong một group rồi so sánh.

### Đo một process: memory và thread
Nhiều lab kiểm tra memory giữ phẳng hoặc số thread. Cả hai nằm trong
`/proc`:

```sh
pid=$(pgrep -n static-server)     # binary được đặt tên theo package
grep -E 'VmRSS|Threads' /proc/$pid/status
watch -n1 "grep -E 'VmRSS|Threads' /proc/$pid/status"   # trực tiếp, trong lúc load test
```

`VmRSS` là memory thường trú. Ghi lại lúc rảnh, rồi trong lúc test. "Phẳng"
nghĩa là nó nằm trong khoảng vài chục MB so với lúc rảnh trong khi lượng dữ
liệu truyền lớn hơn thế rất nhiều.

### Prometheus và promtool
Cho [`labs/15-prometheus`](../../labs/15-prometheus), với `/metrics` của bạn ở port 9000:

```yaml
# prometheus.yml
global: { scrape_interval: 5s }
scrape_configs:
  - job_name: lab
    static_configs:
      - targets: ["localhost:9000"]   # host.docker.internal:9000 với Docker Desktop
```

```sh
docker run --rm -d --name prom --network host \
  -v "$PWD/prometheus.yml:/etc/prometheus/prometheus.yml" prom/prometheus
# lint định dạng exposition bằng promtool có sẵn trong cùng image:
curl -s localhost:9000/metrics | docker run --rm -i --entrypoint promtool prom/prometheus check metrics
```

UI nằm ở `http://localhost:9090`. **Status → Targets** phải hiện endpoint
của bạn là `UP` trước khi bất kỳ query nào trả dữ liệu. p99 từ một histogram
là `histogram_quantile(0.99, sum by (le) (rate(<name>_bucket[1m])))`, trong
đó `<name>` là tên histogram của bạn ([`08-observability/02-metrics.md`](../08-observability/02-metrics.md) giải
thích vì sao `sum by (le)` là bắt buộc).

### Các setup được ghi ở nơi khác
- Toolchain và lệnh fuzzing: [`12-testing/02-fuzzing.md`](02-fuzzing.md).
- CA local và certificate, `openssl s_client`: [`01-network/19-tls.md`](../01-network/19-tls.md).
- Jaeger cho trace: [`08-observability/03-tracing.md`](../08-observability/03-tracing.md).
- `tokio-console` (cần crate `console-subscriber` và
  `RUSTFLAGS="--cfg tokio_unstable"`): [`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md).
- Toolchain Aya, network namespace và `veth`: [`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md).

## Practice
1. Cài mọi thứ trong "Cài một lần" và chạy `--version` (hoặc `-h`) của từng công cụ. **Done when** `oha`, `wrk`, `h2load`, `nghttp`, `vegeta` và `tokio-console` đều phản hồi.
2. Chạy `oha -z 10s -c 20` nhắm vào [`labs/02-http-server`](../../labs/02-http-server), rồi `h2load -n 2000 -c 1 -m 50` nhắm vào cùng server, và dùng `ss -tn` trong lúc chạy mỗi cái để đếm connection. **Done when** bạn nói được mỗi bài test đã dùng bao nhiêu TCP connection và vì sao.
3. Tạo `targets.txt` bằng script Zipf, và đếm số lần 10 key đầu xuất hiện (`sort | uniq -c | sort -rn | head`). **Done when** bạn nêu được bao nhiêu phần request rơi vào 1% key hàng đầu.
4. Khởi động Prometheus với config ở trên nhắm vào một endpoint `/metrics` bất kỳ và đưa target lên `UP`. **Done when** một query cho một metric của bạn trả về dữ liệu.
