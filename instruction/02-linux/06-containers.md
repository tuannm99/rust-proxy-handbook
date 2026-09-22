# Containers: Namespaces and cgroups

Part of the from-scratch fundamentals series — see `02-linux/01-fundamentals.md`
for the full index. Kubernetes, pods, and containers get referenced
constantly from `09-architecture/` onward and in several `07-security/`
and `08-observability/` gotchas — this file is where "what actually *is*
a container" gets answered, since nothing else in the handbook stops to
define it.

## What to learn

### A container is not a tiny VM
A virtual machine virtualizes *hardware* — it runs its own full kernel,
believing it has its own CPU, memory, and devices, on top of a hypervisor
that intercepts and emulates the real hardware. A **container** does none
of that: it's an ordinary process (or group of processes), running under
the *same* kernel as everything else on the host, that the kernel makes
*believe* it's alone on the machine using two separate mechanisms. No
second kernel, no hypervisor, no hardware emulation — which is exactly
why containers start in milliseconds where a VM takes seconds, and why a
container's `uname -r` reports the *host's* kernel version, not its own.

### Namespaces: what a process can see
A **namespace** limits what a process can *see*. Linux has several kinds,
each isolating one category of global system state:
- **PID namespace** — a container's process sees itself as PID 1 (or
  close to it) and cannot see or signal processes outside its namespace,
  even though they're all real processes on the same host kernel.
- **Network namespace** — a container gets its own network interfaces,
  IP addresses, routing table, and port space, isolated from the host's
  — this is *why* two containers can each bind port 8080 without
  conflicting: they're in different network namespaces, so from the
  kernel's point of view those are two unrelated "port 8080"s
  (`01-network/02-addressing.md`'s ports are namespace-local, not truly
  global, once namespaces are involved).
- **Mount namespace** — a container sees its own filesystem root,
  layered from an image, distinct from the host's actual filesystem.
- **UTS namespace** — its own hostname.

None of this involves a second kernel or emulated hardware — it's the
same kernel, the same physical CPU, the same physical RAM, just with the
view each process gets of global state (process list, network stack,
filesystem) restricted per namespace.

### cgroups: what a process is allowed to use
Namespaces limit *visibility*; **cgroups** (control groups) limit
*consumption* — CPU time, memory, I/O bandwidth — for a group of
processes, enforced by the kernel regardless of what those processes
think they're allowed to do. This is the direct mechanism behind
`02-linux/08-memory.md`'s cgroup-memory-limit gotcha and every
"OOM-killed in Kubernetes" incident: a container's memory limit is a
cgroup limit, enforced against **RSS** (resident, physically-backed
memory — the actual consequence of `04-memory-basics.md`'s
virtual-vs-physical distinction), not against however much virtual
memory your process merely *reserved*.

```
$ cat /sys/fs/cgroup/memory.max     # the limit, in bytes (cgroup v2 path)
$ cat /sys/fs/cgroup/memory.current # current usage against that limit
```

A process that's allocated (reserved) far more virtual memory than its
cgroup limit is completely fine — right up until it actually *writes* to
enough of those pages that RSS crosses the limit, at which point the
kernel's OOM killer ends the process abruptly, often with no warning your
own code can catch. This is precisely the scenario `02-linux/08-memory.md`
warns about for a proxy that pre-allocates large buffer pools.

### A pod is a shared set of namespaces
In Kubernetes terms: a **container** is one namespaced, cgroup-limited
process group running one image. A **pod** is Kubernetes's unit of
deployment — one or more containers that *share* a network namespace
(and therefore an IP address and port space) while keeping separate mount
namespaces (separate filesystems) and separate cgroup limits. This is
precisely the mechanism a **sidecar proxy**
(`01-network/05-proxy-taxonomy.md`) relies on: the sidecar and the
application container are different processes, isolated from each other
in most ways, but share one network namespace, so the sidecar can
transparently intercept the application's traffic on `localhost` without
any special networking trick.

### Why this matters for a proxy
A proxy running inside a container inherits every one of these limits
whether or not its own code is aware of them: its fd limit
(`03-kernel-and-syscalls.md`) may be capped tighter by the container
runtime than the host default, its visible CPU count may not match the
host's physical core count (cgroup CPU limits can present as fractional
cores — `nproc` inside a container can lie about what's actually
available, which matters directly for sizing a tokio worker-thread pool),
and its memory behavior is governed by RSS-against-cgroup-limit as
described above, not by whatever `ulimit` or the process's own accounting
believes. None of `09-architecture/`'s deployment content (rolling
restarts, graceful shutdown timing against `terminationGracePeriodSeconds`)
makes full sense without this namespace/cgroup picture underneath it.

## Practice
1. If you have Docker or Podman available, run `docker run --rm -it
   alpine sh`, then inside it run `ps aux` (see how few processes exist —
   PID namespace) and `hostname` (see the container's own UTS namespace).
   From another terminal on the host, run `ps aux` and try to find the
   container's process — note its *real* host PID differs from what the
   container itself sees as its PID.
2. Start two containers each binding port 8080 (`docker run -p 18080:8080
   ...` and `docker run -p 18081:8080 ...` using any simple HTTP server
   image) and confirm both work — explain, using the network-namespace
   section above, why this doesn't conflict.
3. If cgroup v2 is available on your system (`ls /sys/fs/cgroup`), find a
   running container's cgroup directory and read its `memory.max` and
   `memory.current` — compare `memory.current` against what `docker
   stats` reports for the same container.
4. Set a container's memory limit deliberately low (`docker run -m 50m
   ...`) and run a program inside it that allocates and *writes to* more
   than 50MB — observe it get killed, then check `dmesg` on the host for
   the OOM killer's log line naming the process.
5. Run `nproc` on the host, then run it again inside a container started
   with `--cpus=1` on that same host — note the container may still
   report the host's full core count even though its cgroup limits it to
   one CPU's worth of time; explain why a tokio runtime that sizes its
   worker pool from `nproc` could over-provision threads inside such a
   container.
