# Chạy như một Service: systemd, Supervision và Hợp đồng của Process

File này trả lời: một proxy production thực sự được khởi động, restart, gửi signal, giới hạn tài nguyên và quan sát
như thế nào trên một Linux host. Nó cũng nói về hợp đồng giữa supervisor và process của bạn: signal đi vào, exit code
và readiness đi ra. File này nối [`04-process-lifecycle.md`](04-process-lifecycle.md),
[`17-signals.md`](17-signals.md), [`20-limits-and-proc.md`](20-limits-and-proc.md) và
[`13-containers.md`](13-containers.md) lại với nhau.

## What to learn

### Supervisor làm gì
**Supervisor** là chương trình khởi động và trông coi process của bạn. Trên hầu hết distribution đó là systemd. Trong
container thì là container runtime, và trong Kubernetes là kubelet. Supervisor:

- Khởi động process trong một môi trường được kiểm soát: user, working directory, environment, limit, cgroup.
- Theo dõi process, và restart nếu nó chết.
- Gửi signal để dừng nó.
- Thu thập output của nó.

Việc của process bạn là cư xử có thể đoán trước trong hợp đồng đó, thay vì tự daemonize: **chạy ở foreground, log ra
stdout/stderr, xử lý `SIGTERM`, và thoát với exit code có ý nghĩa**
([`04-process-lifecycle.md`](04-process-lifecycle.md),
[`08-observability/01-logging.md`](../08-observability/01-logging.md)). systemd là PID 1 trên host, nên nó reap
orphan và sở hữu cgroup của mọi service.

### Một unit file
```ini
# /etc/systemd/system/proxy.service
# (systemd không cho phép comment ở cuối dòng, nên mỗi comment một dòng riêng)
[Unit]
Description=L7 reverse proxy
After=network-online.target
Wants=network-online.target

[Service]
# "notify" tốt hơn khi proxy đã báo readiness; xem Type=notify bên dưới
Type=simple
ExecStart=/usr/local/bin/proxy --config /etc/proxy/proxy.toml
# hợp đồng reload: SIGHUP đọc lại config
ExecReload=/bin/kill -HUP $MAINPID
User=proxy
Group=proxy
Restart=on-failure
RestartSec=1
# nâng trần fd (xem 20-limits-and-proc.md)
LimitNOFILE=1048576
# bind :443 mà không cần là root
AmbientCapabilities=CAP_NET_BIND_SERVICE
NoNewPrivileges=yes
# graceful shutdown được phép mất bao lâu trước khi bị SIGKILL
TimeoutStopSec=30

[Install]
WantedBy=multi-user.target
```
Các lệnh hay dùng:
- `systemctl daemon-reload` (sau khi sửa unit file).
- `systemctl start|stop|restart|reload|status proxy`.
- `systemctl enable` (tự khởi động cùng lúc boot).
- `journalctl -u proxy -f` (xem log).

`ExecStart` phải gọi trực tiếp tên binary. Nếu nó bị bọc trong `sh -c`, signal sẽ đi tới shell
([`04-process-lifecycle.md`](04-process-lifecycle.md)).

### Hợp đồng dừng: SIGTERM, timeout, SIGKILL
`systemctl stop` gửi **`SIGTERM`** tới main process của service, chờ tối đa `TimeoutStopSec`, rồi gửi **`SIGKILL`**
cho mọi thứ còn lại trong cgroup.

Vì vậy khi nhận `SIGTERM`, proxy của bạn phải:
1. Ngừng nhận connection mới.
2. Để các request đang xử lý chạy xong (có giới hạn bởi một deadline).
3. Thoát với mã 0.

