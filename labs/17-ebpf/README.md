# 17-ebpf

## Goal

XDP/eBPF-based packet filtering at the earliest possible point in the
receive path, ahead of the userspace proxy — relevant to volumetric DDoS
mitigation.

## Handbook references
- `instruction/16-kernel/ebpf.md`, `instruction/16-kernel/xdp.md` — the kernel-side deep dive
- `instruction/07-security/ddos.md`

## Run

```
cargo run -p ebpf-lab
```
