# 17-ebpf

## Goal

XDP/eBPF-based packet filtering at the earliest possible point in the
receive path, ahead of the userspace proxy — relevant to volumetric DDoS
mitigation.

## Handbook references
- `instruction/16-kernel/09-ebpf.md`, `instruction/16-kernel/10-xdp.md` — the kernel-side deep dive
- `instruction/07-security/09-ddos.md`, `instruction/07-security/10-slowloris.md`, `instruction/07-security/11-load-shedding.md`

## Run

```
cargo run -p ebpf-lab
```