([`17-signals.md`](17-signals.md),
[`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md)).

Hãy đặt `TimeoutStopSec` dài hơn deadline của việc drain. Nếu không, một lần drain chậm sẽ bị `SIGKILL` cắt ngang, và
exit status 137 trong log là manh mối ([`04-process-lifecycle.md`](04-process-lifecycle.md)).

Kubernetes có cùng hình dạng: `SIGTERM`, rồi `terminationGracePeriodSeconds`, rồi `SIGKILL`. Thêm vào đó có hook
`preStop`, và việc gỡ pod khỏi endpoint của load balancer diễn ra *đồng thời* với signal. Vì vậy proxy thường cần
tiếp tục phục vụ thêm một lúc ngắn sau `SIGTERM`
([`09-architecture/06-canary-deploy.md`](../09-architecture/06-canary-deploy.md)).

`ExecReload=` là hợp đồng reload: `SIGHUP` nghĩa là đọc lại config mà không làm rơi connection
([`09-architecture/03-config.md`](../09-architecture/03-config.md)).

### Chính sách restart và crash loop
- `Restart=on-failure`: restart khi exit code khác 0, hoặc khi bị một signal giết.
- `Restart=always`: restart cả khi thoát sạch.
- `RestartSec`: độ trễ trước khi restart.

Mặc định, systemd bỏ cuộc sau vài lần restart trong một khoảng thời gian ngắn
(`StartLimitBurst`/`StartLimitIntervalSec`).

Một proxy **crash vì config sai** sẽ bị restart vào đúng cái crash đó. Vì vậy: *hãy fail nhanh khi config không hợp
lệ lúc khởi động, kèm một thông báo rõ ràng và một exit code riêng.* Khi reload, hãy validate config mới **trước khi**
thay thế config cũ, để một lỗi gõ nhầm không làm sập service
([`09-architecture/03-config.md`](../09-architecture/03-config.md)). Hãy cảnh báo theo số lần restart, không chỉ theo
"process có đang chạy không" ([`08-observability/06-alerting.md`](../08-observability/06-alerting.md)).

### Type=notify: readiness, không chỉ là "đã khởi động"
Với `Type=simple`, systemd coi service là đã khởi động ngay khi process được fork, tức là trước cả khi nó bind port
hay load config. Các service phụ thuộc và health gate khi đó sẽ chạy quá sớm.

`Type=notify` sửa việc này. Process gửi `READY=1` qua socket có tên trong `$NOTIFY_SOCKET` (qua `sd_notify`, hoặc
crate `sd-notify`) *sau khi* nó thực sự sẵn sàng. Nó cũng có thể gửi `RELOADING=1`/`STOPPING=1`.

Sự phân biệt này cũng tồn tại trong Kubernetes, dưới dạng **readiness probe** (khác với việc process chỉ đang chạy)
([`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md)). Hãy tránh `Type=forking` (kiểu daemonize cũ) cho phần
mềm mới.

### Socket activation: systemd sở hữu listening socket
Một unit `.socket` cho phép **systemd bind port** và đưa cho process của bạn fd đã listen sẵn. fd được truyền ở vị trí
số 3, với `$LISTEN_FDS`/`$LISTEN_PID` được set. Các crate `listenfd` hoặc `sd-listen-fds` đọc nó. Lợi ích:

- Port luôn mở, và **connection được xếp hàng trong kernel backlog khi service restart**. Đây là một cách restart không
  downtime đơn giản ([`01-network/11-socket.md`](../01-network/11-socket.md)).
- Service có thể dùng port 443 mà chạy không đặc quyền, vì nó không bao giờ tự gọi `bind`
  ([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)).
- Thứ tự khởi động không còn quan trọng.

Nó dùng cùng cơ chế thừa kế fd như hot restart ([`10-ipc.md`](10-ipc.md),
[`09-architecture/05-rolling-restart.md`](../09-architecture/05-rolling-restart.md)): một listening fd sống lâu hơn bất
kỳ process nào.

### cgroup cho từng service: limit và accounting có sẵn
systemd đặt mỗi service vào một cgroup riêng ([`13-containers.md`](13-containers.md)). Vì vậy `systemctl status proxy`
liệt kê mọi process của service (kể cả child). Các cài đặt trong unit ánh xạ thành limit của cgroup: `MemoryMax=`,
`CPUQuota=200%`, `TasksMax=`, `IOWeight=`.

`systemd-cgls` hiện cây process. `systemd-cgtop` hiện mức dùng tài nguyên trực tiếp. Hãy cẩn thận với `CPUQuota`: nó là
một trần có throttle, với ảnh hưởng đến latency đã mô tả ở [`12-cpu-scheduling.md`](12-cpu-scheduling.md). `MemoryMax`
kích hoạt cgroup OOM killer ([`16-memory.md`](16-memory.md)).

### Hardening bằng cấu hình
Chỉ với khoảng một tá dòng trong unit, bạn có sandboxing mà không cần viết code:

