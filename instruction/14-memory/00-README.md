# Memory

Allocator- and layout-level memory management, one level deeper than
`02-linux/08-memory.md`'s OS-level treatment (virtual memory, paging). This is
about what your process does with the memory it's been given.

## Status: written; a reasonable order is allocator → arena → object-pool → buffer-pool → slab-allocator → fragmentation

`01-allocator.md` first (the general case), then the two request-scoped
strategies that trade off against it (`02-arena.md`, `03-object-pool.md`,
`04-buffer-pool.md` — the last builds on `03-object-pool.md`'s pattern),
then `05-slab-allocator.md` (which complements `13-algorithms/slab.md`),
and `06-fragmentation.md` last since it explains the failure mode all of
the above exist to avoid.

## Written

- `06-fragmentation.md` — why long-running processes degrade over time, and how pooling/arenas avoid it
- `05-slab-allocator.md` — fixed-size-class allocation, per-CPU caches, typed pools; complements `13-algorithms/slab.md`
- `01-allocator.md` — size classes, thread-local arenas, `#[global_allocator]`, why glibc's default usually isn't the right choice
- `02-arena.md` — bump allocation for request-scoped data, freed in bulk
- `03-object-pool.md` — reusing heap-allocated objects (buffers, connection structs) instead of alloc/free per request
- `04-buffer-pool.md` — pooling byte buffers specifically for I/O, size-classed tiers, ties into `02-linux/10-zerocopy.md`
