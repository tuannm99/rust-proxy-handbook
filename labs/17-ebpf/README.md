# 17-ebpf

## Goal

XDP/eBPF-based packet filtering at the earliest possible point in the
receive path, ahead of the userspace proxy — relevant to volumetric DDoS
mitigation.

## Done when

- [ ] An XDP program (for example written with Aya) is attached to an interface — a `veth` pair in a network namespace is fine — and drops packets from IPs in a denylist map.
- [ ] Userspace adds and removes denylist entries at runtime without reloading the program.
- [ ] A per-IP drop counter map is readable from userspace.
- [ ] Traffic from a denied IP never reaches the userspace proxy: no socket appears in `ss`, no log line is written.
- [ ] Allowed traffic is unaffected: a load test shows no measurable throughput or latency change with the program attached.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/16-kernel/09-ebpf.md`](../../instruction/16-kernel/09-ebpf.md), [`instruction/16-kernel/10-xdp.md`](../../instruction/16-kernel/10-xdp.md) — the kernel-side deep dive
- [`instruction/07-security/09-ddos.md`](../../instruction/07-security/09-ddos.md), [`instruction/07-security/10-slowloris.md`](../../instruction/07-security/10-slowloris.md), [`instruction/07-security/11-load-shedding.md`](../../instruction/07-security/11-load-shedding.md)

## Run

```
cargo run -p ebpf-lab
```
