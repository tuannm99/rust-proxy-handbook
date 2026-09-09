# Buffer Pools

`14-memory/object-pool.md` covers pooling same-shaped objects generally.
I/O buffers specifically need their own treatment because their sizes
vary a lot (a 4 KB read chunk vs a 64 KB TLS record), which breaks a
pool designed around one fixed object shape.

## What to learn

### Size-classed pools, not one fixed size
A single pool of, say, 8 KB buffers wastes most of the buffer on a small
64-byte read and can't serve a 64 KB one at all. Buffer pools are
typically size-classed the same way a general allocator is
(`14-memory/allocator.md`): a handful of fixed tiers (e.g. 4 KB, 16 KB,
64 KB), and a checkout picks the smallest tier that fits the request,
rather than one pool trying to serve every size.

### Pooling at the type level `bytes`/`hyper` already use
`hyper` and the wider tokio ecosystem pass I/O data around as
`bytes::Bytes`/`BytesMut` — reference-counted, zero-copy-sliceable byte
buffers, not raw `Vec<u8>`. Pooling at the `Vec<u8>` level and then
copying into a `BytesMut` to hand to hyper defeats much of the point;
pool `BytesMut` allocations directly (or use a crate like `bytes-pool` /
roll a size-classed pool over `BytesMut::with_capacity`) so a checked-out
buffer composes with the rest of the stack without an extra copy.

### Where pooling doesn't apply at all: true zero-copy paths
`02-linux/zerocopy.md`'s `sendfile`/`splice` move data from the page
cache directly to a socket without it ever entering a userspace buffer —
there's nothing to pool on that path, because userspace never holds the
bytes. Buffer pooling matters for the paths that *do* copy through
userspace: TLS record encryption/decryption (the kernel can't decrypt
for you), WAF body inspection (`07-security/waf.md`, which must read the
bytes to inspect them), and any transformation that touches the payload.
Know which of your proxy's paths are which before assuming a buffer pool
helps a given one.

### Gotcha: tier granularity is a real tuning knob, not a detail
Too few tiers wastes memory (a 5 KB request rounds up to a 64 KB buffer
if that's the next tier); too many tiers fragments the pool itself into
many small, rarely-reused free lists, losing the benefit of pooling at
all. Size tiers from your actual traffic's payload size distribution
(measure it — `08-observability/metrics.md` — rather than guessing), the
same way a general allocator's size classes are chosen from real
allocation-size histograms.

## Practice
1. Build a size-classed buffer pool (e.g. 4 KB / 16 KB / 64 KB tiers)
   returning `BytesMut` for `labs/05-reverse-proxy`'s request/response
   copy path.
2. Measure the payload size distribution of a representative traffic mix
   and confirm (or adjust) your chosen tiers against it.
3. Compare allocator pressure (allocation count, RSS growth per
   `14-memory/fragmentation.md`'s method) with and without the pool under
   sustained concurrent load.
4. Identify which of `proxy`'s I/O paths are true zero-copy
   (`02-linux/zerocopy.md`) and confirm pooling has no effect on those,
   versus which paths copy through userspace and do benefit.
