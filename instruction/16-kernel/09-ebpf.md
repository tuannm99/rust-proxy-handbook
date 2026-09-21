# eBPF

Running your own verified code inside the kernel, without a module and
without a reboot. The foundation for XDP packet filtering
(`16-kernel/10-xdp.md`) and for most modern kernel-level observability.

## What to learn

### What eBPF actually is
A small RISC-like virtual machine in the kernel, with 11 registers, a 512-byte
stack, and a bytecode instruction set. You compile a restricted C (or Rust)
program to eBPF bytecode, the kernel **verifies** it, JIT-compiles it to
native instructions, and attaches it to a hook. From then on it runs at
native speed every time that hook fires.

The point is the safety boundary. A kernel module that crashes takes the
machine down; an eBPF program that would crash is rejected before it ever
loads. That is what makes it deployable on production hosts.

### The verifier is the whole story
Before loading, the verifier simulates every possible execution path and
rejects anything it cannot prove safe. The rules that shape how you write
eBPF:

- **Bounded execution.** No unbounded loops. Early kernels banned loops
  entirely; modern ones allow bounded loops the verifier can unroll, plus
  `bpf_loop()` for a bounded helper-driven loop. There is a hard limit on
  total verified instructions (1M on current kernels).
- **Every memory access must be provably in bounds.** Reading a packet
  requires an explicit `if (data + offset > data_end) return ...` check
  *before* the access — and the check must be visible to the verifier on
  every path, which is why eBPF code is full of bounds checks that look
  redundant to a human.
- **No arbitrary kernel memory.** Access goes through helper functions
  (`bpf_probe_read_kernel`, map operations), not raw pointers.
- **No unbounded stack.** 512 bytes total, which is why anything larger
  lives in a map.

Gotcha: verifier rejections are the dominant development cost, and the
error messages describe the *verifier's* state, not your intent. The usual
cause is a bounds check the verifier cannot connect to the access —
restructuring the code so the check immediately precedes the access, on
every path, is the standard fix. Expect this to be most of your debugging
time.

### Maps: the only way to keep state
eBPF programs are stateless between invocations. All persistent state, and
all communication with user space, goes through **maps** — typed key/value
stores created and managed by the kernel.

The types that matter here: `HASH` (general key/value, e.g. per-IP
counters), `ARRAY` (index-keyed, fast), `PERCPU_HASH`/`PERCPU_ARRAY` (one
instance per CPU, no atomics needed — the right choice for counters),
`LPM_TRIE` (longest-prefix match, which is exactly CIDR matching for
`07-security/08-ip-filtering.md`), and `RINGBUF` (efficient
kernel-to-userspace event streaming).

```
// the shape: kernel side increments, user space reads, no syscall in the hot path
PERCPU_HASH<u32 /* src ip */, u64 /* packet count */>
```

Gotcha: prefer per-CPU maps for counters. A shared `HASH` counter needs an
atomic on every packet and becomes a contention point at multi-million-pps
rates; per-CPU maps are lock-free in the kernel and summed in user space at
read time. The trade is that you cannot read an exact instantaneous total
from the kernel side.

### Attach points relevant to a proxy
- **XDP** — earliest possible, in the NIC driver before an `sk_buff` exists.
  Fastest, most restricted. See `16-kernel/10-xdp.md`.
- **TC (traffic control)** — after `sk_buff` allocation; slower than XDP but
  sees both ingress and egress and can modify packets more freely.
- **Socket filters / `SO_ATTACH_BPF`** — per-socket, useful for steering.
- **kprobes / tracepoints / USDT** — observability rather than filtering:
  attach to kernel functions or static tracepoints to measure what the
  kernel is doing under your proxy. This is what `bpftrace` compiles to,
  and it is the most immediately practical eBPF for
  `08-observability/04-profiling.md`.

### The Rust story
Two real options. **Aya** is pure Rust for both the kernel-side program and
the user-space loader, with no libbpf/clang dependency — the more pleasant
choice, and what `labs/17-ebpf` targets. **libbpf-rs** binds the C libbpf
library and inherits its maturity and CO-RE support.

**CO-RE** (Compile Once, Run Everywhere) is the portability mechanism worth
knowing: kernel struct layouts differ between versions, so a program
hardcoding field offsets breaks on a different kernel. CO-RE emits
relocations resolved at load time against the running kernel's BTF type
information, so one binary works across kernels.

Gotcha: eBPF requires root or `CAP_BPF`/`CAP_NET_ADMIN`. A proxy that drops
privileges after binding its port cannot load eBPF programs afterward —
load at startup while still privileged, or split loading into a separate
privileged helper. This ordering constraint has to be designed in, not
retrofitted.

### When it is worth it
eBPF filtering pays off when you need to drop traffic *before* it costs
anything (`07-security/09-ddos.md`), or observe the kernel without
instrumenting your application. It is not a substitute for application
logic: it cannot parse HTTP meaningfully, cannot make decisions requiring
user-space state, and every rule is limited by the verifier. Use it as the
cheap first filter, with the proxy handling everything that survives.

## Practice
1. Write a minimal Aya program in `labs/17-ebpf` that counts received
   packets in a `PERCPU_ARRAY` and a user-space loader that prints the
   summed total once a second.
2. Deliberately trigger a verifier rejection: read a packet byte without a
   preceding `data_end` bounds check. Read the error, then fix it — this is
   the loop you will spend most of your eBPF time in.
3. Replace the per-CPU counter with a shared `HASH` and benchmark both
   under load; measure the contention cost.
4. Build an `LPM_TRIE` map of blocked CIDRs, populate it from user space,
   and look up source addresses against it from the kernel side — the same
   matching `07-security/08-ip-filtering.md` does in the proxy.
5. Stream events to user space with a `RINGBUF` and compare its throughput
   against a per-event map lookup.
6. Use `bpftrace` (no code) to histogram the latency of `tcp_sendmsg` while
   your proxy serves load, and reconcile it with the metrics from
   `08-observability/02-metrics.md`.
