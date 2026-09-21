# RSS (Receive Side Scaling)

`16-kernel/04-interrupt.md` covers why one core handling all packet
interrupts becomes a ceiling on throughput. RSS is the hardware-level
fix: the NIC itself spreads incoming packets — and their interrupts —
across multiple cores before the kernel ever sees them.

## What to learn

### Hardware hashing into multiple queues
A multi-queue NIC hashes each incoming packet's headers (commonly the
5-tuple: source/dest IP and port, protocol) using a hash function
implemented in hardware (Toeplitz is the common one) to pick one of
several receive queues. Each queue gets its own interrupt, and each
interrupt can be steered (via `/proc/irq/<n>/smp_affinity` or `irqbalance`)
to a specific core. The result: packets belonging to the same flow
(same 5-tuple, so the same TCP connection) always land on the same
queue and the same core, while different connections spread across
however many queues the NIC and driver support.

### Why this specifically is what lets you use more cores for networking
Without RSS (a single-queue NIC, or RSS disabled), every packet's
interrupt lands on one core — usually core 0 by default — regardless of
how many cores your application spawns worker threads on. RSS is what
actually makes "packet processing" a parallel, multi-core workload at
the hardware level; without it, `16-kernel/04-interrupt.md`'s ceiling
applies no matter how the application is architected.

### Inspecting and tuning it
```
ethtool -l eth0   # show current/max queue count
ethtool -L eth0 combined 8   # request 8 combined queues
ethtool -x eth0   # show the current indirection table (hash -> queue mapping)
```
The indirection table maps hash buckets to queues; on most NICs you
don't hand-tune the table itself, just the queue count and per-queue IRQ
affinity.

### Gotcha: connection concentration behind another proxy/NAT
The 5-tuple hash assumes source IP/port diversity to spread load evenly.
A proxy sitting behind another load balancer doing source NAT, or
serving a workload where most traffic comes from a small number of
upstream/client IPs, can see the hash concentrate onto a handful of
queues despite RSS being correctly configured — the hash function has
nothing to work with when the input space is narrow. Check per-queue
packet counts (`ethtool -S eth0 | grep rx_queue`) rather than assuming
configured RSS means evenly *achieved* balance.

## Practice
1. Check your test system's queue count and indirection table with
   `ethtool -l` / `ethtool -x` (a cloud VM's virtio-net NIC may report
   only one queue — note that if so, since it changes what's actually
   achievable here).
2. If multiple queues are available, set per-queue IRQ affinity to spread
   across several cores and confirm with `/proc/interrupts` that
   different queues' interrupts land on different cores.
3. Drive load at `proxy` from many distinct source ports/connections and
   compare per-queue packet counts (`ethtool -S`) against a test that
   reuses very few source connections — observe the concentration effect
   the gotcha above describes.
4. Correlate per-core CPU usage (`mpstat -P ALL`) with per-queue packet
   counts during a load test to confirm RSS's queue-to-core mapping is
   actually being reflected in application-level throughput scaling.