- `NoNewPrivileges=yes`
- `ProtectSystem=strict` (cây thư mục của OS ở chế độ read-only)
- `ProtectHome=yes`
- `PrivateTmp=yes`
- `ReadWritePaths=/var/log/proxy`
- `CapabilityBoundingSet=CAP_NET_BIND_SERVICE`
- `SystemCallFilter=@system-service` (một allow-list seccomp,
  [`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md))
- `RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX`

`systemd-analyze security proxy` chấm điểm một unit và liệt kê những gì còn để hở. Nếu hardening quá chặt, bạn sẽ thấy
`EACCES`/`EPERM` trong `strace`.

### Log: journald và stdout
Mọi thứ ghi ra stdout/stderr đều vào journal, được gắn tag theo unit kèm timestamp.
`journalctl -u proxy --since '10 min ago' -o json` cho bạn truy xuất có cấu trúc.

Ngược lại, logging ra file đòi hỏi bạn phải xử lý rotation từ bên ngoài process
([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)). Logging ra stdout cũng là hợp đồng mà container dùng.

### Gotcha: môi trường bạn test không phải môi trường nó chạy
Dưới một supervisor, proxy chạy với `PATH` tối thiểu, không có TTY, working directory khác, không có file khởi động
của shell, limit khác, user khác, và có thể filesystem chỉ đọc. Khi gặp "chạy được lúc tôi chạy tay" nhưng "fail dưới
systemd", nguyên nhân gần như luôn nằm trong số này.

`systemd-run --pty --uid=proxy -p LimitNOFILE=1024 ...` hoặc `systemd-run --scope` giúp tái hiện môi trường đó theo
kiểu tương tác. `systemctl show proxy` in ra mọi cài đặt đang có hiệu lực.

## Practice

Hãy làm theo thứ tự.

1. Viết unit ở trên cho bản build proxy của bạn ([`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), hoặc `proxy/`
   khi đã build) và cài nó dưới `/etc/systemd/system/` (hoặc một unit `--user` trong `~/.config/systemd/user/`).
   **Done when** `systemctl status` hiện `active (running)`, `journalctl -u` hiện log của nó, và
   `systemctl show -p MainPID` bằng PID của binary của bạn (không có shell chen vào giữa).
2. Kiểm chứng hợp đồng dừng. Thêm một `SIGTERM` handler biết drain các request đang xử lý, giữ một request chậm đang mở
   bằng `curl`, rồi chạy `systemctl stop`. **Done when** request chậm kết thúc, process thoát với mã 0 và `journalctl`
   không hiện `SIGKILL`. Sau đó hạ `TimeoutStopSec` xuống dưới thời gian drain và cho thấy kết quả 137/`killed`.
3. Cố tình làm nó crash (`kill -SEGV $MAINPID`, rồi một config sai) và quan sát hành vi của `Restart=` cùng
   `StartLimitBurst`. **Done when** bạn giải thích được từng dòng của `systemctl status` / `journalctl` cho cả hai
   trường hợp, và proxy từ chối config sai lúc khởi động bằng một thông báo rõ ràng.
4. Thêm `LimitNOFILE=` và `AmbientCapabilities=CAP_NET_BIND_SERVICE`, chạy với `User=proxy`, và bind `:80`.
   **Done when** `/proc/<pid>/limits` và `/proc/<pid>/status` (`CapEff`) cho thấy cả hai tác dụng.
5. Chuyển sang socket activation: tạo một `proxy.socket` với `ListenStream=8080` và đọc fd qua `$LISTEN_FDS`.
   **Done when** `systemctl restart proxy` trong lúc một vòng lặp `curl` đang chạy không hiện connection nào bị từ chối
   (các request chỉ chờ một chút).
6. Chạy `systemd-analyze security proxy.service`, áp dụng ít nhất năm tùy chọn hardening, rồi test lại. **Done when**
   điểm số cải thiện và proxy vẫn phục vụ bình thường, và mọi `EACCES` bạn gây ra đều được giải thích bằng `strace`.
7. Thêm `Type=notify` (crate `sd-notify`) và gửi `READY=1` sau khi listener đã bind. **Done when** `systemctl start`
   chặn lại cho đến khi port thực sự đang listen, và một unit phụ thuộc với `After=proxy.service` có thể tin cậy vào
   điều đó.

---

<!-- nav -->
← Trước: [`02-linux/20-limits-and-proc.md`](20-limits-and-proc.md) · [Mục lục](00-README.md) · Tiếp theo: [`02-linux/22-recall-and-review.md`](22-recall-and-review.md) →
