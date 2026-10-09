# Chạy như một Service: systemd, Supervision, và Hợp đồng của Process

Một proxy production thực sự được khởi động, restart, gửi signal, giới hạn và quan sát trên một Linux host ra sao — và
hợp đồng giữa supervisor và process của bạn (signal vào, exit code và readiness ra). Nối
[`04-process-lifecycle.md`](04-process-lifecycle.md), [`17-signals.md`](17-signals.md), [`20-limits-and-proc.md`](20-limits-and-proc.md) và [`13-containers.md`](13-containers.md) lại với nhau.

## What to learn

### Một supervisor làm gì
**Supervisor** (systemd trên hầu hết distribution; container runtime và kubelet của Kubernetes trong container) khởi động process của bạn
trong một môi trường được kiểm soát — user, working directory, environment, limit, cgroup — theo dõi nó, restart nếu nó chết, giao signal
dừng, và thu output của nó. Việc của process bạn là cư xử có thể dự đoán được trong hợp đồng đó thay vì tự daemonize: **ở foreground,
log ra stdout/stderr, xử lý `SIGTERM`, thoát với exit code có ý nghĩa** ([`04-process-lifecycle.md`](04-process-lifecycle.md),
[`08-observability/01-logging.md`](../08-observability/01-logging.md)). systemd là PID 1 trên host nên thu dọn orphan và sở hữu cgroup của mọi service.

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
# bind :443 mà không là root
AmbientCapabilities=CAP_NET_BIND_SERVICE
NoNewPrivileges=yes
# graceful shutdown được phép mất bao lâu trước khi SIGKILL
TimeoutStopSec=30

[Install]
WantedBy=multi-user.target
```
`systemctl daemon-reload` (sau khi sửa), `systemctl start|stop|restart|reload|status proxy`, `systemctl enable` (khởi động cùng boot),
`journalctl -u proxy -f` (log của nó). `ExecStart` phải gọi tên binary trực tiếp: nếu bị bọc trong `sh -c`, signal đi tới shell
([`04-process-lifecycle.md`](04-process-lifecycle.md)).

### Hợp đồng dừng: SIGTERM, timeout, SIGKILL
`systemctl stop` gửi **`SIGTERM`** tới main process của service, chờ tối đa `TimeoutStopSec`, rồi gửi **`SIGKILL`** cho mọi thứ còn lại trong cgroup.
Nên proxy của bạn, khi nhận `SIGTERM`, phải ngừng nhận connection mới, để các request đang bay kết thúc (giới hạn bởi một deadline), rồi thoát với 0
([`17-signals.md`](17-signals.md), [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md)). Hãy đặt `TimeoutStopSec` dài hơn deadline drain, nếu không một lần drain chậm sẽ bị `SIGKILL` cắt ngang — và exit status 137
trong log là manh mối ([`04-process-lifecycle.md`](04-process-lifecycle.md)). Kubernetes có cùng hình dạng: `SIGTERM`, rồi `terminationGracePeriodSeconds`, rồi `SIGKILL`; cộng một hook `preStop` và việc gỡ khỏi endpoint
của load balancer diễn ra *đồng thời* với signal, nên một proxy thường cần phục vụ thêm một lúc ngắn sau `SIGTERM` ([`09-architecture/06-canary-deploy.md`](../09-architecture/06-canary-deploy.md)).
`ExecReload=` là hợp đồng reload: `SIGHUP` -> đọc lại config mà không làm rơi connection ([`09-architecture/03-config.md`](../09-architecture/03-config.md)).

### Chính sách restart và crash loop
`Restart=on-failure` restart sau một exit khác 0 hoặc một signal gây chết; `Restart=always` cũng restart sau một exit sạch. `RestartSec` làm trễ nó. Mặc định systemd
bỏ cuộc sau vài lần restart trong một cửa sổ ngắn (`StartLimitBurst`/`StartLimitIntervalSec`). Một proxy **crash vì config sai** sẽ bị restart vào đúng cái crash đó —
*hãy fail nhanh với config không hợp lệ lúc khởi động bằng một thông báo rõ ràng và một exit code riêng*, và validate config mới **trước** khi hoán đổi nó trong một lần reload để một lỗi gõ
không làm sập service ([`09-architecture/03-config.md`](../09-architecture/03-config.md)). Hãy cảnh báo theo số lần restart, không chỉ "process đang chạy" ([`08-observability/06-alerting.md`](../08-observability/06-alerting.md)).

### Type=notify: readiness, không chỉ "đã khởi động"
Với `Type=simple`, systemd coi service đã khởi động ngay khoảnh khắc process được fork — trước khi nó bind port hay nạp config. Các dependent và health gate khi đó chạy lố lên trước.
`Type=notify` sửa việc này: process gửi `READY=1` qua socket có tên trong `$NOTIFY_SOCKET` (qua `sd_notify`, hoặc crate `sd-notify`) *sau khi* nó thực sự sẵn sàng, và có thể gửi
`RELOADING=1`/`STOPPING=1`. Cùng sự phân biệt đó tồn tại trong Kubernetes dưới dạng **readiness probe** vs việc process chỉ đang chạy
([`06-proxy/03-healthcheck.md`](../06-proxy/03-healthcheck.md)). Tránh `Type=forking` (kiểu daemonize cũ) cho phần mềm mới.

### Socket activation: systemd sở hữu listening socket
Một unit `.socket` cho phép **systemd bind port** và trao cho process của bạn fd đã listen sẵn (truyền dưới dạng fd 3, với `$LISTEN_FDS`/`$LISTEN_PID` được set; các crate
`listenfd` hoặc `sd-listen-fds` đọc nó). Lợi ích: port vẫn mở và **connection xếp hàng trong kernel backlog khi service restart** — một cách restart không downtime đơn giản
([`01-network/11-socket.md`](../01-network/11-socket.md)); service có thể bind port 443 và chạy không đặc quyền vì nó không bao giờ tự gọi `bind`
([`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)); và thứ tự khởi động thôi quan trọng. Nó dùng cùng cơ chế thừa kế như một hot restart
([`10-ipc.md`](10-ipc.md), [`09-architecture/05-rolling-restart.md`](../09-architecture/05-rolling-restart.md)): một listening fd sống lâu hơn bất kỳ process đơn lẻ nào.

