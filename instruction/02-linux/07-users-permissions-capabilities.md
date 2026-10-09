# Users, Permissions, and Capabilities

Who a process *is* to the kernel, what that lets it do, and how a proxy that must
bind port 443 and read a private key still avoids running as all-powerful root.
Complements the "kernel mode vs root" distinction in [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md).

## What to learn

### Identity: UIDs and GIDs
To the kernel a user is a number. Every process carries a **UID** and a primary **GID**
plus supplementary groups; names in `/etc/passwd` and `/etc/group` are only labels for
`ls` and `id`. UID `0` is **root**, with special treatment. Each process actually has
several UIDs: **real** (who started it), **effective** (what permission checks use —
the one that matters), and **saved** (so it can switch back). A **setuid** executable runs
with its *owner's* effective UID (how `sudo` and `ping` historically worked), which is
why setuid root binaries are security-sensitive. Files carry an owner UID and group GID
([`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md)); your process's *effective* IDs are compared against them.
Inside a container the numbers still mean the same thing to the kernel — UID 0 in a
container is host UID 0 unless a **user namespace** remaps it ([`13-containers.md`](13-containers.md)).

### Permission bits: rwx for owner, group, other
Each inode has nine permission bits — read, write, execute for **owner**, **group** and
**other** — plus three special bits. The kernel checks the first class that matches the
caller (owner if you own it, else group, else other); it does not combine them.

```text
-rw-r----- 1 proxy proxy 1675 key.pem     = 0640: owner read/write, group read, others nothing
drwxr-x--- 2 proxy proxy 4096 certs/      = 0750: directory
```
Meaning differs for **directories**: `r` lists names, `w` creates/deletes/renames entries,
`x` lets you traverse *through* the directory (needed on every directory in a path).
Deleting a file needs write permission on its **directory**, not on the file. **`umask`**
removes bits from newly created files (`022` turns `666` into `644`); a proxy writing a
private key or a socket file should set modes explicitly (`OpenOptions::mode(0o600)` from
`std::os::unix::fs::OpenOptionsExt`) rather than inherit a umask. **root bypasses these
checks** entirely — which is exactly the problem.

### Why not just run as root?
A bug in a root process (a path-traversal in static serving, a parser memory-safety flaw in
`unsafe` code, a shell injection) gives the attacker the whole machine. A proxy handles
hostile bytes from the internet all day, so the design goal is **least privilege**: run as an
ordinary, dedicated user with only the access it needs. The two things that usually push people
toward root are binding a low port and reading a private key; both have better answers.

### Capabilities: root, sliced up
Linux splits root's power into ~40 **capabilities**. A process holds sets of them
(permitted, effective, inheritable, bounding, ambient), and a privileged operation needs
the specific capability, not UID 0. The ones you will meet:

- `CAP_NET_BIND_SERVICE` — bind ports below 1024. The classic fix for "my proxy must listen on 443":
  grant just this, not root (`setcap cap_net_bind_service=+ep ./proxy`, or `AmbientCapabilities=`
  in a systemd unit, or `--cap-add` in Docker). Alternatively lower the threshold:
  `sysctl net.ipv4.ip_unprivileged_port_start=443`, or have the supervisor open the socket.
- `CAP_NET_RAW` — raw sockets and packet capture (`ping`, `tcpdump`).
- `CAP_NET_ADMIN` — configure interfaces, routes, firewall, `tc`.
- `CAP_SYS_ADMIN` — the "new root": a grab-bag of dozens of operations; granting it to a
  container is nearly equivalent to giving root.
- `CAP_SYS_RESOURCE`, `CAP_SYS_NICE`, `CAP_IPC_LOCK` — exceed resource limits, raise
  scheduling priority, lock memory ([`20-limits-and-proc.md`](20-limits-and-proc.md), [`12-cpu-scheduling.md`](12-cpu-scheduling.md)).

`getpcaps <pid>` or `grep Cap /proc/<pid>/status` shows a process's sets
(`capsh --decode=<hex>` decodes them). Containers start with a trimmed default set; the safe
posture is to **drop all and add back** only what is needed.

### The drop-privileges pattern
When root really is needed briefly (a traditional start), do the privileged work *first*,
then permanently lower privilege:

1. Bind the low port and open the certificate/private key files.
2. Switch to an unprivileged user — in the right order: `setgroups` (clear
   supplementary groups), then `setgid`, then `setuid` (after `setuid` you can't `setgid`).
3. Verify you cannot regain root, and continue running the proxy.

An already-open fd stays valid after the drop — that's the whole trick: the listening socket
and the key material are acquired while privileged, then used unprivileged. Rust: the `nix`
crate (`nix::unistd::setuid`) or `libc`; **check every return value** — a failed `setuid` that
is ignored leaves you silently root. Today it's better still to have the *supervisor* start
you unprivileged with the socket pre-opened ([`21-systemd-and-services.md`](21-systemd-and-services.md): socket
activation) or with `User=` and the right ambient capability, and skip the dance.

### Layered hardening beyond UIDs
- **`no_new_privs`** (`prctl(PR_SET_NO_NEW_PRIVS)`): the process can never gain privileges via
  exec/setuid binaries — set it, as seccomp requires it.
- **seccomp**: an allow-list of syscalls the process may make; one stray exploit that tries `execve`
  is killed. The deepest hardening, and where eBPF filters also live ([`16-kernel/09-ebpf.md`](../16-kernel/09-ebpf.md)).
- **Namespaces and cgroups** ([`13-containers.md`](13-containers.md)) restrict what the process sees and uses.
- **Mandatory access control** (SELinux, AppArmor) adds policy that applies even to root-owned files.
  A common production surprise: everything looks permitted by mode bits yet fails with `EACCES`
  because an SELinux label forbids it (`ausearch -m avc`, `dmesg`).

### Gotcha: secrets on disk and in `/proc`
Private keys and tokens should be mode `0600` (or `0400`) owned by the service user, in a directory
others can't traverse. Secrets passed as **command-line arguments** are visible to every user in
`ps` and `/proc/<pid>/cmdline`; the **environment** is readable at `/proc/<pid>/environ` by the same
user and root. Prefer files with tight permissions, or a secret mount. And remember that a core dump
([`20-limits-and-proc.md`](20-limits-and-proc.md)) of the proxy contains its keys.

## Practice

1. Run `id`, `ls -l /etc/shadow /etc/passwd /usr/bin/sudo`; identify the owner, mode bits and (for
   `sudo`) the setuid bit (`s`), and explain which bit lets an ordinary user's `sudo` run as root.
2. Create `d/f` with modes of your choosing, then predict and verify whether you can `ls d`,
   `cat d/f`, and `rm d/f` for modes `000`/`500`/`700` on the directory — confirming that
   deletion depends on the directory's permissions.
3. As an unprivileged user try `python3 -m http.server 80` (expect `PermissionError`), then grant
   the capability: `sudo setcap cap_net_bind_service=+ep $(readlink -f $(which python3))` (revert
   with `setcap -r`) or use [`labs/00-tcp-server`](../../labs/00-tcp-server) on port 80 with the capability,
   and check `getpcaps`/`/proc/<pid>/status` (`CapEff`).
4. Test `sysctl net.ipv4.ip_unprivileged_port_start` before and after lowering it (and restore it).
5. Run your server in a container as non-root with `--cap-drop ALL --cap-add NET_BIND_SERVICE
   --user 1000` and binding port 80; then try an operation that needs a dropped capability
   (`ip link add`) and read the failure.
6. Write a scratch Rust program that binds port 80 as root, then drops to `nobody` (`setgroups`,
   `setgid`, `setuid`, via `nix`/`libc`), proves with `id`/`/proc/self/status` that it can no longer
   regain root (`setuid(0)` returns an error), and still accepts connections on the pre-opened
   listener.
