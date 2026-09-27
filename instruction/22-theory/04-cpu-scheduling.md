# CPU Scheduling Algorithms

## What to learn

### The metrics scheduling is actually judged by
Turnaround time (completion − arrival), waiting time (turnaround − service time), and response time (first response − arrival) measure different things, and a scheduler optimizing one can worsen another. [`16-kernel/07-scheduler.md`](../16-kernel/07-scheduler.md)'s practical CFS discussion assumes these metrics; this file is where they're defined precisely enough to compute by hand.

### FCFS and the convoy effect
First-come-first-served is the simplest policy and the worst under mixed workloads: one long job ahead of many short ones makes every short job wait behind it — the convoy effect. It's a direct model for a very real proxy bug: one slow upstream request occupying a worker thread ahead of many fast ones ([`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md)'s blocking-thread hazard is the same shape, one level up the stack).

### SJF/SRTF: provably optimal for average waiting time, and why nobody runs it as-is
Shortest-Job-First (non-preemptive) or Shortest-Remaining-Time-First (preemptive) minimizes average waiting time — provably, this is the best achievable if you're purely optimizing that one metric. The catch: it requires knowing job length in advance, unavailable for most real workloads, and it starves long jobs indefinitely under continuous short-job arrival.

### Round-robin and the quantum-size tradeoff
Each process gets a fixed time slice (quantum) before being preempted back to the queue. A quantum too large degenerates toward FCFS, with its convoy effect; a quantum too small wastes time on context-switch overhead relative to actual work done. There is no universally correct quantum — it's a direct tradeoff between responsiveness and throughput, tuned per workload.

```text
quantum too large  -> behaves like FCFS, long jobs block short ones
quantum too small  -> most CPU time spent context-switching, not computing
```

### Multi-Level Feedback Queue (MLFQ): approximating SJF without knowing job length
MLFQ runs multiple queues at different priority levels with different quanta; a process that uses its full quantum (behaving like a long job) is demoted to a lower-priority, longer-quantum queue, while one that yields early (behaving like an I/O-bound short job) stays high-priority. This approximates SJF's benefit using only *observed* behavior instead of requiring foreknowledge — the actual design principle behind most general-purpose OS schedulers, including a simplified relative of Linux's CFS.

### Where this connects to a proxy's own scheduling problem
A tokio runtime's work-stealing scheduler ([`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md)) and the cooperative-yield budget ([`03-rust/05-async.md`](../03-rust/05-async.md)'s coop discussion) are solving a version of this exact problem one level above the kernel: many tasks sharing a small number of worker threads, needing fairness without knowing task length in advance. The kernel-level theory here is the direct ancestor of tokio's own scheduling design.

## Practice
1. By hand, compute turnaround/waiting/response time for a small fixed set of jobs (given arrival time and burst time) under FCFS, then under SJF — confirm SJF's average waiting time is lower.
2. Simulate the convoy effect: one long job followed by five short jobs under FCFS vs. SJF, and compute the total waiting-time difference.
3. Implement round-robin over the same job set at two different quantum sizes (very small, very large) and plot context-switch count vs. completion time for each.
4. Implement a minimal 3-level MLFQ (rule: full quantum used → demote; yields early → stay) over a mixed CPU-bound/IO-bound synthetic workload, and compare average waiting time against plain round-robin.
5. Read [`16-kernel/07-scheduler.md`](../16-kernel/07-scheduler.md) and [`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md) back to back and write one paragraph mapping MLFQ's core idea (behavior-based priority, not foreknowledge) onto tokio's cooperative scheduling budget.