### cgroup cho từng service: limit và accounting miễn phí
systemd đặt mỗi service vào cgroup riêng ([`13-containers.md`](13-containers.md)), nên `systemctl status proxy` liệt kê mọi process của service (kể cả child), và các cài đặt unit ánh xạ thành limit cgroup: `MemoryMax=`,
`CPUQuota=200%`, `TasksMax=`, `IOWeight=`. `systemd-cgls` hiện cây; `systemd-cgtop` hiện mức dùng trực tiếp. Hãy cẩn thận với `CPUQuota`: nó là trần throttle với hiệu ứng latency
mô tả ở [`12-cpu-scheduling.md`](12-cpu-scheduling.md). `MemoryMax` kích hoạt cgroup OOM killer ([`16-memory.md`](16-memory.md)).

### Hardening như cấu hình
Một tá dòng unit cho sandboxing mà không cần code: `NoNewPrivileges=yes`, `ProtectSystem=strict` (cây OS read-only), `ProtectHome=yes`, `PrivateTmp=yes`,
`ReadWritePaths=/var/log/proxy`, `CapabilityBoundingSet=CAP_NET_BIND_SERVICE`, `SystemCallFilter=@system-service` (một allow-list seccomp,
[`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)), `RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX`. `systemd-analyze security proxy` chấm điểm một unit và liệt kê những gì còn
phơi ra. Hardening quá chặt hiện ra thành `EACCES`/`EPERM` trong `strace`.

### Log: journald và stdout
Mọi thứ trên stdout/stderr rơi vào journal, gắn tag unit và timestamp; `journalctl -u proxy --since '10 min ago' -o json` là truy xuất có cấu trúc.
Vậy logging ra file đòi hỏi rotation được xử lý bên ngoài process ([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)); logging ra stdout cũng là hợp đồng mà container dùng.

### Gotcha: môi trường bạn test không phải môi trường nó chạy
Dưới một supervisor, proxy chạy với `PATH` tối thiểu, không TTY, working directory khác, không có file khởi động shell, limit khác, user khác, và
có thể filesystem read-only. "Chạy được khi tôi chạy" và "fail dưới systemd" gần như luôn là một trong những thứ này. `systemd-run --pty --uid=proxy -p LimitNOFILE=1024 ...`
hoặc `systemd-run --scope` tái hiện môi trường một cách tương tác; `systemctl show proxy` in mọi cài đặt đang có hiệu lực.

## Practice

Hãy làm theo thứ tự.

1. Viết unit ở trên cho bản build proxy của bạn ([`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), hoặc `proxy/` khi đã build) và cài nó dưới `/etc/systemd/system/` (hoặc một unit `--user` trong
   `~/.config/systemd/user/`). **Done when** `systemctl status` hiện `active (running)`, `journalctl -u` hiện log của nó, và `systemctl show -p MainPID` bằng
   PID của binary bạn (không có shell chen giữa).
2. Kiểm chứng hợp đồng dừng: thêm một `SIGTERM` handler drain các request đang bay, giữ một request chậm mở bằng `curl`, và `systemctl stop`. **Done when** request
   chậm kết thúc, process thoát 0 và `journalctl` không hiện `SIGKILL`; rồi hạ `TimeoutStopSec` xuống dưới thời gian drain và cho thấy kết quả 137/`killed`.
3. Cố tình làm nó crash (`kill -SEGV $MAINPID`, rồi một config sai) và xem hành vi `Restart=` cùng `StartLimitBurst`. **Done when** bạn giải thích được từng dòng của
   `systemctl status` / `journalctl` cho cả hai trường hợp và proxy từ chối config sai lúc khởi động với một thông báo rõ ràng.
4. Thêm `LimitNOFILE=` và `AmbientCapabilities=CAP_NET_BIND_SERVICE`, chạy với `User=proxy`, và bind `:80`. **Done when** `/proc/<pid>/limits` và `/proc/<pid>/status`
   (`CapEff`) hiện cả hai hiệu ứng.
5. Chuyển sang socket activation: một `proxy.socket` với `ListenStream=8080` và đọc fd qua `$LISTEN_FDS`. **Done when** `systemctl restart proxy` giữa lúc chạy một vòng `curl` không hiện connection
   bị từ chối (request chờ ngắn thay vì thế).
6. Chạy `systemd-analyze security proxy.service`, áp dụng ít nhất năm tùy chọn hardening, và test lại. **Done when** điểm cải thiện và proxy vẫn phục vụ,
   với mọi `EACCES` bạn gây ra được giải thích qua `strace`.
7. Thêm `Type=notify` (crate `sd-notify`) và `READY=1` sau khi listener đã bind. **Done when** `systemctl start` block cho tới khi port thực sự đang listen, và một unit phụ thuộc với `After=proxy.service`
   có thể dựa vào nó.
