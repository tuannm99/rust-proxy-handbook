# Interrupts and Softirqs

Why "how many packets per second can this box handle" is partly a
question about interrupt handling, not application code — and why that
question gets harder, not easier, as core counts grow.

## What to learn

### Hard IRQ: minimal, fast, interrupts disabled
When a NIC has a packet ready, it raises a hardware interrupt. The
handler that runs for it (the "hard IRQ" context) runs with interrupts
disabled on that core and must be extremely fast — it does the bare
minimum (acknowledge the device, queue further work) and returns
immediately. Anything slower would block *all* interrupt handling on
that core, including the timer interrupts the scheduler depends on.

### Softirq: the deferred, schedulable half
The actual packet processing — walking up the network stack, socket
demux, eventually reaching epoll's wakeup path
(`16-kernel/01-epoll-internals.md`) — happens in a **softirq**, scheduled to
run right after the hard IRQ handler returns, but in a context that can
be interrupted and is subject to normal scheduling pressure. `NET_RX` is
the softirq specifically responsible for incoming packet processing.
Under sustained high packet rates, `NET_RX` softirq work itself can
consume enough of a core that normal process scheduling on that core
starves — visible as high `%si` (softirq time) in `top`/`mpstat`, and as
latency in anything else trying to run on that core.

### Where this shows up under load, and the RSS connection
If every packet's interrupt (and therefore its softirq processing) lands
on one core regardless of how many cores the application uses, that one
core becomes the ceiling on total throughput no matter how many worker
threads `proxy` spawns. This is exactly the problem
`16-kernel/05-rss.md` (hardware) and `16-kernel/06-rps.md` (software) solve —
spreading interrupt/softirq load for different flows across different
cores so packet processing itself scales with core count.

### Gotcha: `irqbalance` fighting manual tuning
`irqbalance` dynamically rebalances IRQ affinity across cores based on
load. If you've hand-tuned IRQ affinity to align with where your
application's threads are pinned (`16-kernel/07-scheduler.md`), a running
`irqbalance` daemon can silently move interrupts back off your chosen
cores, producing intermittent latency regressions that are hard to
reproduce because the assignment keeps changing underneath you. Disable
`irqbalance` if you're setting IRQ affinity by hand — the two are not
meant to coexist.

## Practice
1. Watch `/proc/interrupts` and `/proc/softirqs` (or `mpstat -P ALL 1`'s
   `%irq`/`%soft` columns) while driving sustained load at
   `labs/00-tcp-server` or `proxy`.
2. Identify which core(s) are handling the bulk of `NET_RX` softirq work
   and compare against which cores your application's worker threads are
   actually running on.
3. If you have access to hardware/a VM where you can set IRQ affinity
   (`/proc/irq/<n>/smp_affinity`), try aligning NIC interrupt cores with
   your pinned application cores (from `16-kernel/07-scheduler.md`'s
   exercise) and measure the effect on throughput and latency.
4. Check whether `irqbalance` is running on your test system; stop it,
   re-run your affinity experiment, and compare stability of the
   measurements with it running vs. stopped.
