# Memory

Allocator- and layout-level memory management, one level deeper than
`02-linux/memory.md`'s OS-level treatment (virtual memory, paging). This is
about what your process does with the memory it's been given.

## Planned topics

- `allocator.md` — how a general-purpose allocator (e.g. what's behind Rust's global allocator) actually works
- `arena.md` — bump allocation for request-scoped data, freed in bulk
- `slab-allocator.md` — fixed-size-class allocation to avoid fragmentation, complements `13-algorithms/slab.md`
- `object-pool.md` — reusing heap-allocated objects (buffers, connection structs) instead of alloc/free per request
- `buffer-pool.md` — pooling byte buffers specifically for I/O, ties into `02-linux/zerocopy.md`
- `fragmentation.md` — why long-running processes degrade over time and how pooling/arenas avoid it
