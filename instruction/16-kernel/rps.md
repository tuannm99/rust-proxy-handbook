# RPS (Receive Packet Steering)

`16-kernel/rss.md` covers spreading packet processing across cores in
hardware. RPS is the kernel's software equivalent, for NICs that don't
give you enough hardware queues to do it there — common on cloud VMs
with single-queue virtio-net interfaces.

## What to learn

### The same goal, moved into software
Once a packet arrives on whatever queue/core the NIC (or its single
queue) delivered it to, RPS computes a hash over the packet headers —
conceptually the same 5-tuple hash RSS uses — and, if it selects a
*different* core than the one currently processing the packet, sends an
inter-processor interrupt (IPI) to steer the rest of that packet's
processing there. It solves the same "spread load across cores"
problem as RSS, but after the fact and in software, which costs an IPI
that hardware RSS avoids entirely.

### RFS: steering toward the consuming application thread
RPS's hash-based steering doesn't know or care which core the
*application* thread reading the socket is actually running on — it can
steer packet processing to a core, and then have the application thread
(on a different core) pick up the data anyway, losing the cache-locality
benefit steering was meant to provide. **Receive Flow Steering (RFS)**
refines this by tracking, per flow, which CPU last called `recvmsg` for
it, and steering that flow's future packets there instead of by hash
alone — better alignment with where the data will actually be consumed,
at the cost of maintaining a flow table.

### Configuring it
```
# per receive-queue bitmap of CPUs eligible for RPS steering
echo f > /sys/class/net/eth0/queues/rx-0/rps_cpus   # CPUs 0-3

# RFS: size of the global and per-queue flow tables
echo 32768 > /proc/sys/net/core/rps_sock_flow_entries
echo 2048 > /sys/class/net/eth0/queues/rx-0/rps_flow_cnt
```

### When it actually matters for a proxy
Check `ethtool -l` first (per `16-kernel/rss.md`'s exercise): if the NIC
already exposes multiple hardware queues with RSS properly spread across
cores, RPS adds IPI overhead for no benefit — it's a fallback for when
hardware steering isn't available or isn't sufficient (a single-queue
virtio-net NIC is the common real case in cloud environments), not a
strict upgrade over RSS.

### Gotcha: the IPI cost can become its own bottleneck
At very high packet rates on a many-core machine, the inter-processor
interrupts RPS issues to steer packets are themselves not free — enough
of them can consume a meaningful fraction of a core, especially if
steering targets are spread thinly across many cores each handling a
small amount of traffic. Measure whether enabling RPS actually improves
throughput/latency for your traffic pattern rather than assuming
"spread across more cores" is unconditionally better; sometimes it just
moves the bottleneck from "one core doing everything" to "every core
spending cycles on IPIs."

## Practice
1. Confirm via `ethtool -l` that your test system's NIC exposes only one
   (or few) hardware queues — the scenario RPS is actually for.
2. Enable RPS by setting `rps_cpus` for the receive queue, and RFS via
   the flow-table sysctls above.
3. Drive sustained load at `proxy` and compare per-core CPU distribution
   and throughput/latency with RPS on versus off.
4. If you have access to a multi-queue NIC from `16-kernel/rss.md`'s
   exercise, compare RPS's achieved core distribution and overhead
   against RSS's hardware-based result on the same workload.
